use serde_json;
use std::thread;
use std::net::SocketAddr;
use ws::{Sender, Handler, Handshake, Message, CloseCode, Builder, Result};
use messages::{Log};

struct WebsocketServer {
    out: Sender,
}

pub struct WebsocketThread {
    pub thread: thread::JoinHandle<()>,
    pub sender: Sender
}

impl Handler for WebsocketServer {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("Opened!");
        let s = Log::new("testing string!".to_string());
        self.out.send(serde_json::to_string(&s).unwrap());
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        println!("Got Message!");
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("Closed!");
    }
}

pub fn start_server(addr : SocketAddr) -> WebsocketThread {
    let socket = Builder::new().build(move |out: Sender| {
        WebsocketServer { out: out }
    }).unwrap();
    WebsocketThread {
        sender: socket.broadcaster(),
        thread: thread::spawn(move|| {
            socket.listen(addr).unwrap();
        })
    }
}
