// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::json::JSONValidator;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::DeviceMessageAttributesMap,
  },
  server::device::{
    configuration::{
      BluetoothLESpecifier, DeviceConfigurationManager, DeviceConfigurationManagerBuilder, HIDSpecifier, LovenseConnectServiceSpecifier,
      ProtocolAttributesIdentifier, ProtocolCommunicationSpecifier, ProtocolDeviceAttributes,
      ProtocolDeviceConfiguration, SerialSpecifier, USBSpecifier,
      WebsocketSpecifier, XInputSpecifier,
    },
    ServerDeviceIdentifier,
  }
};
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

pub static DEVICE_CONFIGURATION_JSON: &str =
  include_str!("../../buttplug-device-config/buttplug-device-config.json");
static DEVICE_CONFIGURATION_JSON_SCHEMA: &str =
  include_str!("../../buttplug-device-config/buttplug-device-config-schema.json");

#[derive(Serialize, Deserialize, Debug, Getters, Setters, Default, Clone, PartialEq)]
#[getset(get = "pub", set = "pub")]
pub struct DeviceUserConfig {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  #[serde(rename = "display-name")]
  display_name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  allow: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  deny: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  messages: Option<DeviceMessageAttributesMap>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  index: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ProtocolAttributes {
  #[serde(skip_serializing_if = "Option::is_none")]
  identifier: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  messages: Option<DeviceMessageAttributesMap>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct ProtocolDefinition {
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
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct UserConfigDefinition {
  specifiers: HashMap<String, ProtocolDefinition>,
  #[serde(rename = "devices", with = "vectorize")]
  user_configs: HashMap<ServerDeviceIdentifier, DeviceUserConfig>,
}

#[derive(Default, Debug, Getters)]
#[getset(get = "pub")]
pub struct ExternalDeviceConfiguration {
  allow_list: Vec<String>,
  deny_list: Vec<String>,
  reserved_indexes: HashMap<u32, ServerDeviceIdentifier>,
  protocol_configurations: HashMap<String, ProtocolDeviceConfiguration>,
  user_configs: HashMap<ServerDeviceIdentifier, ProtocolDeviceAttributes>,
}

impl From<ProtocolDefinition> for ProtocolDeviceConfiguration {
  fn from(protocol_def: ProtocolDefinition) -> Self {
    // Make a vector out of the protocol definition specifiers
    let mut specifiers = vec![];
    if let Some(usb_vec) = protocol_def.usb {
      usb_vec
        .iter()
        .for_each(|spec| specifiers.push(ProtocolCommunicationSpecifier::USB(*spec)));
    }
    if let Some(serial_vec) = protocol_def.serial {
      serial_vec
        .iter()
        .for_each(|spec| specifiers.push(ProtocolCommunicationSpecifier::Serial(spec.clone())));
    }
    if let Some(hid_vec) = protocol_def.hid {
      hid_vec
        .iter()
        .for_each(|spec| specifiers.push(ProtocolCommunicationSpecifier::HID(*spec)));
    }
    if let Some(btle) = protocol_def.btle {
      specifiers.push(ProtocolCommunicationSpecifier::BluetoothLE(btle));
    }
    if let Some(xinput) = protocol_def.xinput {
      specifiers.push(ProtocolCommunicationSpecifier::XInput(xinput));
    }
    if let Some(websocket) = protocol_def.websocket {
      specifiers.push(ProtocolCommunicationSpecifier::Websocket(websocket));
    }
    if let Some(lcs) = protocol_def.lovense_connect_service {
      specifiers.push(ProtocolCommunicationSpecifier::LovenseConnectService(lcs));
    }

    let mut configurations = HashMap::new();

    let default_attrs = if let Some(defaults) = protocol_def.defaults {
      let default_attrs = Arc::new(ProtocolDeviceAttributes::new(
        ProtocolAttributesIdentifier::Default,
        defaults.name,
        None,
        defaults.messages.unwrap_or_default(),
        None,
      ));
      configurations.insert(ProtocolAttributesIdentifier::Default, default_attrs.clone());
      Some(default_attrs)
    } else {
      None
    };

    for config in protocol_def.configurations {
      if let Some(identifiers) = config.identifier {
        for identifier in identifiers {
          let config_attrs = Arc::new(ProtocolDeviceAttributes::new(
            ProtocolAttributesIdentifier::Identifier(identifier.clone()),
            config.name.clone(),
            None,
            config.messages.clone().unwrap_or_default(),
            default_attrs.clone(),
          ));
          configurations.insert(
            ProtocolAttributesIdentifier::Identifier(identifier),
            config_attrs,
          );
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
  for (user_config_protocol, protocol_def) in user_config_def.specifiers() {
    if !external_config
      .protocol_configurations
      .contains_key(user_config_protocol)
    {
      continue;
    }

    let base_protocol_def = external_config
      .protocol_configurations
      .get_mut(user_config_protocol)
      .unwrap();

    // Make a vector out of the protocol definition specifiers
    if let Some(usb_vec) = &protocol_def.usb {
      usb_vec.iter().for_each(|spec| {
        base_protocol_def
          .specifiers_mut()
          .push(ProtocolCommunicationSpecifier::USB(*spec))
      });
    }
    if let Some(serial_vec) = &protocol_def.serial {
      serial_vec.iter().for_each(|spec| {
        base_protocol_def
          .specifiers_mut()
          .push(ProtocolCommunicationSpecifier::Serial(spec.clone()))
      });
    }
    if let Some(hid_vec) = &protocol_def.hid {
      hid_vec.iter().for_each(|spec| {
        base_protocol_def
          .specifiers_mut()
          .push(ProtocolCommunicationSpecifier::HID(*spec))
      });
    }
    if let Some(btle) = &protocol_def.btle {
      base_protocol_def
        .specifiers_mut()
        .push(ProtocolCommunicationSpecifier::BluetoothLE(btle.clone()));
    }
    if let Some(websocket) = &protocol_def.websocket {
      base_protocol_def
        .specifiers_mut()
        .push(ProtocolCommunicationSpecifier::Websocket(websocket.clone()));
    }
  }
  for (specifier, user_config) in user_config_def.user_configs() {
    if *user_config.allow().as_ref().unwrap_or(&false) {
      external_config.allow_list.push(specifier.address().clone());
    }
    if *user_config.deny().as_ref().unwrap_or(&false) {
      external_config.deny_list.push(specifier.address().clone());
    }
    if let Some(index) = user_config.index().as_ref() {
      external_config
        .reserved_indexes
        .insert(*index, specifier.clone());
    }
    let config_attrs = ProtocolDeviceAttributes::new(
      specifier.identifier().clone(),
      None,
      user_config.display_name.clone(),
      user_config.messages.clone().unwrap_or_default(),
      None,
    );
    external_config
      .user_configs
      .insert(specifier.clone(), config_attrs);
  }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProtocolConfiguration {
  pub version: u32,
  #[serde(default)]
  pub protocols: Option<HashMap<String, ProtocolDefinition>>,
  #[serde(rename = "user-configs", default)]
  pub user_configs: Option<UserConfigDefinition>,
}

impl Default for ProtocolConfiguration {
  fn default() -> Self {
    Self {
      version: get_internal_config_version(),
      protocols: Some(HashMap::new()),
      user_configs: Some(UserConfigDefinition::default()),
    }
  }
}

impl ProtocolConfiguration {
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
        let internal_config_version = get_internal_config_version();
        if !skip_version_check && protocol_config.version < internal_config_version {
          Err(ButtplugDeviceError::DeviceConfigurationError(format!(
            "Device configuration file version {} is older than internal version {}. Please use a newer file.",
            protocol_config.version,
            internal_config_version
          )).into())
        } else {
          Ok(protocol_config)
        }
      }
      Err(err) => Err(ButtplugDeviceError::DeviceConfigurationError(format!("{}", err)).into()),
    },
    Err(err) => Err(ButtplugDeviceError::DeviceConfigurationError(format!("{}", err)).into()),
  }
}

pub fn load_protocol_configs_from_json(
  main_config_str: Option<String>,
  user_config_str: Option<String>,
  skip_version_check: bool,
) -> Result<ExternalDeviceConfiguration, ButtplugError> {
  // Start by loading the main config
  let main_config = load_protocol_config_from_json(
    &main_config_str.unwrap_or_else(|| DEVICE_CONFIGURATION_JSON.to_owned()),
    skip_version_check,
  )?;

  // Each protocol will need to become a ProtocolDeviceConfiguration, so we'll need to
  //
  // - take the specifiers from both the main and user configs and make a vector out of them
  // - for each configuration and user config, we'll need to create message lists and figure out
  //   what to do with allow/deny/index.

  let mut protocols: HashMap<String, ProtocolDeviceConfiguration> = HashMap::new();

  // Iterate through all of the protocols in the main config first and build up a map of protocol
  // name to ProtocolDeviceConfiguration structs.
  for (protocol_name, protocol_def) in main_config.protocols.unwrap_or_default() {
    protocols.insert(protocol_name, protocol_def.into());
  }

  let mut external_config = ExternalDeviceConfiguration {
    protocol_configurations: protocols,
    ..Default::default()
  };

  // Then load the user config
  if let Some(user_config) = user_config_str {
    let config = load_protocol_config_from_json(&user_config, skip_version_check)?;
    if let Some(user_configs) = config.user_configs {
      add_user_configs_to_protocol(&mut external_config, user_configs);
    }
  }

  Ok(external_config)
}

pub fn create_test_dcm(allow_raw_messages: bool) -> DeviceConfigurationManager {
  let devices = load_protocol_configs_from_json(None, None, false)
    .expect("If this fails, the whole library goes with it.");
  let mut builder = DeviceConfigurationManagerBuilder::default();
   if allow_raw_messages {
    builder.allow_raw_messages();
  }
  for (name, def) in devices.protocol_configurations {
    builder
      .protocol_device_configuration(&name, &def);
  }
  builder.finish().expect("If this fails, the whole library goes with it.")
}
