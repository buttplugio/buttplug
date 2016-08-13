use mio::{EventLoop, Handler, Sender};
use std::thread;
use std::vec::{Vec};
use messages::{Message, Shutdown};
use config::{Config};
// for start_server
use local_server;
use local_server::{LocalServer};
use websocket_server;
use ws;

pub fn start_server(config: Config,
                    local_server_loop: Option<EventLoop<LocalServer>>) {
    let mut event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut server = ButtplugServer::new(config, local_server_loop, event_loop.channel());
    event_loop.run(&mut server).expect("Failed to start event loop");
}

pub struct ButtplugServer {
    threads: Vec<thread::JoinHandle<()>>,
    channels: Vec<Sender<Message>>,
    websocket_sender: Option<ws::Sender>,
    tx: Sender<Message>,
    // TODO: field for Currently open devices
    // TODO: field for Device lists?
}

impl ButtplugServer {
    pub fn new(config: Config,
               local_server_loop: Option<EventLoop<LocalServer>>,
               tx: Sender<Message>) -> ButtplugServer {
        let mut server_threads = vec![];
        let mut channels = vec![];
        let mut sender = None;
        if let Some(_) = config.network_address {
            info!("Starting network server");
            // threads.push(thread::spawn(move|| {
            //     network_server::start_server(network_address);
            // }));
        }
        if let Some(wsaddr) = config.websocket_address {
            info!("Starting websocket server");
            let ws = websocket_server::start_server(tx.clone(), wsaddr);
            server_threads.push(ws.thread);
            sender = Some(ws.sender);
        }
        if let Some(local_server_loop) = local_server_loop {
            info!("Starting local server");
            channels.push(local_server_loop.channel());
            let server_tx = tx.clone();
            server_threads.push(thread::spawn(move|| {
                local_server::start_server(server_tx, local_server_loop);
            }));
        }

        ButtplugServer {
            threads: server_threads,
            tx: tx,
            websocket_sender: sender,
            channels: channels
        }
    }

    fn shutdown(&mut self) {
        for c in &self.channels {
            // If we're shutting down, there's a chance the message came through
            // the local server, which will have shut itself down first. Just
            // ignore the error.
            if let Err(_) = c.send(Message::Shutdown(Shutdown::new())) {
                info!("Thread already shutdown!");
            }
        }
        if let Some(ref ws) = self.websocket_sender {
            ws.shutdown();
        }
        // Drain the vector here so we have ownership, since joining is
        // join(self)
        let ts = self.threads.drain(..);
        for t in ts {
            t.join().expect("Could not join thread!");
        }
    }
}

impl Handler for ButtplugServer {
    type Timeout = usize;
    type Message = Message;
    /// A message has been delivered
    fn notify(&mut self, _reactor: &mut EventLoop<ButtplugServer>, msg: Message) {
        match msg {
            Message::Shutdown(_) => {
                self.shutdown();
                _reactor.shutdown();
            },
            _ => println!("Don't care!")
        };
    }
}
