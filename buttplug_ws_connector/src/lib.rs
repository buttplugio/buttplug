// Buttplug Client Websocket Connector
//
// The big thing to understand here is that we'll only ever need one connection.
// Just one. No more, no less. So there's no real reason to futz with trying to
// get async clients going here other than to lose us a thread, which means we
// shouldn't really need to wait for any network library to update to futures
// 0.3. For now, we can:
//
// - Create a futures channel, retain the receiver in the main thread.
// - Create a ws channel, retain a sender in the main thread
// - Create a thread (for the ws), hand it a sender from the futures channel
// - In ws thread, spin up the connection, waiting on success response in
//   our main thread as a future.
// - Continue on our way with the two channels, happy to know we don't have to
//   wait for networking libraries to get on our futures 0.3 level.

#[macro_use]
extern crate log;

use async_trait::async_trait;
use buttplug::client::connector::{
    ButtplugClientConnector, ButtplugClientConnectorError, ButtplugRemoteClientConnectorHelper,
    ButtplugRemoteClientConnectorMessage, ButtplugRemoteClientConnectorSender,
};
use buttplug::client::internal::{ButtplugClientMessageStateShared, ButtplugClientMessageFuture};
use buttplug::core::messages::{self, ButtplugMessage, ButtplugMessageUnion};
use async_std::{sync::{channel, Sender, Receiver}, future::{select}, task};
use std::thread;
use ws::{CloseCode, Handler, Handshake, Message};

// TODO Should probably let users pass in their own addresses
const CONNECTION: &'static str = "ws://127.0.0.1:12345";

struct InternalClient {
    connector_waker: ButtplugClientMessageStateShared,
    buttplug_out: Sender<ButtplugRemoteClientConnectorMessage>,
}

impl Handler for InternalClient {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        info!("Opened websocket");
        // TODO Use another future type when it's not midnight and you're less
        // tired.
        self.connector_waker.lock().unwrap().set_reply_msg(&ButtplugMessageUnion::Ok(messages::Ok::new(1)));
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        info!("Got message: {}", msg);
        let out = self.buttplug_out.clone();
        task::spawn(async move {
            out.send(ButtplugRemoteClientConnectorMessage::Text(msg.to_string())).await;
        });
        ws::Result::Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        info!("Closed!");
    }

    fn on_error(&mut self, err: ws::Error) {
        info!("The server encountered an error: {:?}", err);
    }
}

pub struct ButtplugWebsocketClientConnector {
    helper: ButtplugRemoteClientConnectorHelper,
    ws_thread: Option<thread::JoinHandle<()>>,
    recv: Option<Receiver<ButtplugMessageUnion>>,
}

impl ButtplugWebsocketClientConnector {
    pub fn new() -> ButtplugWebsocketClientConnector {
        let (send, recv) = channel(256);
        ButtplugWebsocketClientConnector {
            helper: ButtplugRemoteClientConnectorHelper::new(send.clone()),
            ws_thread: None,
            recv: Some(recv),
        }
    }
}

pub struct ButtplugWebsocketWrappedSender {
    sender: ws::Sender,
}

unsafe impl Send for ButtplugWebsocketWrappedSender {}
unsafe impl Sync for ButtplugWebsocketWrappedSender {}

impl ButtplugWebsocketWrappedSender {
    pub fn new(send: ws::Sender) -> ButtplugWebsocketWrappedSender {
        ButtplugWebsocketWrappedSender { sender: send }
    }
}

impl ButtplugRemoteClientConnectorSender for ButtplugWebsocketWrappedSender {
    fn send(&self, msg: ButtplugMessageUnion) {
        let m = msg.as_protocol_json();
        info!("Sending message: {}", m);
        self.sender.send(m);
    }

    fn close(&self) {
        self.sender.close(CloseCode::Normal);
    }
}

#[async_trait]
impl ButtplugClientConnector for ButtplugWebsocketClientConnector {
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError> {
        let send = self.helper.get_remote_send().clone();
        let fut = ButtplugClientMessageFuture::default();
        let mut waker = fut.get_state_ref().clone();
        self.ws_thread = Some(thread::spawn(|| {
            ws::connect(CONNECTION, move |out| {
                let bp_out = send.clone();
                // Get our websocket sender back to the main thread
                task::spawn(async move {
                    bp_out.send(ButtplugRemoteClientConnectorMessage::Sender(Box::new(
                        ButtplugWebsocketWrappedSender::new(out.clone()),
                    ))).await;
                });
                // Go ahead and create our internal client
                InternalClient {
                    buttplug_out: send.clone(),
                    connector_waker: waker.clone(),
                }
            });
        }));

        let read_future = self.helper.get_recv_future();

        // TODO This should be part of the ButtplugClientInternalLoop
        task::spawn(async {
            read_future.await;
        });

        fut.await;
        None
    }

    fn disconnect(&mut self) -> Option<ButtplugClientConnectorError> {
        None
    }

    async fn send(
        &mut self,
        msg: &ButtplugMessageUnion,
        state: &ButtplugClientMessageStateShared,
    ) {
        self.helper.send(msg, state).await;
    }

    fn get_event_receiver(&mut self) ->
        Receiver<ButtplugMessageUnion> {
        // This will panic if we've already taken the receiver.
        self.recv.take().unwrap()
    }
}

#[cfg(test)]
mod test {
    use log::{info};
    use super::ButtplugWebsocketClientConnector;
    use async_std::task;
    use buttplug::client::connector::ButtplugClientConnector;
    use buttplug::client::{ButtplugClient, ButtplugClientEvent};
    use env_logger;

    // Only run these tests when we know there's an external server up to reply

    #[test]
    #[ignore]
    fn test_websocket() {
        task::block_on(async {
            assert!(ButtplugWebsocketClientConnector::new()
                    .connect()
                    .await
                    .is_none());
        })
    }

    #[test]
    #[ignore]
    fn test_client_websocket() {
        env_logger::init();
        task::block_on(async {
            info!("connecting");
            let mut client = ButtplugClient::new("test client");
            let lp = client.get_loop();
            let app = task::spawn(async move {
                client
                    .connect(ButtplugWebsocketClientConnector::new())
                    .await;
                info!("connected");
                client.start_scanning().await;
                info!("scanning!");
                info!("starting event loop!");
                loop {
                    info!("Waiting for event!");
                    for mut event in client.wait_for_event().await {
                        match event {
                            ButtplugClientEvent::DeviceAdded(ref mut _device) => {
                                info!("Got device! {}", _device.name);
                                let mut d = _device.clone();
                                if d.allowed_messages.contains_key("VibrateCmd") {
                                    d.send_vibrate_cmd(1.0).await;
                                    info!("Should be vibrating!");
                                }
                            }
                            _ => info!("Got something else!"),
                        }
                    }
                }
            });
            futures::join!(lp, app);
        })
    }
}
