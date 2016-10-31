#![feature(proc_macro, custom_attribute)]

extern crate ws;
extern crate lovesense;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate bytes;
extern crate mio;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate libtrancevibe;

use config::{Config};

mod local_server;
mod websocket_server;
mod devices;
pub mod buttplug_server;
pub mod messages;
pub mod config;

pub fn start_server(config: Config,
                    local_server_loop: Option<mio::deprecated::EventLoop<local_server::LocalServer>>,
                    local_server_loop_tx: Option<std::sync::mpsc::Sender<messages::Message>>)
{
    env_logger::init().expect("Failed to init logger");
    buttplug_server::start_server(config, local_server_loop, local_server_loop_tx);
}
