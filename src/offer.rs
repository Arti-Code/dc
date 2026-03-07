use std::sync::Arc;
use std::time::Duration;
use signaler::command::DescriptionType;
use signaler::{client::Client as SignalClient, command::generate_description};
use bytes::BytesMut;
use futures::FutureExt;
use webrtc::{
    data_channel::DataChannelEvent, 
    peer_connection::{
        MediaEngine, 
        RTCConfigurationBuilder, 
        RTCIceServer, 
        Registry, 
        register_default_interceptors
    }
};
use webrtc::peer_connection::{PeerConnection, PeerConnectionBuilder};
use webrtc::runtime::{channel, default_runtime, sleep};
use crate::util::get_local_ip;
use crate::event_handler::*;
use colored::*;


pub async fn process_offerer(name: &str, target: &str) -> anyhow::Result<()> {
    let mut media = MediaEngine::default();
    media.register_default_codecs()?;
    let (ctrlc_tx, mut ctrlc_rx) = channel::<()>(1);
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.try_send(());
    })?;
    let (gather_tx, mut gather_rx) = channel::<()>(1);
    let (done_tx, mut done_rx) = channel::<()>(1);
    let runtime = default_runtime()
    .ok_or_else(|| anyhow::anyhow!("no async runtime found"))?;
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
    .with_media_engine(media).with_interceptor_registry(registry)
    .with_handler(Arc::new(OfferHandler {
        gather_complete_tx: gather_tx,
        done_tx: done_tx.clone(),
    })).with_runtime(runtime.clone())
    .with_udp_addrs(vec![format!("{}:0", get_local_ip())])
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
                        println!("{}", "datachannel open".to_string().green().bold());
                        opened = true;
                        send_timer = Box::pin(sleep(Duration::from_secs(5)));
                    }
                    Some(DataChannelEvent::OnClose) | None => break,
                    _ => {}
                }
            }
        }
    
        println!("{}", "exit datachannel loop".to_string().yellow().bold());
    }));

    let url = "ws://yamanote.proxy.rlwy.net:25134";
    let mut signal_client = SignalClient::new(&name, url);
    signal_client.connect().await?;
    let offer = pc.create_offer(None).await?;
    pc.set_local_description(offer).await?;
    gather_rx.recv().await;
    let offer_sdp = pc.local_description().await
    .ok_or_else(|| anyhow::anyhow!("no local description"))?;
    let sdp = serde_json::to_string(&offer_sdp)?;
    signal_client.send_data(&target, sdp, DescriptionType::Offer).await?;
    println!("sent offer to {}, waiting for answer...", target);
    let sd = signal_client.wait_data().await?;
    println!("answer received from {}", sd.sender);
    let answer_str = sd.description;
    let answer_sdp = serde_json::from_str(&answer_str)?;
    pc.set_remote_description(answer_sdp).await?;
    println!("set remote description");
    println!("press ctrl-c to stop");
    futures::select! {
        _ = done_rx.recv().fuse() => {
            println!("{}", "data channel closed".to_string().red().bold());
        }
        _ = ctrlc_rx.recv().fuse() => {
            println!();
        }
    }
    pc.close().await?;
    Ok(())
}