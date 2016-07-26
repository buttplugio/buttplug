#[macro_use]
extern crate clap;
use clap::{App};

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("buttplug-cli.yml");
    let matches = App::from_yaml(yaml)
        .version(crate_version!())
        .author(crate_authors!())
        .get_matches();
}

