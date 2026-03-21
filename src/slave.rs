use dialoguer::theme::ColorfulTheme;
use anyhow::Result;
use webrtc::runtime::block_on;
use dialoguer::*;
use colored::*;
use std::{sync::Arc, time::Duration};
use signaler::{client::Client as SignalClient, command::DescriptionType};
use futures::FutureExt;
use webrtc::{peer_connection::{
        MediaEngine, PeerConnection, PeerConnectionBuilder, RTCConfigurationBuilder, RTCIceServer, RTCSessionDescription, Registry, register_default_interceptors
    }, runtime::{channel, default_runtime}};
use dc::{event_handler::*, util::get_local_ip};
use tokio::sync::mpsc::{self, Receiver};

fn main() -> Result<()> {
    let (ctrlc_tx, mut ctrlc_rx) = mpsc::channel::<()>(1);
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.try_send(());
    })?;

    display_init();

    std::thread::sleep(Duration::from_millis(500));
    let name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter name")
    .default("ROBOT".to_string()).allow_empty(false).show_default(true)
    .interact_text().unwrap();
    loop {
         match block_on(async_main(name.clone(), &mut ctrlc_rx)) {
             Ok(false) => {
                println!("{}", "closing...".red().bold());
                std::thread::sleep(Duration::from_millis(3000));
                break
            },
             Ok(true) => {
                 println!("{}", "restarting...".yellow().bold());
                 std::thread::sleep(Duration::from_millis(500));
             }
             Err(e) => {
                 println!("error: {}", e);
                 println!("{}", "restarting...".bright_red().bold());
                 std::thread::sleep(Duration::from_millis(500));
             }
         }
    }
    Ok(())
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
            .with_ice_servers(
                vec![
                    RTCIceServer {
                        urls: vec!["stun:fr-turn8.xirsys.com".to_owned()],
                        ..Default::default()
                    },
                    RTCIceServer {
                        username: "xrlEivlkdTCQvwPYbCRHDur872L9CNM7DlbAya3tEhbBcn7zMgFFN8q43pP_2v-4AAAAAGmxwT1nd296ZHlr".to_owned(),
                        credential: "d05d03e4-1d7f-11f1-b1bb-be96737d4d7e".to_owned(),
                        urls: vec![
                            "turn:fr-turn8.xirsys.com:3478?transport=udp".to_owned(),
                            "turn:fr-turn8.xirsys.com:80?transport=tcp".to_owned(),
                            "turn:fr-turn8.xirsys.com:3478?transport=tcp".to_owned(), 
                            "turns:fr-turn8.xirsys.com:443?transport=tcp".to_owned(),
                            "turns:fr-turn8.xirsys.com:5349?transport=tcp".to_owned(),
                            "turn:fr-turn8.xirsys.com:80?transport=udp".to_owned(),
                        ]
                    },
                    RTCIceServer {
                        urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                        ..Default::default()
                    },
                ]
            )
            .build(),
        )
        .with_media_engine(media.clone())
        .with_interceptor_registry(registry)
        .with_handler(Arc::new(AnswerHandler {
            runtime: runtime.clone(),
            gather_complete_tx: gather_tx2.clone(),
            done_tx: done_tx.clone(),
        }))
        .with_runtime(runtime.clone())
        .with_udp_addrs(vec![format!("{}:0", get_local_ip())])
        .build().await?;

        let url = "ws://yamanote.proxy.rlwy.net:25134";
        let mut signal_client = SignalClient::new(&name, url);
        signal_client.connect().await?;
        println!("{}", "connection ready!".to_string().blue().bold());
        let sd =signal_client.wait_data().await?;
        println!("offer received from {}", sd.sender);
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
            }
        }
}

fn display_init() {
    let ver = env!("CARGO_PKG_VERSION").to_string();
    let authors = env!("CARGO_PKG_AUTHORS").to_string();
    let title = format!("-=WebRTC Slave=-");
    let date = "2026y".to_string();
    println!("");
    println!("{}", title.underline().bold().green());
    println!("");
    println!("{} {}", "version".to_string().yellow(), ver.yellow());
    println!("{} {}", authors.italic().cyan(), date.italic().cyan());
    println!("");
    println!("");
}