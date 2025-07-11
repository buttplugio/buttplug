use std::collections::BTreeMap;

use serde_yaml;
use serde_json::{self, Value};
use serde::{Serialize, Deserialize};
use buttplug_core::util::json::JSONValidator;

const VERSION_FILE: &str = "./device-config-v4/version.yaml";
const OUTPUT_FILE: &str = "./build-config/buttplug-device-config-v4.json";
const PROTOCOL_DIR: &str = "./device-config-v4/protocols/";
const SCHEMA_FILE: &str = "./device-config-v4/buttplug-device-config-schema-v4.json";

#[derive(Serialize, Deserialize, Eq, PartialEq)]
struct VersionFile {
  version: BuildVersion
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
struct BuildVersion {
  pub major: u32,
  pub minor: u32
}

#[derive(Deserialize, Serialize, Eq, PartialEq)]
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

  let json = serde_json::to_string_pretty(&output).unwrap();

  // Validate
  let validator = JSONValidator::new(&std::fs::read_to_string(SCHEMA_FILE).unwrap());
  validator.validate(&json).unwrap();

  // See if it's actually different than our last output file
  if let Ok(true) = std::fs::exists(OUTPUT_FILE) {
    let old_output: JsonOutputFile = serde_json::from_str(&std::fs::read_to_string(OUTPUT_FILE).unwrap()).unwrap();
    if old_output.protocols == output.protocols {
      // No actual changes, break out early, don't save
      return;
    }
  }

  // Save it to the build_config directory
 std::fs::write(VERSION_FILE, serde_yaml::to_string(&version).unwrap().as_bytes()).unwrap();
 std::fs::write(OUTPUT_FILE, json.as_bytes()).unwrap();
}