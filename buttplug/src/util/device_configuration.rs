// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::json::JSONValidator;
use crate::{
  core::{errors::ButtplugDeviceError, message::DeviceFeature},
  server::device::configuration::{
    BaseDeviceDefinition,
    BaseDeviceIdentifier,
    BluetoothLESpecifier,
    DeviceConfigurationManager,
    DeviceConfigurationManagerBuilder,
    HIDSpecifier,
    LovenseConnectServiceSpecifier,
    ProtocolCommunicationSpecifier,
    SerialSpecifier,
    USBSpecifier,
    UserDeviceCustomization,
    UserDeviceDefinition,
    UserDeviceIdentifier,
    WebsocketSpecifier,
    XInputSpecifier,
  },
};
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

#[derive(Serialize, Deserialize, Debug, Getters, Setters, Default, Clone)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
struct UserDeviceConfig {
  #[serde(rename = "name")]
  name: String,
  #[serde(default)]
  features: Vec<DeviceFeature>,
  #[serde(rename = "user-config")]
  user_config: UserDeviceCustomization,
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
  // Can't get serde flatten specifiers into a String/DeviceSpecifier map, so
  // they're kept separate here, and we return them in specifiers(). Feels
  // very clumsy, but we really don't do this a bunch during a session.
  #[serde(skip_serializing_if = "Option::is_none")]
  usb: Option<Vec<USBSpecifier>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  btle: Option<BluetoothLESpecifier>,
  #[serde(skip_serializing_if = "Option::is_none")]
  serial: Option<Vec<SerialSpecifier>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  hid: Option<Vec<HIDSpecifier>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  xinput: Option<XInputSpecifier>,
  #[serde(skip_serializing_if = "Option::is_none")]
  websocket: Option<WebsocketSpecifier>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "lovense-connect-service")]
  lovense_connect_service: Option<LovenseConnectServiceSpecifier>,
  #[serde(skip_serializing_if = "Option::is_none")]
  defaults: Option<ProtocolAttributes>,
  #[serde(default)]
  configurations: Vec<ProtocolAttributes>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
struct UserDeviceConfigPair {
  identifier: UserConfigDeviceIdentifier,
  config: UserDeviceConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
struct UserConfigDefinition {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  specifiers: Option<HashMap<String, ProtocolDefinition>>,
  #[serde(rename = "devices", default, skip_serializing_if = "Option::is_none")]
  user_device_configs: Option<Vec<UserDeviceConfigPair>>,
}

#[derive(
  Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters, Eq, PartialEq, Hash,
)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
struct UserConfigDeviceIdentifier {
  pub address: String,
  pub protocol: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub identifier: Option<String>,
}

impl From<UserConfigDeviceIdentifier> for UserDeviceIdentifier {
  fn from(ident: UserConfigDeviceIdentifier) -> Self {
    let server_identifier = if let Some(ident_string) = ident.identifier {
      Some(ident_string)
    } else {
      None
    };
    UserDeviceIdentifier::new(&ident.address, &ident.protocol, &server_identifier)
  }
}

impl From<UserDeviceIdentifier> for UserConfigDeviceIdentifier {
  fn from(ident: UserDeviceIdentifier) -> Self {
    UserConfigDeviceIdentifier {
      address: ident.address().clone(),
      protocol: ident.protocol().clone(),
      identifier: ident.attributes_identifier().clone(),
    }
  }
}

#[derive(Default, Debug, Getters)]
#[getset(get = "pub")]
struct ExternalDeviceConfiguration {
  protocol_specifiers: HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  protocol_features: HashMap<BaseDeviceIdentifier, BaseDeviceDefinition>,
  user_configs: HashMap<UserDeviceIdentifier, UserDeviceDefinition>,
}

impl From<ProtocolDefinition> for ProtocolDeviceConfiguration {
  fn from(protocol_def: ProtocolDefinition) -> Self {
    // Make a vector out of the protocol definition specifiers
    let mut specifiers = vec![];
    if let Some(usb_vec) = &protocol_def.usb {
      usb_vec
        .iter()
        .for_each(|spec| specifiers.push(ProtocolCommunicationSpecifier::USB(*spec)));
    }
    if let Some(serial_vec) = &protocol_def.serial {
      serial_vec
        .iter()
        .for_each(|spec| specifiers.push(ProtocolCommunicationSpecifier::Serial(spec.clone())));
    }
    if let Some(hid_vec) = &protocol_def.hid {
      hid_vec
        .iter()
        .for_each(|spec| specifiers.push(ProtocolCommunicationSpecifier::HID(*spec)));
    }
    if let Some(btle) = &protocol_def.btle {
      specifiers.push(ProtocolCommunicationSpecifier::BluetoothLE(btle.clone()));
    }
    if let Some(xinput) = &protocol_def.xinput {
      specifiers.push(ProtocolCommunicationSpecifier::XInput(*xinput));
    }
    if let Some(websocket) = &protocol_def.websocket {
      specifiers.push(ProtocolCommunicationSpecifier::Websocket(websocket.clone()));
    }
    if let Some(lcs) = &protocol_def.lovense_connect_service {
      specifiers.push(ProtocolCommunicationSpecifier::LovenseConnectService(
        lcs.clone(),
      ));
    }

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

    Self::new(specifiers, configurations)
  }
}

