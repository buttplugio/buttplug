use mio::deprecated::{EventLoop, Handler, Sender};
use messages::{Message, InternalMessage, Shutdown};

pub struct LocalServer {
    core_tx: Sender<Message>
}

pub fn start_server(core_tx: Sender<Message>, mut event_loop: EventLoop<LocalServer>) {
    info!("Event loop starting...");
    event_loop.run(&mut LocalServer { core_tx: core_tx }).expect("Failed to start event loop");
}

impl Handler for LocalServer {
    type Timeout = usize;
    type Message = Message;
    /// A message has been delivered
    fn notify(&mut self, _reactor: &mut EventLoop<LocalServer>, msg: Message) {
        match msg {
            Message::Internal(m) => {
                match m {
                    InternalMessage::Shutdown(_) => {
                        //self.core_tx.send(msg).expect("Can't send?!");
                        self.core_tx.send(Shutdown::as_message());
                        _reactor.shutdown();
                    }
                }
            },
            // Message::Device(_, _) => {
            //     self.device_manager.handle_message(&msg);
            // },
            _ => {
                warn!("Don't know what to do with this message!");
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use mio::deprecated::EventLoop;
    use buttplug_server;
    use super::{start_server};
    use std:: thread;
    use config::Config;
    use messages::{Shutdown, Message, InternalMessage};

    #[test]
    fn test_local_server_shutdown() {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let server_tx = event_loop.channel();
        let child = thread::spawn(move|| {
            buttplug_server::start_server(Config::default(), Some(event_loop));
        });
        println!("Waiting on send");
        server_tx.send(Shutdown::as_message());
        child.join();
    }
}
