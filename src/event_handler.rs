use std::{sync::Arc, time::Duration};

use bytes::BytesMut;
use futures::FutureExt;
use signaler::command::generate_description;
use webrtc::{
    data_channel::{DataChannel, DataChannelEvent}, 
    peer_connection::{PeerConnectionEventHandler, RTCIceGatheringState, RTCPeerConnectionState}, 
    runtime::{Runtime, Sender, sleep}
};
use colored::*;


#[derive(Clone)]
pub struct CameraHandler {
    pub runtime: Arc<dyn Runtime>,
    pub gather_complete_tx: Sender<()>,
    pub connected_tx: Sender<()>,
    pub done_tx: Sender<()>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for CameraHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        match state {
            RTCIceGatheringState::Complete => {
                println!("{}{}", "[ICE GATHERING STATE]: ".to_string().bold().green(), state.to_string().bold().green());
                let _ = self.gather_complete_tx.try_send(());
            },
            _ => {
                println!("{}{}", "[ICE GATHERING STATE]: ".to_string(), state.to_string());
            }
        }
    }

    /* async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        println!("Peer Connection State has changed: {state}");
        match state {
            RTCPeerConnectionState::Connected => {
                let _ = self.connected_tx.try_send(());
            }
            RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed => {
                let _ = self.done_tx.try_send(());
            }
            _ => {}
        }
    } */

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        let mut state_info = String::new();
        if state == RTCPeerConnectionState::Failed {
            state_info = format!("{}{}", "[PEER CONNECTION]: ".to_string().bold().red(), state.to_string().bold().red());
            let _ = self.done_tx.try_send(());
        } else if state == RTCPeerConnectionState::Disconnected {
            state_info = format!("{}{}", "[PEER CONNECTION]: ".to_string().bold().yellow(), state.to_string().bold().yellow());
            let _ = self.done_tx.try_send(());
        } else if state == RTCPeerConnectionState::Connected {
            state_info = format!("{}{}", "[PEER CONNECTION]: ".to_string().bold().green(), state.to_string().bold().green());
            let _ = self.connected_tx.try_send(());
        } else if state == RTCPeerConnectionState::Closed {
            state_info = format!("{}{}", "[PEER CONNECTION]: ".to_string().bold().yellow(), state.to_string().bold().yellow());
            let _ = self.done_tx.try_send(());
        }
        println!("{}", &state_info);
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
                            println!("{}", "datachannel open".to_string().green().bold());
                            opened = true;
                            send_timer = Box::pin(sleep(Duration::from_secs(5)));
                        }
                        Some(DataChannelEvent::OnClose) => {
                            let _ = done_tx.try_send(());
                            //println!("datachannel close");
                            println!("{}", "datachannel closed".to_string().red().bold());
                            break;
                        },
                        Some(DataChannelEvent::OnClosing) => {
                            //let _ = done_tx.try_send(());
                            println!("{}", "datachannel closing".to_string().yellow().bold());
                            break;
                        },
                        Some(DataChannelEvent::OnError) => {
                            let _ = done_tx.try_send(());
                            println!("{}", "datachannel error".to_string().red().bold());
                            break;
                        },
                        None => {
                            //let _ = done_tx.try_send(());
                            println!("none event");
                            break;
                        }
                        _ => {
                            println!("other datachannel event");
                        }
                    }
                }
            }

            //println!("exit datachannel loop");
        }));
    }
}

#[derive(Clone)]
pub struct OfferHandler {
    pub gather_complete_tx: Sender<()>,
    pub done_tx: Sender<()>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for OfferHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        println!("ice gathering state: {state}");
        if state == RTCIceGatheringState::Complete {
            let _ = self.gather_complete_tx.try_send(());
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        //println!("peer connection state: {state}");
        if state == RTCPeerConnectionState::Failed {
            println!("{}", "peer connection failed".to_string().red().bold());
            let _ = self.done_tx.try_send(());
        } else if state == RTCPeerConnectionState::Disconnected {
            println!("{}", "peer connection disconnected".to_string().bright_red().bold());
            let _ = self.done_tx.try_send(());
        } else if state == RTCPeerConnectionState::Connected {
            println!("{}", "peer connection connected".to_string().green().bold());
        } else if state == RTCPeerConnectionState::Connecting {
            println!("{}", "peer connection connecting...".to_string().bright_green().bold());
        } else if state == RTCPeerConnectionState::New {
            println!("{}", "peer connection new".to_string().cyan().bold());
        } else if state == RTCPeerConnectionState::Closed {
            println!("{}", "peer connection closed".to_string().red().bold());
            let _ = self.done_tx.try_send(());
        } else {
            println!("peer connection: {state}");
        }
    }
}

#[derive(Clone)]
pub struct AnswerHandler {
    pub runtime: Arc<dyn Runtime>,
    pub gather_complete_tx: Sender<()>,
    pub done_tx: Sender<()>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for AnswerHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        println!("ice gathering state: {state}");
        if state == RTCIceGatheringState::Complete {
            let _ = self.gather_complete_tx.try_send(());
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        //println!("Peer connection state: {state}");
        if state == RTCPeerConnectionState::Failed {
            println!("{}", "peer connection failed".to_string().red().bold());
            let _ = self.done_tx.try_send(());
        } else if state == RTCPeerConnectionState::Disconnected {
            println!("{}", "peer connection disconnected".to_string().bright_red().bold());
            let _ = self.done_tx.try_send(());
        } else if state == RTCPeerConnectionState::Connected {
            println!("{}", "peer connection connected".to_string().green().bold());
        } else if state == RTCPeerConnectionState::Connecting {
            println!("{}", "peer connection connecting...".to_string().bright_green().bold());
        } else if state == RTCPeerConnectionState::New {
            println!("{}", "peer connection new".to_string().cyan().bold());
        } else if state == RTCPeerConnectionState::Closed {
            println!("{}", "peer connection closed".to_string().red().bold());
            let _ = self.done_tx.try_send(());
        } else {
            println!("peer connection: {state}");
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
                            println!("{}", "datachannel open".to_string().green().bold());
                            opened = true;
                            send_timer = Box::pin(sleep(Duration::from_secs(5)));
                        }
                        Some(DataChannelEvent::OnClose) => {
                            let _ = done_tx.try_send(());
                            //println!("datachannel close");
                            println!("{}", "datachannel closed".to_string().red().bold());
                            break;
                        },
                        Some(DataChannelEvent::OnClosing) => {
                            let _ = done_tx.try_send(());
                            println!("{}", "datachannel closing".to_string().yellow().bold());
                            break;
                        },
                        Some(DataChannelEvent::OnError) => {
                            let _ = done_tx.try_send(());
                            println!("{}", "datachannel error".to_string().red().bold());
                            break;
                        },
                        None => {
                            let _ = done_tx.try_send(());
                            println!("none event");
                            break;
                        }
                        _ => {
                            println!("other datachannel event");
                        }
                    }
                }
            }

            println!("exit datachannel loop");
        }));
    }
}
