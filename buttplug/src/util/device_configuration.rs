// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::json::JSONValidator;
use crate::{
  core::{errors::{ButtplugDeviceError, ButtplugError}, message::DeviceFeature},
  server::device::configuration::{
    BaseDeviceDefinition,
    BaseDeviceIdentifier,
    DeviceConfigurationManager,
    DeviceConfigurationManagerBuilder,
    ProtocolCommunicationSpecifier,
    UserDeviceDefinition,
    UserDeviceIdentifier,
  },
};
use dashmap::DashMap;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};

pub static DEVICE_CONFIGURATION_JSON: &str =
  include_str!("../../buttplug-device-config/build-config/buttplug-device-config-v3.json");
static DEVICE_CONFIGURATION_JSON_SCHEMA: &str = include_str!(
  "../../buttplug-device-config/device-config-v3/buttplug-device-config-schema-v3.json"
);

/// The top level configuration for a protocol. Contains all data about devices that can use the
/// protocol, as well as names, message attributes, etc... for different devices.
///
/// Example: A Kiiroo ProtocolDeviceConfiguration would contain the Bluetooth LE information for all
/// devices supported under the Kiiroo protocol. It would also contain information about the names
/// and capabilities of different Kiiroo devices (Cliona, Onyx, Keon, etc...).
#[derive(Debug, Clone, Getters, MutGetters, Default)]
struct ProtocolDeviceConfiguration {
  /// BLE/USB/etc info for device identification.
  #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
  specifiers: Vec<ProtocolCommunicationSpecifier>,
  /// Names and message attributes for all possible devices that use this protocol
  #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
  configurations: HashMap<Option<String>, BaseDeviceDefinition>,
}

