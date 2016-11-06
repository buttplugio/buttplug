use mio::deprecated::{EventLoop, Handler, Sender};
use serde_json;
use mio::channel;
use std::thread;
use std;
use std::vec::{Vec};
use messages;
use messages::{Message, IncomingMessage, Shutdown, Internal, Host, Client, ServerInfo, ClaimDevice};
use config::{Config};
// for start_server
use local_server;
use local_server::{LocalServer};
use websocket_server;
use devices::{DeviceManager};
use ws;

pub fn start_server(config: Config,
                    local_server_loop: Option<EventLoop<LocalServer>>,
                    local_server_test_tx: Option<std::sync::mpsc::Sender<Message>>) {
    let mut event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut server = ButtplugServer::new(config,
                                         local_server_loop,
                                         local_server_test_tx,
                                         event_loop.channel());
    event_loop.run(&mut server).expect("Failed to start event loop");
}

pub struct ButtplugServer {
    threads: Vec<thread::JoinHandle<()>>,
    websocket_sender: Option<ws::Sender>,
    tx: Sender<IncomingMessage>,
    device_manager: DeviceManager,
    // TODO: field for Currently open devices
    // TODO: field for Device lists?
}

impl ButtplugServer {
    pub fn new(config: Config,
               local_server_loop: Option<EventLoop<LocalServer>>,
               local_server_test_tx: Option<std::sync::mpsc::Sender<Message>>,
               tx: Sender<IncomingMessage>) -> ButtplugServer {
        let mut server_threads = vec![];
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
            let unwrapped_local_server_test_tx = match local_server_test_tx {
                Some(m) => m,
                None => panic!("Require tx with local server loop!")
            };
            info!("Starting local server");
            let server_tx = tx.clone();
            server_threads.push(thread::spawn(move|| {
                local_server::start_server(server_tx, unwrapped_local_server_test_tx, local_server_loop);
            }));
        }
        println!("{}", serde_json::to_string(&ServerInfo::as_message("Testing".to_string())).unwrap());
        println!("{}", serde_json::to_string(&ClaimDevice::as_message(1)).unwrap());
        ButtplugServer {
            threads: server_threads,
            tx: tx,
            websocket_sender: sender,
            device_manager: DeviceManager::new()
        }
    }

    fn shutdown(&mut self) {
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
    type Message = IncomingMessage;
    /// A message has been delivered
    fn notify(&mut self, _reactor: &mut EventLoop<ButtplugServer>, msg: IncomingMessage) {
        match msg.msg {
            Message::Internal(m) => {
                match m {
                    Internal::Shutdown(_) => {
                        self.shutdown();
                        _reactor.shutdown();
                    }
                }
            },
            Message::Client(m) => {
                match m {
                    Client::RequestServerInfo(_) => {
                        let s = messages::ServerInfo::as_message("Buttplug v0.0.1".to_string());
                        (msg.callback)(s);
                    }
                    _ => {
                        warn!("Don't know what to do with this client message!");
                    }
                }
            },
            Message::Device(_, _) => {
                info!("Got device message!");
                self.device_manager.handle_message(msg);
            },
            _ => {
                warn!("Don't know how to handle this host message!");
            }
        };
    }
}
