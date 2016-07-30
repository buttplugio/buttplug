#[macro_use]
extern crate clap;
use clap::{App};
extern crate ws;
use ws::listen;

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("buttplug-cli.yml");
    let matches = App::from_yaml(yaml)
        .version(crate_version!())
        .author(crate_authors!())
        .get_matches();

    if let Err(error) = listen(matches.value_of("address").unwrap(), |out| {
        move |msg| {
            out.send(msg)
        }
    }) {
        println!("Failed to create websocket server!");
        println!("{:?}", error);
    }
}