impl ProtocolDeviceConfiguration {
  /// Create a new instance
  pub fn new(
    specifiers: Vec<ProtocolCommunicationSpecifier>,
    configurations: HashMap<Option<String>, BaseDeviceDefinition>,
  ) -> Self {
    Self {
      specifiers,
      configurations,
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
struct ProtocolAttributes {
  #[serde(skip_serializing_if = "Option::is_none")]
  identifier: Option<Vec<String>>,
  name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  features: Option<Vec<DeviceFeature>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
struct ProtocolDefinition {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub communication: Option<Vec<ProtocolCommunicationSpecifier>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub defaults: Option<ProtocolAttributes>,
  #[serde(default)]
  pub configurations: Vec<ProtocolAttributes>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
struct UserDeviceConfigPair {
  identifier: UserDeviceIdentifier,
  config: UserDeviceDefinition,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
struct UserConfigDefinition {
  #[serde(skip_serializing_if = "Option::is_none")]
  protocols: Option<DashMap<String, ProtocolDefinition>>,
  #[serde(rename = "devices", default, skip_serializing_if = "Option::is_none")]
  user_device_configs: Option<Vec<UserDeviceConfigPair>>,
}

impl From<ProtocolDefinition> for ProtocolDeviceConfiguration {
  fn from(protocol_def: ProtocolDefinition) -> Self {
    let mut configurations = HashMap::new();

    if let Some(defaults) = protocol_def.defaults() {
      let config_attrs = BaseDeviceDefinition::new(
        &defaults.name,
        defaults
          .features
          .as_ref()
          .expect("This is a default, therefore we'll always have features."),
      );
      configurations.insert(None, config_attrs);
      for config in &protocol_def.configurations {
        if let Some(identifiers) = &config.identifier {
          for identifier in identifiers {
            let config_attrs = BaseDeviceDefinition::new(
              // Even subconfigurations always have names
              &config.name,
              config
                .features
                .as_ref()
                .or(Some(
                  defaults
                    .features
                    .as_ref()
                    .expect("Defaults always have features"),
                ))
                .unwrap(),
            );
            configurations.insert(Some(identifier.to_owned()), config_attrs);
          }
        }
      }
    }

    Self::new(protocol_def.communication.unwrap_or_default(), configurations)
  }
}

#[derive(Deserialize, Serialize, Debug, CopyGetters, Clone, Copy)]
#[getset(get_copy = "pub", get_mut = "pub")]
struct ConfigVersion {
  pub major: u32,
  pub minor: u32,
}

impl Display for ConfigVersion {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}.{}", self.major, self.minor)
  }
}

trait ConfigVersionGetter {
  fn version(&self) -> ConfigVersion;
}

#[derive(Deserialize, Serialize, Debug, Getters)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct BaseConfigFile {
  version: ConfigVersion,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  protocols: Option<HashMap<String, ProtocolDefinition>>,
}

impl Default for BaseConfigFile {
  fn default() -> Self {
    Self {
      version: get_internal_config_version(),
      protocols: Some(HashMap::new()),
    }
  }
}

impl ConfigVersionGetter for BaseConfigFile {
  fn version(&self) -> ConfigVersion {
    self.version
  }
}

impl BaseConfigFile {
  pub fn new(major_version: u32, minor_version: u32) -> Self {
    Self {
      version: ConfigVersion {
        major: major_version,
        minor: minor_version,
      },
      protocols: None,
    }
  }
}

#[derive(Deserialize, Serialize, Debug, Getters)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
struct UserConfigFile {
  version: ConfigVersion,
  #[serde(rename = "user-configs", default)]
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

fn get_internal_config_version() -> ConfigVersion {
  let config: BaseConfigFile = serde_json::from_str(DEVICE_CONFIGURATION_JSON)
    .expect("If this fails, the whole library goes with it.");
  config.version
}

fn load_protocol_config_from_json<'a, T>(
  config_str: &'a str,
  skip_version_check: bool,
) -> Result<T, ButtplugDeviceError>
where
  T: ConfigVersionGetter + Deserialize<'a>,
{
  let config_validator = JSONValidator::new(DEVICE_CONFIGURATION_JSON_SCHEMA);
  match config_validator.validate(config_str) {
    Ok(_) => match serde_json::from_str::<T>(config_str) {
      Ok(protocol_config) => {
        let internal_config_version = get_internal_config_version();
        if !skip_version_check && protocol_config.version().major != internal_config_version.major {
          Err(ButtplugDeviceError::DeviceConfigurationError(format!(
            "Device configuration file major version {} is different than internal major version {}. Cannot load external files that do not have matching major version numbers.",
            protocol_config.version(),
            internal_config_version
          )))
        } else {
          Ok(protocol_config)
        }
      }
      Err(err) => Err(ButtplugDeviceError::DeviceConfigurationError(format!(
        "{}",
        err
      ))),
    },
    Err(err) => Err(ButtplugDeviceError::DeviceConfigurationError(format!(
      "{}",
      err
    ))),
  }
}

fn load_main_config(
  main_config_str: &Option<String>,
  skip_version_check: bool
) -> Result <DeviceConfigurationManagerBuilder, ButtplugDeviceError> {
  if main_config_str.is_some() {
    info!("Loading from custom base device configuration...")
  } else {
    info!("Loading from internal base device configuration...")
  }
  // Start by loading the main config
  let main_config = load_protocol_config_from_json::<BaseConfigFile>(
    &main_config_str.as_ref().unwrap_or(&DEVICE_CONFIGURATION_JSON.to_owned()),
    skip_version_check,
  )?;

  let mut dcm_builder = DeviceConfigurationManagerBuilder::default();

  // Each protocol will need to become a ProtocolDeviceConfiguration, so we'll need to
  //
  // - take the specifiers from both the main and user configs and make a vector out of them
  // - for each configuration and user config, we'll need to create message lists and figure out
  //   what to do with allow/deny/index.

  let mut protocol_specifiers = HashMap::new();
  let mut protocol_features = HashMap::new();

  // Iterate through all of the protocols in the main config first and build up a map of protocol
  // name to ProtocolDeviceConfiguration structs.
  for (protocol_name, protocol_def) in main_config.protocols.unwrap_or_default() {
    let protocol_device_config: ProtocolDeviceConfiguration = protocol_def.into();
    protocol_specifiers.insert(
      protocol_name.clone(),
      protocol_device_config.specifiers().clone(),
    );
    for (config_ident, config) in protocol_device_config.configurations() {
      let ident = BaseDeviceIdentifier::new(&protocol_name, config_ident);
      protocol_features.insert(ident, config.clone());
    }
  }

  for (name, specifiers) in &protocol_specifiers {
    dcm_builder.communication_specifier(name, specifiers);
  }

  
  for (ident, features) in protocol_features {
    dcm_builder.protocol_features(&ident, &features);
  }

  Ok(dcm_builder)
}

fn load_user_config(
  user_config_str: &str,
  skip_version_check: bool,
  dcm_builder: &mut DeviceConfigurationManagerBuilder
) -> Result<(), ButtplugDeviceError> {

  info!("Loading user configuration from string.");
  let user_config_file = load_protocol_config_from_json::<UserConfigFile>(
    &user_config_str,
    skip_version_check,
  )?;

  if user_config_file.user_configs.is_none() {
    info!("No user configurations provided in user config.");
    return Ok(());
  }

  let user_config = user_config_file.user_configs.expect("Just checked validity");

  for (protocol, specifier) in user_config.protocols.unwrap_or_default() {
    if let Some(comm_specifiers) = specifier.communication() {
      dcm_builder.user_communication_specifier(&protocol, comm_specifiers);
    }
  }

  for user_device_config_pair in user_config.user_device_configs.unwrap_or_default() {
    dcm_builder.user_protocol_features(user_device_config_pair.identifier(), user_device_config_pair.config());
  }

  Ok(())
}

pub fn load_protocol_configs(
  main_config_str: &Option<String>,
  user_config_str: &Option<String>,
  skip_version_check: bool,
) -> Result<DeviceConfigurationManagerBuilder, ButtplugDeviceError> {

  let mut dcm_builder = load_main_config(main_config_str, skip_version_check)?;

  if let Some(config_str) = user_config_str {
    load_user_config(config_str, skip_version_check, &mut dcm_builder)?;
  } else {
    info!("No user configuration provided.");
  }

  Ok(dcm_builder)
}

pub fn save_user_config(dcm: &DeviceConfigurationManager) -> Result<String, ButtplugError> {
  let user_specifiers = dcm.user_communication_specifiers();
  let user_definitions_vec = dcm.user_device_definitions().iter().map(|kv| UserDeviceConfigPair {
    identifier: kv.key().clone(),
    config: kv.value().clone()
  }).collect();
  let user_protos = DashMap::new();
  for spec in user_specifiers {
    user_protos.insert(spec.key().clone(), ProtocolDefinition {
      communication: Some(spec.value().clone()),
      .. Default::default()
    });
  }
  let user_config_definition = UserConfigDefinition {
    protocols: Some(user_protos.clone()),
    user_device_configs: Some(user_definitions_vec)
  };
  let mut user_config_file = UserConfigFile::new(3, 0);
  user_config_file.user_configs = Some(user_config_definition);
  Ok(serde_json::to_string(&user_config_file).map_err(|e| ButtplugError::from(ButtplugDeviceError::DeviceConfigurationError(format!(
    "Cannot save device configuration file: {e:?}",
  ))))?)
}
