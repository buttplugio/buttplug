use super::json::JSONValidator;
use crate::{
  core::errors::{ButtplugDeviceError, ButtplugError},
  device::configuration_manager::{DeviceConfigurationManager, ProtocolDefinition},
  server::device_manager::DeviceUserConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub static DEVICE_CONFIGURATION_JSON: &str =
  include_str!("../../buttplug-device-config/buttplug-device-config.json");
static DEVICE_CONFIGURATION_JSON_SCHEMA: &str =
  include_str!("../../buttplug-device-config/buttplug-device-config-schema.json");

#[derive(Deserialize, Serialize, Debug)]
pub struct ProtocolConfiguration {
  pub version: u32,
  #[serde(default)]
  pub protocols: HashMap<String, ProtocolDefinition>,
  #[serde(rename = "user-config", default)]
  pub user_config: HashMap<String, DeviceUserConfig>,
}

impl Default for ProtocolConfiguration {
  fn default() -> Self {
    Self {
      version: get_internal_config_version(),
      protocols: HashMap::new(),
      user_config: HashMap::new(),
    }
  }
}

impl ProtocolConfiguration {
  pub fn merge(&mut self, other: ProtocolConfiguration) {
    // For now, we're only merging serial info in.
    for (protocol, conf) in other.protocols {
      if let Some(protocol_conf) = self.protocols.get_mut(&protocol) {
        protocol_conf.merge_user_definition(conf);
      } else {
        self.protocols.insert(protocol, conf);
      }
    }
    // Just copy the user config wholesale.
    self.user_config = other.user_config;
  }

  pub fn to_json(&self) -> String {
    serde_json::to_string(self)
      .expect("All types below this are Serialize, so this should be infallible.")
  }
}

pub fn get_internal_config_version() -> u32 {
  let config: ProtocolConfiguration = serde_json::from_str(DEVICE_CONFIGURATION_JSON)
    .expect("If this fails, the whole library goes with it.");
  config.version
}

pub fn load_protocol_config_from_json(
  config_str: &str,
  skip_version_check: bool,
) -> Result<ProtocolConfiguration, ButtplugError> {
  let config_validator = JSONValidator::new(DEVICE_CONFIGURATION_JSON_SCHEMA);
  match config_validator.validate(config_str) {
    Ok(_) => match serde_json::from_str::<ProtocolConfiguration>(config_str) {
      Ok(protocol_config) => {
        for (_, protocol_def) in &protocol_config.protocols {
          for default in protocol_def.defaults() {
            for message_map in default.messages() {
              for (key, value) in message_map {
                value.check(key).map_err(|err| ButtplugError::from(err))?;
              } 
            }
          }
          for configs in protocol_def.configurations() {
            for message_map in configs.messages() {
              for (key, value) in message_map {
                value.check(key).map_err(|err| ButtplugError::from(err))?;
              } 
            }
          }
        }
        let internal_config_version = get_internal_config_version();
        if !skip_version_check && protocol_config.version < internal_config_version {
          Err(ButtplugDeviceError::DeviceConfigurationFileError(format!(
            "Device configuration file version {} is older than internal version {}. Please use a newer file.",
            protocol_config.version,
            internal_config_version
          )).into())
        } else {
          Ok(protocol_config)
        }
      }
      Err(err) => Err(ButtplugDeviceError::DeviceConfigurationFileError(format!("{}", err)).into()),
    },
    Err(err) => Err(ButtplugDeviceError::DeviceConfigurationFileError(format!("{}", err)).into()),
  }
}

pub fn create_test_dcm(allow_raw_messages: bool) -> DeviceConfigurationManager {
  let devices = load_protocol_config_from_json(DEVICE_CONFIGURATION_JSON, false)
    .expect("If this fails, the whole library goes with it.");
  let dcm = DeviceConfigurationManager::new(allow_raw_messages);
  for (name, def) in devices.protocols {
    dcm.add_protocol_definition(&name, def);
  }
  dcm
}