fn add_user_configs_to_protocol(
  external_config: &mut ExternalDeviceConfiguration,
  user_config_def: UserConfigDefinition,
) {
  if let Some(specifiers) = user_config_def.specifiers() {
    for (user_config_protocol, protocol_def) in specifiers {
      if !external_config
        .protocol_specifiers
        .contains_key(user_config_protocol)
      {
        continue;
      }

      let base_protocol_def = external_config
        .protocol_specifiers
        .get_mut(user_config_protocol)
        .unwrap();

      // Make a vector out of the protocol definition specifiers
      if let Some(usb_vec) = &protocol_def.usb {
        usb_vec
          .iter()
          .for_each(|spec| base_protocol_def.push(ProtocolCommunicationSpecifier::USB(*spec)));
      }
      if let Some(serial_vec) = &protocol_def.serial {
        serial_vec.iter().for_each(|spec| {
          base_protocol_def.push(ProtocolCommunicationSpecifier::Serial(spec.clone()))
        });
      }
      if let Some(hid_vec) = &protocol_def.hid {
        hid_vec
          .iter()
          .for_each(|spec| base_protocol_def.push(ProtocolCommunicationSpecifier::HID(*spec)));
      }
      if let Some(btle) = &protocol_def.btle {
        base_protocol_def.push(ProtocolCommunicationSpecifier::BluetoothLE(btle.clone()));
      }
      if let Some(websocket) = &protocol_def.websocket {
        base_protocol_def.push(ProtocolCommunicationSpecifier::Websocket(websocket.clone()));
      }
    }
  }
  if let Some(user_device_configs) = user_config_def.user_device_configs() {
    for user_config in user_device_configs {
      let server_ident: UserDeviceIdentifier = user_config.identifier.clone().into();
      debug!("Server Ident: {:?}", server_ident);

      let config_attrs = UserDeviceDefinition::new(
        user_config.config().name(),
        user_config.config().features(),
        user_config.config().user_config(),
      );
      info!("Adding user config for {:?}", server_ident);
      external_config
        .user_configs
        .insert(server_ident, config_attrs);
    }
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
pub struct ProtocolConfiguration {
  version: ConfigVersion,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  protocols: Option<HashMap<String, ProtocolDefinition>>,
}

impl Default for ProtocolConfiguration {
  fn default() -> Self {
    Self {
      version: get_internal_config_version(),
      protocols: Some(HashMap::new()),
    }
  }
}

impl ConfigVersionGetter for ProtocolConfiguration {
  fn version(&self) -> ConfigVersion {
    self.version
  }
}

impl ProtocolConfiguration {
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
struct UserProtocolConfiguration {
  version: ConfigVersion,
  #[serde(rename = "user-configs", default)]
  user_configs: Option<UserConfigDefinition>,
}

impl Default for UserProtocolConfiguration {
  fn default() -> Self {
    Self {
      version: get_internal_config_version(),
      user_configs: Some(UserConfigDefinition::default()),
    }
  }
}

impl ConfigVersionGetter for UserProtocolConfiguration {
  fn version(&self) -> ConfigVersion {
    self.version
  }
}

impl UserProtocolConfiguration {
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
  let config: ProtocolConfiguration = serde_json::from_str(DEVICE_CONFIGURATION_JSON)
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

fn load_protocol_configs_internal(
  main_config_str: Option<String>,
  user_config_str: Option<String>,
  skip_version_check: bool,
) -> Result<ExternalDeviceConfiguration, ButtplugDeviceError> {
  if main_config_str.is_some() {
    info!("Loading from custom base device configuration...")
  } else {
    info!("Loading from internal base device configuration...")
  }
  // Start by loading the main config
  let main_config = load_protocol_config_from_json::<ProtocolConfiguration>(
    &main_config_str.unwrap_or_else(|| DEVICE_CONFIGURATION_JSON.to_owned()),
    skip_version_check,
  )?;

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

  let mut external_config = ExternalDeviceConfiguration {
    protocol_specifiers,
    protocol_features,
    ..Default::default()
  };

  // Then load the user config
  if let Some(user_config) = user_config_str {
    info!("Loading user configuration from string.");
    let config = load_protocol_config_from_json::<UserProtocolConfiguration>(
      &user_config,
      skip_version_check,
    )?;
    if let Some(user_configs) = config.user_configs {
      add_user_configs_to_protocol(&mut external_config, user_configs);
    }
  } else {
    info!("No user configuration given.");
  }

  Ok(external_config)
}

pub fn load_protocol_configs(
  main_config_str: Option<String>,
  user_config_str: Option<String>,
  skip_version_check: bool,
) -> Result<DeviceConfigurationManagerBuilder, ButtplugDeviceError> {
  let mut dcm_builder = DeviceConfigurationManagerBuilder::default();

  let external_config =
    load_protocol_configs_internal(main_config_str, user_config_str, skip_version_check)?;

  for (name, specifiers) in external_config.protocol_specifiers() {
    for spec in specifiers {
      dcm_builder.communication_specifier(name, spec.clone());
    }
  }

  for (ident, features) in external_config.protocol_features() {
    dcm_builder.protocol_features(ident.clone(), features.clone());
  }

  for (ident, features) in external_config.user_configs() {
    dcm_builder.user_protocol_features(ident.clone(), features.clone());
  }

  Ok(dcm_builder)
}

pub fn create_test_dcm(allow_raw_messages: bool) -> DeviceConfigurationManager {
  let devices = load_protocol_configs_internal(None, None, false)
    .expect("If this fails, the whole library goes with it.");
  let mut builder = DeviceConfigurationManagerBuilder::default();
  if allow_raw_messages {
    builder.allow_raw_messages();
  }
  for (name, specifiers) in devices.protocol_specifiers {
    for spec in specifiers {
      builder.communication_specifier(&name, spec);
    }
  }
  for (ident, def) in devices.protocol_features {
    builder.protocol_features(ident, def);
  }
  builder
    .finish()
    .expect("If this fails, the whole library goes with it.")
}
