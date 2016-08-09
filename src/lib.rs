#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate ws;
extern crate lovesense;
extern crate serde_json;

use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use ws::listen;

pub mod client;
pub mod websocket_client;
pub mod messages;

struct Client {
    // Name
    // Some sort of network connection
    // A list of claimed devices
}

struct Device {
    // Name
    // Representing Object
    // Connection ID (COM Port, socket path, bluetooth ID, etc)
}

pub fn start_websocket_server(address: &str) {
    if let Err(error) = listen(address, |out| {
        move |msg| {
            out.send(msg)
        }
    }) {
        println!("Failed to create websocket server!");
        println!("{:?}", error);
    }
}

pub fn start_server(websocket_address: Option<String>,
                    network_address: Option<String>)
{
    let (tx, rx) = channel();
    tx.send(messages::Message::ClaimDevice(messages::ClaimDevice::new(1)));
    let mut threads = vec![];
    if let Some(websocket_address) = websocket_address {
        threads.push(thread::spawn(move|| {
            if let Err(error) = listen("127.0.0.1:9000", |out| {
                move |msg| {
                    out.send(msg)
                }
            }) {
                println!("Failed to create websocket server!");
                println!("{:?}", error);
            }
        }));
    }
}
