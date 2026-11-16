// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::collections::{BTreeMap, HashMap, HashSet};

use buttplug_core::util::json::JSONValidator;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};

const VERSION_FILE: &str = "./device-config/version.yaml";
const OUTPUT_FILE: &str = "./build-config/buttplug-device-config-v5.json";
const PROTOCOL_DIR: &str = "./device-config/protocols/";
const SCHEMA_FILE: &str = "./device-config/buttplug-device-config-schema-v5.json";

/// Protocols whose configurations are alternate layouts of the same device. Their feature ids
/// identify logical channels and are shared across layouts so user feature settings survive a
/// layout change.
const SHARED_FEATURE_ID_PROTOCOLS: &[&str] = &["sdl-gamepad"];

/// Device ids must be unique across every protocol, since user configs resolve their base device
/// globally by id. Feature ids must be unique within a device (user feature configs resolve by id
/// within their base device) and, outside of `SHARED_FEATURE_ID_PROTOCOLS`, across all devices.
fn check_unique_ids(protocols: &BTreeMap<String, Value>) {
  let mut device_ids: HashMap<&str, String> = HashMap::new();
  let mut feature_ids: HashMap<&str, String> = HashMap::new();
  let mut errors = vec![];

  for (protocol, config) in protocols {
    let shared_features = SHARED_FEATURE_ID_PROTOCOLS.contains(&protocol.as_str());
    let protocol_prefix = format!("{protocol}.");
    let definitions = config
      .get("defaults")
      .map(|d| ("defaults".to_owned(), d))
      .into_iter()
      .chain(
        config
          .get("configurations")
          .and_then(Value::as_array)
          .into_iter()
          .flatten()
          .enumerate()
          .map(|(i, c)| (format!("configurations[{i}]"), c)),
      );

    for (definition_path, definition) in definitions {
      let location = format!("{protocol}.{definition_path}");
      if let Some(id) = definition.get("id").and_then(Value::as_str)
        && let Some(previous) = device_ids.insert(id, location.clone())
      {
        errors.push(format!("device id {id}: {previous} and {location}"));
      }

      let mut definition_feature_ids = HashSet::new();
      let features = definition
        .get("features")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
      for (i, feature) in features.enumerate() {
        let Some(id) = feature.get("id").and_then(Value::as_str) else {
          continue;
        };
        let feature_location = format!("{location}.features[{i}]");
        if !definition_feature_ids.insert(id) {
          errors.push(format!(
            "feature id {id}: repeated within {location} at {feature_location}"
          ));
          continue;
        }
        match feature_ids.get(id) {
          Some(previous) if !(shared_features && previous.starts_with(&protocol_prefix)) => {
            errors.push(format!(
              "feature id {id}: {previous} and {feature_location}"
            ));
          }
          Some(_) => {}
          None => {
            feature_ids.insert(id, feature_location);
          }
        }
      }
    }
  }

  for (id, location) in &device_ids {
    if let Some(feature_location) = feature_ids.get(id) {
      errors.push(format!(
        "id {id}: used as device id at {location} and feature id at {feature_location}"
      ));
    }
  }

  if !errors.is_empty() {
    errors.sort();
    panic!(
      "Duplicate ids in device config protocol files:\n  {}",
      errors.join("\n  ")
    );
  }
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
struct VersionFile {
  version: BuildVersion,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
struct BuildVersion {
  pub major: u32,
  pub minor: u32,
}

#[derive(Deserialize, Serialize, Eq, PartialEq)]
struct JsonOutputFile {
  version: BuildVersion,
  protocols: BTreeMap<String, Value>,
}

fn main() {
  println!("cargo:rerun-if-changed={}", PROTOCOL_DIR);

  // Open version file
  let mut version: VersionFile =
    serde_yaml::from_str(&std::fs::read_to_string(VERSION_FILE).unwrap()).unwrap();
  // Bump minor version
  version.version.minor += 1;

  // Compile device config file
  let mut output = JsonOutputFile {
    // lol
    version: version.version,
    protocols: BTreeMap::new(),
  };

  for protocol_file in std::fs::read_dir(PROTOCOL_DIR).unwrap() {
    let f = protocol_file.unwrap();
    output.protocols.insert(
      f.file_name()
        .into_string()
        .unwrap()
        .split(".")
        .next()
        .unwrap()
        .to_owned(),
      serde_yaml::from_str(&std::fs::read_to_string(f.path()).unwrap()).unwrap(),
    );
  }

  let json = serde_json::to_string_pretty(&output).unwrap();

  // Validate
  let validator = JSONValidator::new(&std::fs::read_to_string(SCHEMA_FILE).unwrap());
  validator.validate(&json).unwrap();
  check_unique_ids(&output.protocols);

  // See if it's actually different than our last output file
  if let Ok(true) = std::fs::exists(OUTPUT_FILE) {
    let old_output: JsonOutputFile =
      serde_json::from_str(&std::fs::read_to_string(OUTPUT_FILE).unwrap()).unwrap();
    if old_output.protocols == output.protocols {
      // No actual changes, break out early, don't save
      return;
    }
  }

  // Save it to the build_config directory
  std::fs::write(
    VERSION_FILE,
    serde_yaml::to_string(&version).unwrap().as_bytes(),
  )
  .unwrap();
  std::fs::write(OUTPUT_FILE, json.as_bytes()).unwrap();
}
