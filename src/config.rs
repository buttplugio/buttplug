use std::net::SocketAddr;

#[derive(Default)]
pub struct Config {
    pub webhost_address: Option<SocketAddr>,
    pub websocket_address: Option<SocketAddr>,
    pub network_address: Option<SocketAddr>,
}

impl Config {
    pub fn new(webhost_address: Option<SocketAddr>,
               websocket_address: Option<SocketAddr>,
               network_address: Option<SocketAddr>) -> Config
    {
        Config {
            webhost_address: webhost_address,
            websocket_address: websocket_address,
            network_address: network_address
        }
    }
}
