use serde_json;
use std::thread;
use std::net::SocketAddr;
use ws::{Sender, Handler, Handshake, Message, CloseCode, Builder, Result};
use mio;
use messages;

struct WebsocketServer {
    core_tx: mio::deprecated::Sender<messages::IncomingMessage>,
    out: Sender,
}

pub struct WebsocketThread {
    pub thread: thread::JoinHandle<()>,
    pub sender: Sender
}

impl Handler for WebsocketServer {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        info!("New websocket connection established");
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let s = msg.into_text().unwrap();
        debug!("Got new message: {:?}", s);
        let m = match serde_json::from_str(&s) {
            Ok(msg) => msg,
            Err(e) => { warn!("{}", e); return Ok(()); }
        };
        let new_out = self.out.clone();
        let incoming_msg = messages::IncomingMessage {
            msg: m,
            callback: Box::new(move |out_msg| {
                let out_msg_str = serde_json::to_string(&out_msg).unwrap();
                new_out.send(out_msg_str);
            })
        };
        self.core_tx.send(incoming_msg);
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        info!("Websocket connection closed: {}.", reason);
        // TODO Shutdown and close all open devices for this connection.
    }
}

pub fn start_server(core_tx: mio::deprecated::Sender<messages::IncomingMessage>,
                    addr : SocketAddr) -> WebsocketThread {
    let socket = Builder::new().build(move |out: Sender| {
        WebsocketServer { out: out,
                          core_tx: core_tx.clone()
        }
    }).unwrap();
    WebsocketThread {
        sender: socket.broadcaster(),
        thread: thread::spawn(move|| {
            socket.listen(addr).unwrap();
        })
    }
}
