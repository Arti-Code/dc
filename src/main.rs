
use std::net::Ipv4Addr;
use std::{net::IpAddr, sync::Arc};
use std::time::Duration;
use dialoguer::theme::ColorfulTheme;
use signaler::command::DescriptionType;
use signaler::{client::Client as SignalClient, command::generate_description};
use anyhow::Result;
use bytes::BytesMut;
use clap::Parser;
use futures::FutureExt;
//use signal::get_local_ip;
use webrtc::{data_channel::{DataChannel, DataChannelEvent}, peer_connection::RTCSessionDescription};
use webrtc::peer_connection::{
    MediaEngine, RTCConfigurationBuilder, RTCIceGatheringState, RTCIceServer,
    RTCPeerConnectionState, Registry, register_default_interceptors,
};
use webrtc::peer_connection::{PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler};
use webrtc::runtime::{Runtime, Sender, block_on, channel, default_runtime, sleep};
use dialoguer::*;

#[derive(Parser)]
#[command(name = "answer", about = "WebRTC answer side")]
struct Cli {
    #[arg(long, default_value = "0.0.0.0:60000")]
    answer_address: String,
    #[arg(long, default_value = "localhost:50000")]
    offer_address: String,
    #[arg(short, long)]
    debug: bool,
}


#[derive(Clone)]
struct OfferHandler {
    gather_complete_tx: Sender<()>,
    done_tx: Sender<()>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for OfferHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        if state == RTCIceGatheringState::Complete {
            let _ = self.gather_complete_tx.try_send(());
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        println!("Peer connection state: {state}");
        if state == RTCPeerConnectionState::Failed {
            let _ = self.done_tx.try_send(());
        }
    }
}

#[derive(Clone)]
struct AnswerHandler {
    runtime: Arc<dyn Runtime>,
    gather_complete_tx: Sender<()>,
    done_tx: Sender<()>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for AnswerHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        if state == RTCIceGatheringState::Complete {
            let _ = self.gather_complete_tx.try_send(());
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        println!("Peer connection state: {state}");
        if state == RTCPeerConnectionState::Failed {
            let _ = self.done_tx.try_send(());
        }
    }

    async fn on_data_channel(&self, dc: Arc<dyn DataChannel>) {
        let done_tx = self.done_tx.clone();
        self.runtime.spawn(Box::pin(async move {
            let mut opened = false;
            let mut send_timer = Box::pin(sleep(Duration::from_secs(5)));
            loop {
                if opened {
                    futures::select! {
                        event = dc.poll().fuse() => {
                            match event {
                                Some(DataChannelEvent::OnMessage(msg)) => {
                                    let text = String::from_utf8(msg.data.to_vec())
                                        .unwrap_or_default();
                                    println!("==> '{text}'");
                                }
                                Some(DataChannelEvent::OnClose) | None => {
                                    let _ = done_tx.try_send(());
                                    break;
                                }
                                _ => {}
                            }
                        }
                        _ = send_timer.as_mut().fuse() => {
                            let message = generate_description(32);
                            println!("<== '{message}'");
                            let _ = dc.send(BytesMut::from(message.as_bytes())).await;
                            send_timer = Box::pin(sleep(Duration::from_secs(5)));
                        }
                    }
                } else {
                    match dc.poll().await {
                        Some(DataChannelEvent::OnOpen) => {
                            println!("datachannel open");
                            opened = true;
                            send_timer = Box::pin(sleep(Duration::from_secs(5)));
                        }
                        Some(DataChannelEvent::OnClose) => {
                            let _ = done_tx.try_send(());
                            println!("onCloseo");
                            break;
                        },
                        None => {
                            let _ = done_tx.try_send(());
                            println!("None event");
                            break;
                        }
                        _ => {}
                    }
                }
            }

            println!("exit datachannel loop");
        }));
    }
}

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
    let target = match sdp_mode {
        0 => read_input("enter target:")?,
        1 => "".to_owned(),
        _ => unreachable!(),
    };
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
            let pc = PeerConnectionBuilder::new()
            .with_configuration(
                RTCConfigurationBuilder::new()
                    .with_ice_servers(vec![RTCIceServer {
                        urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                        ..Default::default()
                    }])
                    .build(),
            )
            .with_media_engine(media).with_interceptor_registry(registry).with_handler(Arc::new(OfferHandler {
                gather_complete_tx: gather_tx,
                done_tx: done_tx.clone(),
            })).with_runtime(runtime.clone()).with_udp_addrs(vec![format!("{}:0", get_local_ip())])
            .build().await?;
            let dc = pc.create_data_channel("data", None).await?;
            runtime.spawn(Box::pin(async move {
                let mut opened = false;
                let mut send_timer = Box::pin(sleep(Duration::from_secs(5)));         
                loop {
                    if opened {
                        futures::select! {
                            event = dc.poll().fuse() => {
                                match event {
                                    Some(DataChannelEvent::OnMessage(msg)) => {
                                        let text = String::from_utf8(msg.data.to_vec()).unwrap_or_default();
                                        println!("==> '{text}'");
                                    }
                                    Some(DataChannelEvent::OnClose) | None => break,
                                    _ => {}
                                }
                            }
                            _ = send_timer.as_mut().fuse() => {
                                let message = generate_description(32);
                                println!("<== '{message}'");
                                let _ = dc.send(BytesMut::from(message.as_bytes())).await;
                                send_timer = Box::pin(sleep(Duration::from_secs(5)));
                            }
                        }
                    } else {
                        match dc.poll().await {
                            Some(DataChannelEvent::OnOpen) => {
                                println!("datachannel open");
                                opened = true;
                                send_timer = Box::pin(sleep(Duration::from_secs(5)));
                            }
                            Some(DataChannelEvent::OnClose) | None => break,
                            _ => {}
                        }
                    }
                }
            
                println!("exit datachannel loop");
            }));
            let offer = pc.create_offer(None).await?;
            pc.set_local_description(offer).await?;
            gather_rx.recv().await;
            let offer_sdp = pc.local_description().await
            .ok_or_else(|| anyhow::anyhow!("no local description"))?;
            let payload = serde_json::to_string(&offer_sdp)?;
            println!("{}", payload);
            signal_client.send_data(&target, payload, DescriptionType::Offer).await?;
            println!("sent offer to {}, waiting for answer...", target);
            let sd = signal_client.wait_data().await?;
            println!("answer received from {}", sd.sender);
            let answer_str = sd.description;
            let answer_sdp = serde_json::from_str(&answer_str)?;
            pc.set_remote_description(answer_sdp).await?;
            println!("set remote description");
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

fn read_input(label: &str) -> Result<String> {
    println!("{label}: ");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    line = line.trim().to_owned();
    println!();

    Ok(line)
}