use std::collections::BTreeMap;

use serde_yaml;
use serde_json::{self, Value};
use serde::{Serialize, Deserialize};

const VERSION_FILE: &str = "./device-config-v4/version.yaml";
const OUTPUT_FILE: &str = "./build-config/build-device-config-v4.json";
const PROTOCOL_DIR: &str = "./device-config-v4/protocols/";

#[derive(Serialize, Deserialize)]
struct VersionFile {
  version: BuildVersion
}

#[derive(Serialize, Deserialize)]
struct BuildVersion {
  pub major: u32,
  pub minor: u32
}

#[derive(Serialize)]
struct JsonOutputFile {
  version: BuildVersion,
  protocols: BTreeMap<String, Value>
}

fn main() {
  println!("cargo:rerun-if-changed={}",  PROTOCOL_DIR);

  // Open version file
  let mut version: VersionFile = serde_yaml::from_str(&std::fs::read_to_string(VERSION_FILE).unwrap()).unwrap();
  // Bump minor version
  version.version.minor += 1;
  std::fs::write(VERSION_FILE, serde_yaml::to_string(&version).unwrap().as_bytes()).unwrap();

  // Compile device config file
  let mut output = JsonOutputFile {
    // lol
    version: version.version,
    protocols: BTreeMap::new()
  };

  for protocol_file in std::fs::read_dir(PROTOCOL_DIR).unwrap() {
    let f = protocol_file.unwrap();
    output.protocols.insert(f.file_name().into_string().unwrap().split(".").next().unwrap().to_owned(), serde_yaml::from_str(&std::fs::read_to_string(f.path()).unwrap()).unwrap());
  }

  // Save it to the build_config directory
  std::fs::write(OUTPUT_FILE, serde_json::to_string(&output).unwrap().as_bytes()).unwrap();
}