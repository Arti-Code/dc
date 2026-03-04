pub mod event_handler;
pub mod offer;
use std::net::Ipv4Addr;
use std::{net::IpAddr, sync::Arc};
use std::time::Duration;
use dialoguer::theme::ColorfulTheme;
use signaler::command::DescriptionType;
use signaler::{client::Client as SignalClient, command::generate_description};
use anyhow::Result;
use bytes::BytesMut;
//use clap::Parser;
use futures::FutureExt;
//use signal::get_local_ip;
use webrtc::{
    data_channel::DataChannelEvent, 
    peer_connection::{
        MediaEngine, 
        RTCConfigurationBuilder, 
        RTCIceServer, 
        RTCSessionDescription, 
        Registry, 
        register_default_interceptors
    }
};
//use webrtc::data_channel::DataChannel;
use webrtc::peer_connection::{PeerConnection, PeerConnectionBuilder};
use webrtc::runtime::{block_on, channel, default_runtime, sleep};
//use webrtc::runtime::Runtime;
use dialoguer::*;
use crate::event_handler::*;
use crate::offer::process_offerer;

/* #[derive(Parser)]
#[command(name = "answer", about = "WebRTC answer side")]
struct Cli {
    #[arg(long, default_value = "0.0.0.0:60000")]
    answer_address: String,
    #[arg(long, default_value = "localhost:50000")]
    offer_address: String,
    #[arg(short, long)]
    debug: bool,
} */



fn main() -> Result<()> {
    block_on(async_main())
}

async fn async_main() -> Result<()> {
    let sdp_modes = &[
        "OFFER",
        "ANSWER"
    ];
    let sdp_mode = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("select SDP mode").default(0).items(&sdp_modes[..]).interact().unwrap();
    let name = read_input("enter name")?;
    
    let (done_tx, mut done_rx) = channel::<()>(1);
    let (ctrlc_tx, mut ctrlc_rx) = channel::<()>(1);
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.try_send(());
    })?;
    let runtime = default_runtime().ok_or_else(|| anyhow::anyhow!("no async runtime found"))?;
    let (gather_tx, mut gather_rx) = channel::<()>(1);
    let mut media = MediaEngine::default();
    media.register_default_codecs()?;
    let registry = register_default_interceptors(Registry::new(), &mut media)?;
    //let url = "ws://localhost:8080";
    let url = "ws://yamanote.proxy.rlwy.net:25134";
    let mut signal_client = SignalClient::new(&name, url);
    signal_client.connect().await?;
    match sdp_mode {
        0 => {
            process_offerer(&name).await?;
        },
        // ANSWERER
        1 => {
            let pc = PeerConnectionBuilder::new()
            .with_configuration(
                RTCConfigurationBuilder::new()
                    .with_ice_servers(vec![RTCIceServer {
                        urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                        ..Default::default()
                    }])
                    .build(),
            )
            .with_media_engine(media).with_interceptor_registry(registry)
            .with_handler(Arc::new(AnswerHandler {
                runtime: runtime.clone(),
                gather_complete_tx: gather_tx,
                done_tx: done_tx.clone(),
            }))
            .with_runtime(runtime.clone()).with_udp_addrs(vec![format!("{}:0", get_local_ip())])
            .build().await?;
            println!("waiting for offer...");
            let sd =signal_client.wait_data().await?;
            //print!("got offer: {}\n", sd.description);
            println!("offer received from {}", sd.sender);
            let offer_sdp = serde_json::from_str::<RTCSessionDescription>(&sd.description)?;
            pc.set_remote_description(offer_sdp).await?;
            println!("set remote sdp, creating answer...");
            let answer = pc.create_answer(None).await?;
            pc.set_local_description(answer).await?;
            gather_rx.recv().await;
            let answer_sdp = pc.local_description().await.ok_or_else(|| anyhow::anyhow!("no local description"))?;
            let payload = serde_json::to_string(&answer_sdp)?;
            signal_client.send_data(&sd.sender, payload, DescriptionType::Answer).await?;
            println!("sent answer to {}", sd.sender);
            futures::select! {
                _ = done_rx.recv().fuse() => {
                    println!("Peer connection failed or data channel closed.");
                }
                _ = ctrlc_rx.recv().fuse() => {
                    println!();
                }
            }
            pc.close().await?;
        },
        _ => unreachable!(),
    }
    println!("Press ctrl-c to stop");
    futures::select! {
        _ = done_rx.recv().fuse() => {
            println!("Peer connection failed or data channel closed.");
        }
        _ = ctrlc_rx.recv().fuse() => {
            println!();
        }
    }

    Ok(())
}


pub fn get_local_ip() -> IpAddr {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0")
        && socket.connect("8.8.8.8:80").is_ok()
        && let Ok(addr) = socket.local_addr()
        && let IpAddr::V4(ip) = addr.ip()
    {
        ip.into()
    } else {
        Ipv4Addr::new(127, 0, 0, 1).into()
    }
}

pub fn read_input(label: &str) -> Result<String> {
    println!("{label}: ");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    line = line.trim().to_owned();
    println!();

    Ok(line)
}