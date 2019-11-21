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

use super::{
    ButtplugClientConnectionFuture, ButtplugClientConnectionStateShared, ButtplugClientConnector,
    ButtplugClientConnectorError, ButtplugRemoteClientConnectorHelper,
    ButtplugRemoteClientConnectorMessage, ButtplugRemoteClientConnectorSender,
};
use crate::{
    client::internal::ButtplugClientMessageStateShared,
    core::messages::{ButtplugMessage, ButtplugMessageUnion},
};
use async_std::{
    sync::{channel, Receiver, Sender},
    task,
};
use async_trait::async_trait;
#[cfg(feature = "client-ws-ssl")]
use openssl::ssl::{SslConnector, SslMethod, SslStream, SslVerifyMode};
use std::thread;
use url::Url;
use ws::util::TcpStream;
use ws::{CloseCode, Handler, Handshake, Message};

struct InternalClient {
    connector_waker: ButtplugClientConnectionStateShared,
    buttplug_out: Sender<ButtplugRemoteClientConnectorMessage>,
    bypass_cert_verify: bool,
}

impl Handler for InternalClient {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        info!("Opened websocket");
        self.connector_waker.lock().unwrap().set_reply(Ok(()));
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
        self.connector_waker
            .lock()
            .unwrap()
            .set_reply(Err(ButtplugClientConnectorError::new(
                &(format!("{}", err)),
            )));
    }

    #[cfg(feature = "client-ws-ssl")]
    fn upgrade_ssl_client(&mut self, sock: TcpStream, _: &Url) -> ws::Result<SslStream<TcpStream>> {
        let mut builder = SslConnector::builder(SslMethod::tls()).map_err(|e| {
            ws::Error::new(
                ws::ErrorKind::Internal,
                format!("Failed to upgrade client to SSL: {}", e),
            )
        })?;

        if self.bypass_cert_verify {
            builder.set_verify(SslVerifyMode::empty());
        }

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
    address: String,
    bypass_cert_verify: bool,
}

impl ButtplugWebsocketClientConnector {
    pub fn new(address: &str, bypass_cert_verify: bool) -> Self {
        let (send, recv) = channel(256);
        ButtplugWebsocketClientConnector {
            helper: ButtplugRemoteClientConnectorHelper::new(send),
            ws_thread: None,
            recv: Some(recv),
            address: address.to_owned(),
            bypass_cert_verify,
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
    async fn connect(&mut self) -> Result<(), ButtplugClientConnectorError> {
        let send = self.helper.get_remote_send();
        let fut = ButtplugClientConnectionFuture::default();
        let waker = fut.get_state_clone();
        let addr = self.address.clone();
        let verify = self.bypass_cert_verify;
        self.ws_thread = Some(thread::spawn(move || {
            let ret = ws::connect(addr, move |out| {
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
                    bypass_cert_verify: verify,
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

    async fn disconnect(&mut self) -> Result<(), ButtplugClientConnectorError> {
        self.helper.close().await;
        Ok(())
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
    use crate::client::{connectors::ButtplugClientConnector, ButtplugClient, ButtplugClientEvent, device::VibrateCommand};
    use async_std::task;
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
            assert!(
                ButtplugWebsocketClientConnector::new("ws://localhost:12345", false)
                    .connect()
                    .await
                    .is_ok()
            );
        })
    }

    #[test]
    #[ignore]
    fn test_client_websocket() {
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async {
            info!("connecting");
            assert!(ButtplugClient::run(
                "test client",
                ButtplugWebsocketClientConnector::new("ws://localhost:12345", false),
                |mut client| {
                    async move {
                        info!("connected");
                        assert!(client.start_scanning().await.is_ok());
                        info!("scanning!");
                        info!("starting event loop!");
                        while client.connected() {
                            info!("Waiting for event!");
                            match client.wait_for_event().await.unwrap() {
                                ButtplugClientEvent::DeviceAdded(ref mut d) => {
                                    info!("Got device! {}", d.name);
                                    if d.allowed_messages.contains_key("VibrateCmd") {
                                        assert!(d.vibrate(VibrateCommand::Speed(1.0)).await.is_ok());
                                        info!("Should be vibrating!");
                                        Delay::new(Duration::from_secs(1)).await;
                                        assert!(d.stop().await.is_ok());
                                        // assert!(client.disconnect().await.is_ok());
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
                        info!("Trying to get device again!");
                        let mut d = client.devices().await.unwrap();
                        if d.len() > 0 && d[0].allowed_messages.contains_key("VibrateCmd") {
                            assert!(d[0].vibrate(VibrateCommand::Speed(1.0)).await.is_ok());
                            info!("Should be vibrating!");
                            Delay::new(Duration::from_secs(1)).await;
                            assert!(d[0].stop().await.is_ok());
                            assert!(client.disconnect().await.is_ok());
                            Delay::new(Duration::from_secs(1)).await;
                        }
                    }
                }
            )
            .await
            .is_ok());
        })
    }
}
