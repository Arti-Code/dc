use std::sync::Arc;
use signaler::command::DescriptionType;
use signaler::client::Client as SignalClient;
//use clap::Parser;
use futures::FutureExt;
//use signal::get_local_ip;
use webrtc::peer_connection::{
        MediaEngine, 
        RTCConfigurationBuilder, 
        RTCIceServer, 
        RTCSessionDescription, 
        Registry, 
        register_default_interceptors
    };
//use webrtc::data_channel::DataChannel;
use webrtc::peer_connection::{PeerConnection, PeerConnectionBuilder};
use webrtc::runtime::{channel, default_runtime};
//use webrtc::runtime::Runtime;
use crate::{event_handler::*, get_local_ip};

pub async fn process_answerer(name: &str) -> anyhow::Result<()> {
    let mut media = MediaEngine::default();
    media.register_default_codecs()?;
    let (ctrlc_tx, mut ctrlc_rx) = channel::<()>(1);
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.try_send(());
    })?;
    let (gather_tx, mut gather_rx) = channel::<()>(1);
    let (done_tx, mut done_rx) = channel::<()>(1);
    let runtime = default_runtime().ok_or_else(|| anyhow::anyhow!("no async runtime found"))?;
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
    .with_handler(Arc::new(AnswerHandler {
        runtime: runtime.clone(),
        gather_complete_tx: gather_tx,
        done_tx: done_tx.clone(),
    }))
    .with_runtime(runtime.clone()).with_udp_addrs(vec![format!("{}:0", get_local_ip())])
    .build().await?;

    let url = "ws://yamanote.proxy.rlwy.net:25134";
    let mut signal_client = SignalClient::new(&name, url);
    signal_client.connect().await?;
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
    println!("Press ctrl-c to stop");
    futures::select! {
        _ = done_rx.recv().fuse() => {
            println!("Peer connection failed or data channel closed.");
        }
        _ = ctrlc_rx.recv().fuse() => {
            println!();
        }
    }
    pc.close().await?;
    Ok(())
}