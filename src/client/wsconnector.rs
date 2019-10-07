// Buttplug Client Websocket Connector
//
// The big thing to understand here is that we'll only ever need one connection.
// Just one. No more, no less. So there's no real reason to futz with trying to
// get async clients going here, which means we shouldn't really need to wait
// for any network library to update. We can:
//
// - Create a futures channel, retain the receiver in the main thread.
// - Create a ws channel, retain a sender in the main thread
// - Create a thread (for the ws), hand it a sender from the futures channel
// - In ws thread, spin up the connection, waiting on success response in
//   our main thread as a future.
// - Continue on our way with the two channels, happy to know we don't have to
//   wait for networking libraries to get on our futures 0.3 level.

use super::connector::{ButtplugClientConnector, ButtplugClientConnectorError};
use super::client::ButtplugClientError;
use crate::core::messages;
use std::thread;
use crate::core::messages::ButtplugMessageUnion;
use futures::stream::StreamExt;
use futures_channel::mpsc;
use async_trait::async_trait;
use ws::{Handler, Sender, Handshake, Message, CloseCode};

const CONNECTION: &'static str = "ws://127.0.0.1:12345";

enum WebsocketIncomingMessage {
    Sender(ws::Sender),
    Text(String),
}

struct InternalClient {
    out: Sender,
    buttplug_out: mpsc::UnboundedSender<WebsocketIncomingMessage>,
}

impl Handler for InternalClient {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        println!("Opened websocket");
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        println!("Got message: {}", msg);
        self.out.close(CloseCode::Normal)
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
    }

    fn on_error(&mut self, err: ws::Error) {
        println!("The server encountered an error: {:?}", err);
    }
}

pub struct ButtplugWebsocketClientConnector
{
    ws_thread: Option<thread::JoinHandle<()>>,
    websocket_to: Option<ws::Sender>,
    websocket_from: mpsc::UnboundedSender<WebsocketIncomingMessage>,
    recv: Option<mpsc::UnboundedReceiver<WebsocketIncomingMessage>>,
}

impl ButtplugWebsocketClientConnector {
    pub fn new() -> ButtplugWebsocketClientConnector {
        let (send, recv) = mpsc::unbounded();
        ButtplugWebsocketClientConnector {
            ws_thread: None,
            websocket_to: None,
            websocket_from: send,
            recv: Some(recv),
        }
    }

    async fn wait(&mut self) {
        if let Some(ref mut recv) = self.recv {
            while let Some(msg) = recv.next().await {
            }
        }
    }
}

#[async_trait]
impl ButtplugClientConnector for ButtplugWebsocketClientConnector {
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError> {
        let send = self.websocket_from.clone();
        self.ws_thread = Some(thread::spawn(|| {
            ws::connect(CONNECTION, move |out| {
                // Get our websocket sender back to the main thread
                send.unbounded_send(WebsocketIncomingMessage::Sender(out.clone())).unwrap();
                // Go ahead and create our internal client
                InternalClient {
                    out: out,
                    buttplug_out: send.clone()
                }
            }).unwrap();
        }));
        None
    }

    fn disconnect(&mut self) -> Option<ButtplugClientConnectorError> {
        None
    }

    async fn send(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        if let Some(sender) = &self.websocket_to {
            sender.send(serde_json::to_string(&msg).unwrap());
        }
        Result::Ok(ButtplugMessageUnion::Ok(messages::Ok::new(0)))
    }
}
