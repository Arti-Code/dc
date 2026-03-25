use bytes::BytesMut;
use dialoguer::theme::ColorfulTheme;
use anyhow::Result;
use rtc::{peer_connection::configuration::media_engine::MIME_TYPE_VP8, rtp, rtp_transceiver::rtp_sender::{
    RTCRtpCodec, 
    RTCRtpCodecParameters, 
    RTCRtpCodingParameters, 
    RTCRtpEncodingParameters, 
    RtpCodecKind
}, shared::marshal::Unmarshal};
use webrtc::{media_stream::{MediaStreamTrack, track_local::TrackLocal}, runtime::{AsyncUdpSocket, block_on}};
use dialoguer::*;
use colored::*;
use std::{sync::Arc, time::Duration};
use signaler::{client::Client as SignalClient, command::DescriptionType};
use futures::FutureExt;
use webrtc::{media_stream::track_local::static_rtp::TrackLocalStaticRTP, peer_connection::{
    MediaEngine, PeerConnection, PeerConnectionBuilder, RTCConfigurationBuilder, RTCIceServer, RTCSessionDescription, Registry, register_default_interceptors
}, runtime::{channel, default_runtime}};
use dc::{event_handler::*, util::get_local_ip};
use tokio::sync::mpsc::{self, Receiver};

const VIDEO_LISTENER: &'static str = "127.0.0.1:5008";

fn main() -> Result<()> {
    let (ctrlc_tx, mut ctrlc_rx) = mpsc::channel::<()>(1);
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.try_send(());
    })?;
    display_init();
    std::thread::sleep(Duration::from_millis(500));
    let name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("enter name")
    .default("CAMERA".to_string()).allow_empty(false).show_default(true)
    .interact_text().unwrap();
    loop {
         match block_on(async_main(name.clone(), &mut ctrlc_rx)) {
             Ok(false) => {
                println!("{}", "closing...".red().bold());
                std::thread::sleep(Duration::from_millis(3000));
                break
            },
             Ok(true) => {
                 println!("{}", "restarting...".yellow());
                 std::thread::sleep(Duration::from_millis(500));
             }
             Err(e) => {
                 println!("{}", e.to_string().bold().yellow());
                 println!("{}", "restarting...".yellow());
                 std::thread::sleep(Duration::from_millis(500));
             }
         }
    }
    Ok(())
}

