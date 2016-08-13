#[macro_use]
extern crate clap;
use clap::{App};

extern crate buttplug;
use std::net::SocketAddr;
use buttplug::start_server;
use buttplug::config::Config;

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("buttplug-cli.yml");
    let matches = App::from_yaml(yaml)
        .version(crate_version!())
        .author(crate_authors!())
        .get_matches();

    if !matches.is_present("netaddr") && !matches.is_present("wsaddr") {
        panic!("Either a network host and/or a web socket host address must specified!");
    }
    let mut webaddr : Option<SocketAddr> = None;
    let mut netaddr : Option<SocketAddr> = None;
    let mut wsaddr : Option<SocketAddr> = None;
    // TODO: Should probably have a slightly nicer UI and cleaner parsing than
    // this :|
    if matches.is_present("webaddr") {
        webaddr = Some(matches.value_of("webaddr")
            .unwrap()
            .parse::<SocketAddr>()
            .unwrap());
    }
    if matches.is_present("netaddr") {
        netaddr = Some(matches.value_of("netaddr")
            .unwrap()
            .parse::<SocketAddr>()
            .unwrap());
    }
    if matches.is_present("wsaddr") {
        wsaddr = Some(matches.value_of("wsaddr")
            .unwrap()
            .parse::<SocketAddr>()
            .unwrap());
    }

    start_server(Config::new(webaddr, wsaddr, netaddr), None);
}
