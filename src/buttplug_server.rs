use mio::{EventLoop, Handler, Sender};
use std::thread;
use std::vec::{Vec};
use messages::{Message, Shutdown};
use config::{Config};
// for start_server
use local_server;
use local_server::{LocalServer};
//use websocket_server::{Websters};

pub fn start_server(config: Config,
                    local_server_loop: Option<EventLoop<LocalServer>>) {
    let mut event_loop = EventLoop::new().ok().expect("Failed to create event loop");
    let mut server = ButtplugServer::new(config, local_server_loop, event_loop.channel());
    event_loop.run(&mut server).ok().expect("Failed to start event loop");
}

pub struct ButtplugServer {
    threads: Vec<thread::JoinHandle<()>>,
    channels: Vec<Sender<Message>>,
    tx: Sender<Message>
}

impl ButtplugServer {
    pub fn new(config: Config,
               local_server_loop: Option<EventLoop<LocalServer>>,
               tx: Sender<Message>) -> ButtplugServer {
        let mut server_threads = vec![];
        let mut channels = vec![];
        // if let Some(config.network_address) = network_address {
        //     threads.push(thread::spawn(move|| {
        //         network_server::start_server(network_address);
        //     }));
        // }
        if let Some(local_server_loop) = local_server_loop {
            channels.push(local_server_loop.channel());
            let server_tx = tx.clone();
            server_threads.push(thread::spawn(move|| {
                local_server::start_server(server_tx, local_server_loop);
            }));
        }

        ButtplugServer {
            threads: server_threads,
            tx: tx,
            channels: channels
        }
    }
}

impl Handler for ButtplugServer {
    type Timeout = usize;
    type Message = Message;
    /// A message has been delivered
    fn notify(&mut self, _reactor: &mut EventLoop<ButtplugServer>, msg: Message) {
        match msg {
            Message::TestShutdown(_) => {
                self.channels.iter().cloned().map(|x| { x.send(Message::Shutdown(Shutdown::new())) });
                // Join and remove all threads from the vector. We're done anyways.
                self.threads.drain(..).map(|x| { x.join(); });
                _reactor.shutdown();
            },
            _ => println!("Don't care!")
        };
    }
}
