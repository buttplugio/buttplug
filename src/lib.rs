#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate ws;
extern crate lovesense;
extern crate serde_json;
extern crate bytes;
extern crate mio;
#[macro_use] extern crate log;
extern crate env_logger;

use config::{Config};

mod local_server;
mod websocket_server;
pub mod buttplug_server;
pub mod messages;
pub mod config;

pub fn start_server(config: Config,
                    local_server_loop: Option<mio::deprecated::EventLoop<local_server::LocalServer>>)
{
    env_logger::init().expect("Failed to init logger");
    buttplug_server::start_server(config, local_server_loop);
}
