use std::{sync::Arc, time::Duration};

use bytes::BytesMut;
use futures::FutureExt;
use signaler::command::generate_description;
use webrtc::{
    data_channel::{DataChannel, DataChannelEvent}, 
    peer_connection::{PeerConnectionEventHandler, RTCIceGatheringState, RTCPeerConnectionState}, 
    runtime::{Runtime, Sender, sleep}
};





#[derive(Clone)]
pub struct OfferHandler {
    pub gather_complete_tx: Sender<()>,
    pub done_tx: Sender<()>,
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
pub struct AnswerHandler {
    pub runtime: Arc<dyn Runtime>,
    pub gather_complete_tx: Sender<()>,
    pub done_tx: Sender<()>,
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
