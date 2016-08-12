use std::net::SocketAddr;

pub struct Config {
    pub webhost_address: Option<SocketAddr>,
    pub websocket_address: Option<SocketAddr>,
    pub network_address: Option<SocketAddr>,
}

impl Config {
    pub fn null_config() -> Config {
        Config {
            webhost_address: None,
            websocket_address: None,
            network_address: None,
        }
    }

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
