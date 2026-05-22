// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use dashmap::DashMap;
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::UserDeviceIdentifier;

use super::{
  ConfigVersion, ConfigVersionGetter, device::ConfigUserDeviceDefinition,
  get_internal_config_version, protocol::ProtocolDefinition,
};

#[derive(Deserialize, Serialize, Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub struct UserDeviceConfigPair {
  pub identifier: UserDeviceIdentifier,
  pub config: ConfigUserDeviceDefinition,
}

#[derive(Deserialize, Serialize, Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub struct SimulatedDeviceConfigEntry {
  pub identifier: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub display_name: Option<String>,
  #[serde(default = "SimulatedDeviceConfigEntry::generate_address")]
  pub address: String,
}

impl SimulatedDeviceConfigEntry {
  pub fn new(identifier: &str, display_name: Option<String>) -> Self {
    Self {
      identifier: identifier.to_owned(),
      display_name,
      address: Self::generate_address(),
    }
  }

  fn generate_address() -> String {
    format!("simulated:{}", Uuid::new_v4())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedDeviceArchetype {
  pub identifier: String,
  pub display_name: String,
  pub output_features: Vec<SimulatedDeviceFeatureSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedDeviceFeatureSummary {
  pub description: String,
  pub output_type: String,
  pub index: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct UserConfigDefinition {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub protocols: Option<DashMap<String, ProtocolDefinition>>,
  #[serde(rename = "devices", default, skip_serializing_if = "Option::is_none")]
  pub user_device_configs: Option<Vec<UserDeviceConfigPair>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub simulated_devices: Option<Vec<SimulatedDeviceConfigEntry>>,
}

#[derive(Deserialize, Serialize, Debug, Getters, Setters)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct UserConfigFile {
  version: ConfigVersion,
  #[serde(default)]
  user_configs: Option<UserConfigDefinition>,
}

impl Default for UserConfigFile {
  fn default() -> Self {
    Self {
      version: get_internal_config_version(),
      user_configs: Some(UserConfigDefinition::default()),
    }
  }
}

impl ConfigVersionGetter for UserConfigFile {
  fn version(&self) -> ConfigVersion {
    self.version
  }
}

impl UserConfigFile {
  pub fn new(major_version: u32, minor_version: u32) -> Self {
    Self {
      version: ConfigVersion {
        major: major_version,
        minor: minor_version,
      },
      user_configs: None,
    }
  }

  #[allow(dead_code)]
  pub fn to_json(&self) -> String {
    serde_json::to_string(self)
      .expect("All types below this are Serialize, so this should be infallible.")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::load_protocol_configs;

  #[test]
  fn test_simulated_device_config_roundtrip() {
    // AC1.2, AC5.2: Create a UserConfigFile with simulated_devices entries,
    // serialize to JSON, deserialize back, verify fields match
    let config = UserConfigDefinition {
      simulated_devices: Some(vec![
        SimulatedDeviceConfigEntry::new("simulated-1vibe", None),
        SimulatedDeviceConfigEntry::new("simulated-2vibe", Some("My Custom 2-Vibe".to_string())),
      ]),
      ..Default::default()
    };

    let user_config_file = UserConfigFile {
      version: get_internal_config_version(),
      user_configs: Some(config),
    };

    // Serialize to JSON
    let json_str = user_config_file.to_json();

    // Deserialize back
    let deserialized: UserConfigFile =
      serde_json::from_str(&json_str).expect("Roundtrip should work");

    // Verify fields match
    let user_cfg = deserialized
      .user_configs()
      .clone()
      .expect("Should have user_configs");
    let devices = user_cfg
      .simulated_devices
      .as_ref()
      .expect("Should have simulated_devices");

    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].identifier, "simulated-1vibe");
    assert_eq!(devices[0].display_name, None);
    assert!(devices[0].address.starts_with("simulated:"));

    assert_eq!(devices[1].identifier, "simulated-2vibe");
    assert_eq!(
      devices[1].display_name,
      Some("My Custom 2-Vibe".to_string())
    );
    assert!(devices[1].address.starts_with("simulated:"));
  }

  #[test]
  fn test_simulated_device_address_auto_generation() {
    // AC5.3: Address should auto-generate with simulated: prefix and UUID
    let entry = SimulatedDeviceConfigEntry::new("simulated-1vibe", None);

    assert!(entry.address.starts_with("simulated:"));
    assert!(entry.address.len() > "simulated:".len());
  }

  #[test]
  fn test_simulated_device_display_name_optional() {
    // AC5.4: Display name can be omitted from JSON when None
    let entry_with_display_name =
      SimulatedDeviceConfigEntry::new("simulated-1vibe", Some("My Device".to_string()));
    let entry_without_display_name = SimulatedDeviceConfigEntry::new("simulated-1vibe", None);

    let json_with = serde_json::to_value(&entry_with_display_name).unwrap();
    let json_without = serde_json::to_value(&entry_without_display_name).unwrap();

    // Entry with display_name should have the field in JSON
    assert!(json_with.get("display_name").is_some());

    // Entry without display_name should NOT have the field in JSON (skip_serializing_if = "Option::is_none")
    assert!(json_without.get("display_name").is_none());
  }

  #[test]
  fn test_invalid_archetype_rejected() {
    // AC8.1: Invalid archetype identifier should produce error at config load time
    let mut builder = load_protocol_configs(&None, &None, false).expect("Should load base configs");

    // Add an invalid simulated device entry
    let invalid_entry = SimulatedDeviceConfigEntry::new("nonexistent-device", None);
    builder.add_simulated_devices(vec![invalid_entry]);

    // Attempt to finish should error because the archetype doesn't exist
    let result = builder.finish();
    assert!(
      result.is_err(),
      "Should reject nonexistent archetype at config load time"
    );
  }

  #[test]
  fn test_duplicate_address_rejected() {
    // AC8.2: Duplicate addresses should be rejected
    let mut builder = load_protocol_configs(&None, &None, false).expect("Should load base configs");

    // Create two devices with the same explicit address
    let addr = "simulated:duplicate-test".to_string();
    let device1 = SimulatedDeviceConfigEntry {
      identifier: "simulated-1vibe".to_string(),
      display_name: None,
      address: addr.clone(),
    };
    let device2 = SimulatedDeviceConfigEntry {
      identifier: "simulated-1vibe".to_string(),
      display_name: Some("Duplicate".to_string()),
      address: addr.clone(),
    };

    builder.add_simulated_devices(vec![device1, device2]);

    // Should error on duplicate address
    let result = builder.finish();
    assert!(result.is_err(), "Should reject duplicate addresses");
  }
}
