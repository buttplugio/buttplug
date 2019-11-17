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

// Required to get tests compiling?!
#![type_length_limit = "2000000"]

#[macro_use]
extern crate log;

use async_std::{
    sync::{channel, Receiver, Sender},
    task,
};
use async_trait::async_trait;
use buttplug::client::connector::{
    ButtplugClientConnectionFuture, ButtplugClientConnectionStateShared, ButtplugClientConnector,
    ButtplugClientConnectorError, ButtplugRemoteClientConnectorHelper,
    ButtplugRemoteClientConnectorMessage, ButtplugRemoteClientConnectorSender,
};
use buttplug::client::internal::ButtplugClientMessageStateShared;
use buttplug::core::messages::{ButtplugMessage, ButtplugMessageUnion};
use openssl::ssl::{SslConnector, SslMethod, SslStream, SslVerifyMode};
use std::thread;
use url;
use ws::util::TcpStream;
use ws::{CloseCode, Handler, Handshake, Message};

// TODO Should probably let users pass in their own addresses
const CONNECTION: &str = "ws://localhost:12345";

struct InternalClient {
    connector_waker: ButtplugClientConnectionStateShared,
    buttplug_out: Sender<ButtplugRemoteClientConnectorMessage>,
}

impl Handler for InternalClient {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        info!("Opened websocket");
        self.connector_waker.lock().unwrap().set_reply_msg(&None);
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        info!("Got message: {}", msg);
        let out = self.buttplug_out.clone();
        task::spawn(async move {
            out.send(ButtplugRemoteClientConnectorMessage::Text(msg.to_string()))
                .await;
        });
        ws::Result::Ok(())
    }

    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        info!("Websocket closed : {}", _reason);
        let out = self.buttplug_out.clone();
        // One rather horrible way to get a copy of the reason to pass along.
        let r = (&(*_reason).to_owned()).clone();
        task::spawn(async move {
            out.send(ButtplugRemoteClientConnectorMessage::Close(r))
                .await;
        });
    }

    fn on_error(&mut self, err: ws::Error) {
        info!("The server encountered an error: {:?}", err);
        self.connector_waker.lock().unwrap().set_reply_msg(&Some(
            ButtplugClientConnectorError::new(&(format!("{}", err))),
        ));
    }

    fn upgrade_ssl_client(
        &mut self,
        sock: TcpStream,
        _: &url::Url,
    ) -> ws::Result<SslStream<TcpStream>> {
        let mut builder = SslConnector::builder(SslMethod::tls()).map_err(|e| {
            ws::Error::new(
                ws::ErrorKind::Internal,
                format!("Failed to upgrade client to SSL: {}", e),
            )
        })?;
        builder.set_verify(SslVerifyMode::empty());

        let connector = builder.build();
        connector
            .configure()
            .unwrap()
            .use_server_name_indication(false)
            .verify_hostname(false)
            .connect("", sock)
            .map_err(From::from)
    }
}

pub struct ButtplugWebsocketClientConnector {
    helper: ButtplugRemoteClientConnectorHelper,
    ws_thread: Option<thread::JoinHandle<()>>,
    recv: Option<Receiver<ButtplugMessageUnion>>,
}

impl Default for ButtplugWebsocketClientConnector {
    fn default() -> Self {
        let (send, recv) = channel(256);
        ButtplugWebsocketClientConnector {
            helper: ButtplugRemoteClientConnectorHelper::new(send),
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
    pub fn new(send: ws::Sender) -> Self {
        Self { sender: send }
    }
}

impl ButtplugRemoteClientConnectorSender for ButtplugWebsocketWrappedSender {
    fn send(&self, msg: ButtplugMessageUnion) {
        let m = msg.as_protocol_json();
        debug!("Sending message: {}", m);
        match self.sender.send(m) {
            Ok(_) => {}
            Err(err) => error!("{}", err),
        }
    }

    fn close(&self) {
        match self.sender.close(CloseCode::Normal) {
            Ok(_) => {}
            Err(err) => error!("{}", err),
        }
    }
}

#[async_trait]
impl ButtplugClientConnector for ButtplugWebsocketClientConnector {
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError> {
        let send = self.helper.get_remote_send();
        let fut = ButtplugClientConnectionFuture::default();
        let waker = fut.get_state_clone();
        self.ws_thread = Some(thread::spawn(|| {
            let ret = ws::connect(CONNECTION, move |out| {
                let bp_out = send.clone();
                // Get our websocket sender back to the main thread
                task::spawn(async move {
                    bp_out
                        .send(ButtplugRemoteClientConnectorMessage::Sender(Box::new(
                            ButtplugWebsocketWrappedSender::new(out.clone()),
                        )))
                        .await;
                });
                // Go ahead and create our internal client
                InternalClient {
                    buttplug_out: send.clone(),
                    connector_waker: waker.clone(),
                }
            });
            match ret {
                Ok(_) => {}
                Err(err) => error!("{}", err),
            }
        }));

        let read_future = self.helper.get_recv_future();

        // TODO This should be part of the ButtplugClientInternalLoop
        task::spawn(async {
            read_future.await;
        });

        fut.await
    }

    async fn disconnect(&mut self) -> Option<ButtplugClientConnectorError> {
        self.helper.close().await;
        None
    }

    async fn send(&mut self, msg: &ButtplugMessageUnion, state: &ButtplugClientMessageStateShared) {
        self.helper.send(msg, state).await;
    }

    fn get_event_receiver(&mut self) -> Receiver<ButtplugMessageUnion> {
        // This will panic if we've already taken the receiver.
        self.recv.take().unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::ButtplugWebsocketClientConnector;
    use async_std::task;
    use buttplug::client::connector::ButtplugClientConnector;
    use buttplug::client::{ButtplugClient, ButtplugClientEvent};
    use env_logger;
    use futures_timer::Delay;
    use log::info;
    use std::time::Duration;

    // Only run these tests when we know there's an external server up to reply

    #[test]
    #[ignore]
    fn test_websocket() {
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async {
            assert!(ButtplugWebsocketClientConnector::default()
                .connect()
                .await
                .is_none());
        })
    }

    #[test]
    #[ignore]
    fn test_client_websocket() {
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async {
            info!("connecting");
            ButtplugClient::run("test client", |mut client| {
                async move {
                    assert!(client
                        .connect(ButtplugWebsocketClientConnector::default())
                        .await
                        .is_ok());
                    info!("connected");
                    assert!(client.start_scanning().await.is_ok());
                    info!("scanning!");
                    info!("starting event loop!");
                    while client.connected() {
                        info!("Waiting for event!");
                        for mut event in client.wait_for_event().await {
                            match event {
                                ButtplugClientEvent::DeviceAdded(ref mut _device) => {
                                    info!("Got device! {}", _device.name);
                                    let mut d = _device.clone();
                                    if d.allowed_messages.contains_key("VibrateCmd") {
                                        assert!(d.send_vibrate_cmd(1.0).await.is_some());
                                        info!("Should be vibrating!");
                                        Delay::new(Duration::from_secs(1)).await;
                                        assert!(d.send_vibrate_cmd(0.0).await.is_some());
                                        assert!(client.disconnect().await.is_ok());
                                        Delay::new(Duration::from_secs(1)).await;
                                        break;
                                    }
                                }
                                ButtplugClientEvent::ServerDisconnect => {
                                    assert!(false, "Server disconnected!");
                                    break;
                                }
                                _ => info!("Got something else!"),
                            }
                        }
                    }
                }
            })
            .await;
        })
    }
}
