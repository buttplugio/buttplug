use serde_json;
use std::thread;
use std::net::SocketAddr;
use ws::{Sender, Handler, Handshake, Message, CloseCode, Builder, Result};
use mio;
use messages;
use messages::{Log,Shutdown};

struct WebsocketServer {
    core_tx: mio::Sender<messages::Message>,
    out: Sender,
}

pub struct WebsocketThread {
    pub thread: thread::JoinHandle<()>,
    pub sender: Sender
}

impl Handler for WebsocketServer {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let s = msg.into_text().unwrap();
        //let m : messages::Message = serde_json::from_str(&s).unwrap();
        let mut m : messages::Message;
        match serde_json::from_str(&s) {
            Ok(msg) => m = msg,
            Err(e) => { warn!("{}", e); return Ok(()); }
        }
        self.core_tx.send(m);
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("Closed!");
    }
}

pub fn start_server(core_tx: mio::Sender<messages::Message>, addr : SocketAddr) -> WebsocketThread {
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
