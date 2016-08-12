use mio::*;
use messages::{Message};

pub struct LocalServer {
    core_tx: Sender<Message>
}

pub fn start_server(core_tx: Sender<Message>, mut event_loop: EventLoop<LocalServer>) {
    info!("Event loop starting...");
    event_loop.run(&mut LocalServer { core_tx: core_tx }).ok().expect("Failed to start event loop");
}

impl Handler for LocalServer {
    type Timeout = usize;
    type Message = Message;
    /// A message has been delivered
    fn notify(&mut self, _reactor: &mut EventLoop<LocalServer>, msg: Message) {
        match msg {
            Message::TestShutdown(_) => self.core_tx.send(msg).ok().expect("Can't send?!"),
            Message::Shutdown(_) => _reactor.shutdown(),
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
    use messages::{Shutdown, Message, TestShutdown};
    // #[test]
    // fn test_local_server_shutdown() {
    //     let event_loop = EventLoop::new().ok().expect("Failed to create event loop");
    //     let server_tx = event_loop.channel();
    //     let child = thread::spawn(move|| {
    //         start_server(new_tx, event_loop);
    //     });
    //     println!("Waiting on send");
    //     server_tx.send(Message::Shutdown(Shutdown::new()));
    //     child.join();
    // }

    #[test]
    fn test_buttplug_server_shutdown() {
        let event_loop = EventLoop::new().ok().expect("Failed to create event loop");
        let server_tx = event_loop.channel();
        let child = thread::spawn(move|| {
            buttplug_server::start_server(Config::null_config(), Some(event_loop));
        });
        println!("Waiting on send");
        server_tx.send(Message::TestShutdown(TestShutdown::new()));
        child.join();
    }
}
