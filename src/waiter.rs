use dialoguer::theme::ColorfulTheme;
use anyhow::Result;
use webrtc::runtime::block_on;
use dialoguer::*;
use colored::*;
use std::sync::Arc;
use signaler::command::DescriptionType;
use signaler::client::Client as SignalClient;
use futures::FutureExt;
use webrtc::peer_connection::{
        MediaEngine, 
        RTCConfigurationBuilder, 
        RTCIceServer, 
        RTCSessionDescription, 
        Registry, 
        register_default_interceptors
    };
use webrtc::peer_connection::{PeerConnection, PeerConnectionBuilder};
use webrtc::runtime::{channel, default_runtime};
use dc::util::get_local_ip;
use dc::event_handler::*;
use tokio::sync::mpsc::{self, Receiver};
//use webrtc::runtime::tokio:: 
//use webrtc::runtime::channel::{Sender as TokioSender, Receiver as TokioReceiver};
fn main() -> Result<()> {
    let (ctrlc_tx, mut ctrlc_rx) = mpsc::channel::<()>(1);
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.try_send(());
    })?;
    display_init();
    std::thread::sleep(std::time::Duration::from_millis(500));
    let name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter name")
    .default("ROBOT".to_string()).allow_empty(false).show_default(true)
    .interact_text().unwrap();
    loop {
         match block_on(async_main(name.clone(), &mut ctrlc_rx)) {
             Ok(false) => {
                println!("{}", "closing...".red().bold());
                std::thread::sleep(std::time::Duration::from_millis(3000));
                break
            },
             Ok(true) => {
                 println!("{}", "restarting...".yellow().bold());
                 std::thread::sleep(std::time::Duration::from_millis(500));
             }
             Err(e) => {
                 println!("error: {}", e);
                 println!("{}", "restarting...".bright_red().bold());
                 std::thread::sleep(std::time::Duration::from_millis(500));
             }
         }
    }
    Ok(())
    //block_on(async_main())
}

async fn async_main(name: String, ctrlc_rx: &mut Receiver<()>) -> Result<bool> {
        let mut media = MediaEngine::default();
        media.register_default_codecs()?;
        let (gather_tx, mut gather_rx) = channel::<()>(1);
        let (done_tx, mut done_rx) = channel::<()>(1);
        let runtime = default_runtime()
        .ok_or_else(|| anyhow::anyhow!("no async runtime found"))?;
        let gather_tx2 = gather_tx.clone();
        let registry = register_default_interceptors(Registry::new(), &mut media)?;
        let pc = PeerConnectionBuilder::new()
        .with_configuration(
            RTCConfigurationBuilder::new()
            .with_ice_servers(vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }])
            .build(),
        )
        .with_media_engine(media.clone()).with_interceptor_registry(registry)
        .with_handler(Arc::new(AnswerHandler {
            runtime: runtime.clone(),
            gather_complete_tx: gather_tx2.clone(),
            done_tx: done_tx.clone(),
        }))
        .with_runtime(runtime.clone()).with_udp_addrs(vec![format!("{}:0", get_local_ip())])
        .build().await?;

        let url = "ws://yamanote.proxy.rlwy.net:25134";
        let mut signal_client = SignalClient::new(&name, url);
        signal_client.connect().await?;
        println!("{}", "ready!".to_string().blue().bold());
        let sd =signal_client.wait_data().await?;
        println!("offer received from {}", sd.sender);
        //dbg!(&sd);
        let offer_sdp = serde_json::from_str::<RTCSessionDescription>(&sd.description)?;
        println!("offer sdp parsed, setting remote description...");
        pc.set_remote_description(offer_sdp).await?;
        println!("set remote sdp, creating answer...");
        let answer = pc.create_answer(None).await?;
        pc.set_local_description(answer).await?;
        gather_rx.recv().await;
        let answer_sdp = pc.local_description().await
        .ok_or_else(|| anyhow::anyhow!("no local description"))?;
        let payload = serde_json::to_string(&answer_sdp)?;
        signal_client.send_data(&sd.sender, payload, DescriptionType::Answer).await?;
        println!("sent answer to {}", sd.sender);
        println!("press ctrl-c to stop");
        futures::select! {
            _ = done_rx.recv().fuse() => {
                println!("{}", "data channel closed".to_string().red().bold());
                pc.close().await?;
                return Ok(true);
            }
            _ = ctrlc_rx.recv().fuse() => {
                println!();
                pc.close().await?;
                return Ok(false)
                //break;
            }
        }
        //pc.close().await?;
        //println!("{}", "restarting...".yellow());
        //tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
    //Ok(true)
}

fn display_init() {
    let ver = env!("CARGO_PKG_VERSION").to_string();
    let authors = env!("CARGO_PKG_AUTHORS").to_string();
    let title = format!("-=WebRTC Waiter=-");
    let date = "2026y".to_string();
    println!("");
    println!("{}", title.underline().bold().green());
    println!("");
    println!("{} {}", "version".to_string().yellow(), ver.yellow());
    println!("{} {}", authors.italic().cyan(), date.italic().cyan());
    println!("");
    println!("");
}