async fn async_main(name: String, ctrlc_rx: &mut Receiver<()>) -> Result<bool> {
        let mut media_engine = MediaEngine::default();
        let video_codec = RTCRtpCodec {
            mime_type: MIME_TYPE_VP8.to_owned(),
            clock_rate: 90000,
            channels: 0,
            sdp_fmtp_line: "".to_owned(),
            rtcp_feedback: vec![],
        };
        media_engine.register_codec(
            RTCRtpCodecParameters {
                rtp_codec: video_codec.clone(),
                payload_type: 96,
                ..Default::default()
            },
            RtpCodecKind::Video,
        )?;
        let registry = register_default_interceptors(
            Registry::new(), 
            &mut media_engine
        )?;
        let config = RTCConfigurationBuilder::new()
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
        ).build();

        let (connected_tx, mut connected_rx) = channel::<()>(1);
        let (gather_tx, mut gather_rx) = channel::<()>(1);
        let (done_tx, mut done_rx) = channel::<()>(1);
        
        let runtime = default_runtime()
        .ok_or_else(|| anyhow::anyhow!("no async runtime found"))?;
        
        //let gather_tx2 = gather_tx.clone();
        
        let handler = Arc::new(CameraHandler {
            gather_complete_tx: gather_tx,
            done_tx,
            connected_tx,
            runtime: runtime.clone(),
        });

        let ssrc = rand::random::<u32>();
        let video_track: Arc<TrackLocalStaticRTP> =
        Arc::new(TrackLocalStaticRTP::new(MediaStreamTrack::new(
            format!("stream-id-{}", RtpCodecKind::Video),
            format!("track-id-{}", RtpCodecKind::Video),
            format!("track-label-{}", RtpCodecKind::Video),
            RtpCodecKind::Video,
            vec![RTCRtpEncodingParameters {
                rtp_coding_parameters: RTCRtpCodingParameters {
                    ssrc: Some(ssrc),
                    ..Default::default()
                },
                codec: video_codec,
                ..Default::default()
            }],
        )));

        let pc = PeerConnectionBuilder::new()
        .with_configuration(config)
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_handler(handler)
        .with_runtime(runtime.clone())
        .with_udp_addrs(vec![format!("{}:0", get_local_ip())])
        .build()
        .await?;

        pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal>).await?;

        let url = "ws://yamanote.proxy.rlwy.net:25134";
        let mut signal_client = SignalClient::new(&name, url);
        signal_client.connect().await?;
        println!("{}", "connection ready!".to_string().blue().bold());
        let sd0 =signal_client.wait_data().await?;
        println!("offer received from {}", sd0.sender);
        let offer_sdp = serde_json::from_str::<RTCSessionDescription>(&sd0.description)?;
        pc.set_remote_description(offer_sdp).await?;
        println!("creating answer...");
        let answer = pc.create_answer(None).await?;
        pc.set_local_description(answer).await?;
        let _ = gather_rx.recv().await;
        let sd1 = pc.local_description().await
        .ok_or_else(|| anyhow::anyhow!("no local description"))?;
        let answer = serde_json::to_string(&sd1)?;
        signal_client.send_data(&sd0.sender, answer, DescriptionType::Answer).await?;
        println!("answer sent to {}", sd0.sender);

        let std_listener = std::net::UdpSocket::bind(VIDEO_LISTENER)?;
        let listener: Arc<dyn AsyncUdpSocket> = runtime.wrap_udp_socket(std_listener)?;
        println!("listening for video {}", VIDEO_LISTENER);
        
        println!("waiting for connection...");
        connected_rx.recv().await;

        let (fwd_done_tx, mut fwd_done_rx) = channel::<()>(1);
        runtime.spawn(Box::pin(async move {
            let mut buf = vec![0u8; 1600];
            loop {
                match listener.recv_from(&mut buf).await {
                    Ok((n, _)) => {
                        //println!("{} bytes", n);
                        let mut bytes = BytesMut::from(&buf[..n]);
                        match rtp::packet::Packet::unmarshal(&mut bytes) {
                            Ok(mut packet) => {
                                packet.header.ssrc = ssrc;
                                if let Err(_) = video_track.write_rtp(packet).await {
                                    println!("write_rtp error");
                                    break;
                                }
                            }
                            Err(err) => eprintln!("RTP unmarshal error: {err}"),
                        }
                    }
                    Err(err) => {
                        eprintln!("UDP read error: {err}");
                        break;
                    }
                }
            }
            let _ = fwd_done_tx.try_send(());
        }));

        futures::select! {
            _ = done_rx.recv().fuse() => {
                println!("{}", "data channel closed".to_string().red().bold());
                pc.close().await?;
                return Ok(true);
            }
            _ = ctrlc_rx.recv().fuse() => {
                println!("{}", "CTRL+C from user".to_string().bold().yellow());
                pc.close().await?;
                return Ok(false)
                //break;
            },
            _ = fwd_done_rx.recv().fuse() => {
                println!("{}", "RTP forwarding ended".to_string().bold());
                pc.close().await?;
                return Ok(true);
            },
        }
}

fn display_init() {
    let ver = env!("CARGO_PKG_VERSION").to_string();
    let authors = env!("CARGO_PKG_AUTHORS").to_string();
    let title = format!("-= WebRTC Camera =-");
    let date = "2026y".to_string();
    println!("");
    println!("{}", title.underline().bold().green());
    println!("");
    println!("{} {}", "version".to_string().yellow(), ver.yellow());
    println!("{} {}", authors.italic().cyan(), date.italic().cyan());
    println!("");
    println!("");
}