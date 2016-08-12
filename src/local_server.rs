use mio::*;
use messages::{Message};

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
            Message::Shutdown(_) => {
                self.core_tx.send(msg).expect("Can't send?!");
                _reactor.shutdown();
            },
            _ => println!("Don't care!")
        };
    }
}

#[cfg(test)]
mod tests {
    use mio::*;
    use buttplug_server;
    use super::{start_server};
    use std:: thread;
    use config::Config;
    use messages::{Shutdown, Message};

    #[test]
    fn test_local_server_shutdown() {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let server_tx = event_loop.channel();
        let child = thread::spawn(move|| {
            buttplug_server::start_server(Config::default(), Some(event_loop));
        });
        println!("Waiting on send");
        server_tx.send(Message::Shutdown(Shutdown::new()));
        child.join();
    }
}
