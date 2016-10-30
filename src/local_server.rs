use std;
use mio::deprecated::{EventLoop, Handler, Sender};
use messages::{Message, Internal, Shutdown, IncomingMessage};

pub struct LocalServer {
    core_tx: Sender<IncomingMessage>,
    local_server_tx: std::sync::mpsc::Sender<Message>
}

pub fn start_server(core_tx: Sender<IncomingMessage>,
                    local_server_tx: std::sync::mpsc::Sender<Message>,
                    mut event_loop: EventLoop<LocalServer>) {
    info!("Event loop starting...");
    event_loop.run(&mut LocalServer { core_tx: core_tx, local_server_tx: local_server_tx }).expect("Failed to start event loop");
}

impl Handler for LocalServer {
    type Timeout = usize;
    type Message = Message;
    /// A message has been delivered
    fn notify(&mut self, _reactor: &mut EventLoop<LocalServer>, msg: Message) {
        let new_out = self.local_server_tx.clone();
        let incoming_msg = IncomingMessage {
            msg: msg.clone(),
            callback: Box::new(move |out_msg| {
                new_out.send(out_msg);
            })
        };
        self.core_tx.send(incoming_msg);
        match msg {
            Message::Internal(m) => {
                match m {
                    Internal::Shutdown(_) => {
                        _reactor.shutdown();
                    }
                }
            },
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;
    use mio::deprecated::EventLoop;
    use buttplug_server;
    use super::{start_server};
    use std:: thread;
    use config::Config;
    use messages::{Shutdown, Message, RequestServerInfo, Host};

    #[test]
    fn test_local_server_shutdown() {
        let (tx, rx) = channel();
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let server_tx = event_loop.channel();
        let child = thread::spawn(move|| {
            buttplug_server::start_server(Config::default(), Some(event_loop), Some(tx));
        });
        server_tx.send(Shutdown::as_message());
        child.join();
    }

    #[test]
    fn test_local_server_info() {
        let (tx, rx) = channel();
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let server_tx = event_loop.channel();
        let child = thread::spawn(move|| {
            buttplug_server::start_server(Config::default(), Some(event_loop), Some(tx));
        });
        server_tx.send(RequestServerInfo::as_message());
        match rx.recv().unwrap() {
            Message::Host(h) => {
                match h {
                    Host::ServerInfo(m) => {
                    },
                    _ => {
                        panic!("Wrong message type received!");
                    }
                }
            },
            _ => {
                panic!("Wrong message type received!");
            }
        }

        server_tx.send(Shutdown::as_message());
        child.join();
    }
}
