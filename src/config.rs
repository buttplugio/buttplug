use std::net::SocketAddr;

pub struct Config {
    pub webhost_address: Option<SocketAddr>,
    pub websocket_address: Option<SocketAddr>,
    pub network_address: Option<SocketAddr>,
    pub local_server: Option<bool>
}

impl Config {
    pub fn new(webhost_address: Option<SocketAddr>,
               websocket_address: Option<SocketAddr>,
               network_address: Option<SocketAddr>,
               local_server: Option<bool>) -> Config
    {
        Config {
            webhost_address: webhost_address,
            websocket_address: websocket_address,
            network_address: network_address,
            local_server: local_server
        }
    }
}
