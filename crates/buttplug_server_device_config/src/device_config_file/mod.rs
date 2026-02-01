// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod base;
mod device;
mod feature;
mod protocol;
mod user;

use base::BaseConfigFile;

use crate::device_config_file::{
  protocol::ProtocolDefinition,
  user::{UserConfigDefinition, UserConfigFile, UserDeviceConfigPair},
};

use super::{BaseDeviceIdentifier, DeviceConfigurationManager, DeviceConfigurationManagerBuilder};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError},
  util::json::JSONValidator,
};
use dashmap::DashMap;
use getset::CopyGetters;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub static DEVICE_CONFIGURATION_JSON: &str =
  include_str!("../../build-config/buttplug-device-config-v4.json");
static DEVICE_CONFIGURATION_JSON_SCHEMA: &str =
  include_str!("../../device-config-v4/buttplug-device-config-schema-v4.json");

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

fn get_internal_config_version() -> ConfigVersion {
  let config: BaseConfigFile = serde_json::from_str(DEVICE_CONFIGURATION_JSON)
    .expect("If this fails, the whole library goes with it.");
  config.version()
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
        "{err}"
      ))),
    },
    Err(err) => Err(ButtplugDeviceError::DeviceConfigurationError(format!(
      "{err}"
    ))),
  }
}

fn load_main_config(
  main_config_str: &Option<String>,
  skip_version_check: bool,
) -> Result<DeviceConfigurationManagerBuilder, ButtplugDeviceError> {
  if main_config_str.is_some() {
    info!("Loading from custom base device configuration...")
  } else {
    info!("Loading from internal base device configuration...")
  }
  // Start by loading the main config
  let main_config = load_protocol_config_from_json::<BaseConfigFile>(
    main_config_str
      .as_ref()
      .unwrap_or(&DEVICE_CONFIGURATION_JSON.to_owned()),
    skip_version_check,
  )?;

  info!("Loaded config version {:?}", main_config.version());

  let mut dcm_builder = DeviceConfigurationManagerBuilder::default();

  for (protocol_name, protocol_def) in main_config.protocols().clone().unwrap_or_default() {
    if let Some(specifiers) = protocol_def.communication() {
      dcm_builder.communication_specifier(&protocol_name, specifiers);
    }

    let mut default = None;
    if let Some(features) = protocol_def.defaults() {
      default = Some(features.clone());
      dcm_builder.base_device_definition(
        &BaseDeviceIdentifier::new_default(&protocol_name),
        &features.clone().into(),
      );
    }

    for config in protocol_def.configurations() {
      if let Some(idents) = config.identifier() {
        for config_ident in idents {
          let ident = BaseDeviceIdentifier::new_with_identifier(&protocol_name, config_ident);
          if let Some(d) = &default {
            dcm_builder
              .base_device_definition(&ident, &d.update_with_configuration(config.clone()).into());
          } else {
            dcm_builder.base_device_definition(&ident, &config.clone().into());
          }
        }
      }
    }
  }

  Ok(dcm_builder)
}

fn load_user_config(
  user_config_str: &str,
  skip_version_check: bool,
  dcm_builder: &mut DeviceConfigurationManagerBuilder,
) -> Result<(), ButtplugDeviceError> {
  let base_dcm = dcm_builder.clone().finish().unwrap();

  info!("Loading user configuration from string.");
  let user_config_file =
    load_protocol_config_from_json::<UserConfigFile>(user_config_str, skip_version_check)?;

  if user_config_file.user_configs().is_none() {
    info!("No user configurations provided in user config.");
    return Ok(());
  }

  let user_config = user_config_file
    .user_configs()
    .clone()
    .expect("Just checked validity");

  for (protocol_name, protocol_def) in user_config.protocols().clone().unwrap_or_default() {
    if let Some(specifiers) = protocol_def.communication() {
      dcm_builder.user_communication_specifier(&protocol_name, specifiers);
    }

    // Defaults aren't valid in user config files. All we can do is create new configurations with
    // valid identifiers.

    for config in protocol_def.configurations() {
      if let Some(idents) = config.identifier() {
        for config_ident in idents {
          let ident = BaseDeviceIdentifier::new_with_identifier(&protocol_name, config_ident);
          dcm_builder.base_device_definition(&ident, &config.clone().into());
        }
      }
    }
  }

  for user_device_config_pair in user_config
    .user_device_configs()
    .clone()
    .unwrap_or_default()
  {
    //let ident = BaseDeviceIdentifier::new(user_device_config_pair.identifier().protocol(), &None);
    // Use device UUID instead of identifier to match here, otherwise we have to do really weird stuff with identifier hashes.
    // TODO How do we deal with user configs derived from default here? We don't handle loading this correctly?
    if let Some(base_config) = base_dcm
      .base_device_definitions()
      .iter()
      .find(|x| x.1.id() == user_device_config_pair.config().base_id())
    {
      if let Ok(loaded_user_config) = user_device_config_pair
        .config()
        .build_from_base_definition(base_config.1)
        && let Err(e) = dcm_builder
          .user_device_definition(user_device_config_pair.identifier(), &loaded_user_config)
      {
        error!(
          "Device definition not valid, skipping:\n{:?}\n{:?}",
          e, user_config
        )
      }
    } else {
      error!(
        "Device identifier {:?} does not have a match base identifier that matches anything in the base config, removing from database.",
        user_device_config_pair.identifier()
      );
    }
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
  let user_definitions_vec: Vec<_> = dcm
    .user_device_definitions()
    .iter()
    .map(|kv| {
      Ok(UserDeviceConfigPair {
        identifier: kv.key().clone(),
        config: kv.value().try_into().map_err(|e| {
          ButtplugError::from(ButtplugDeviceError::DeviceConfigurationError(format!(
            "Cannot convert device definition to user config: {e:?}",
          )))
        })?,
      })
    })
    .collect::<Result<_, ButtplugError>>()?;
  let user_protos = DashMap::new();
  for spec in user_specifiers {
    user_protos.insert(
      spec.key().clone(),
      ProtocolDefinition {
        communication: Some(spec.value().clone()),
        ..Default::default()
      },
    );
  }
  let user_config_definition = UserConfigDefinition {
    protocols: Some(user_protos.clone()),
    user_device_configs: Some(user_definitions_vec),
  };
  let mut user_config_file = UserConfigFile::new(4, 0);
  user_config_file.set_user_configs(Some(user_config_definition));
  serde_json::to_string_pretty(&user_config_file).map_err(|e| {
    ButtplugError::from(ButtplugDeviceError::DeviceConfigurationError(format!(
      "Cannot save device configuration file: {e:?}",
    )))
  })
}

#[cfg(test)]
mod test {
  use crate::device_config_file::load_main_config;

  use super::{DEVICE_CONFIGURATION_JSON, base::BaseConfigFile, load_protocol_config_from_json};

  #[test]
  fn test_config_file_parsing() {
    load_protocol_config_from_json::<BaseConfigFile>(&DEVICE_CONFIGURATION_JSON.to_owned(), true)
      .unwrap();
  }

  #[test]
  fn test_main_file_parsing() {
    load_main_config(&None, false).unwrap();
  }
}
