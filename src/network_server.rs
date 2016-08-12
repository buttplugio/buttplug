use std::net::SocketAddr;
use std::str::FromStr;
use bytes::{ByteBuf};
use mio::*;
use mio::tcp::*;

// struct NetworkServer {
//     sock: TcpStream,
//     token: Token,
//     interest: EventSet,
//     send_queue: Vec<ByteBuf>,
// }

pub fn start_server(network_address: String) {
    let addr: SocketAddr = FromStr::from_str(&network_address)
        .ok().expect("Failed to parse host:port string");
    let sock = TcpListener::bind(&addr).ok().expect("Failed to bind address");

    //let mut event_loop = EventLoop::new().ok().expect("Failed to create event loop");

    // Create our Server object and register that with the event loop. I am hiding away
    // the details of how registering works inside of the `Server#register` function. One reason I
    // really like this is to get around having to have `const SERVER = Token(0)` at the top of my
    // file. It also keeps our polling options inside `Server`.
    // let mut server = NetworkServer::new(sock);
    // server.register(&mut event_loop).ok().expect("Failed to register server with event loop");

    // info!("Even loop starting...");
    // event_loop.run(&mut server).ok().expect("Failed to start event loop");
}
