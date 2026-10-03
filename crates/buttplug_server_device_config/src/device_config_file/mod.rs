// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod base;
mod device;
mod feature;
pub(crate) use feature::ConfigUserDeviceFeature;
mod protocol;
mod user;

pub use user::{
  SimulatedDeviceArchetype,
  SimulatedDeviceConfigEntry,
  SimulatedDeviceFeatureSummary,
};

use base::BaseConfigFile;

use crate::device_config_file::{
  protocol::ProtocolDefinition,
  user::{UserConfigDefinition, UserConfigFile, UserDeviceConfigPair},
};

use super::{
  BaseDeviceIdentifier,
  DeviceConfigurationManager,
  DeviceConfigurationManagerBuilder,
  ServerDeviceDefinition,
};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError},
  util::json::JSONValidator,
};
use dashmap::DashMap;
use getset::CopyGetters;
use jsonschema::{ValidationError, error::ValidationErrorKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, fmt::Display, sync::Arc};

pub static DEVICE_CONFIGURATION_JSON: &str =
  include_str!("../../build-config/buttplug-device-config-v5.json");

/// JSON schema for a configuration file, with a name to use in log messages.
struct ConfigSchema {
  name: &'static str,
  schema: &'static str,
}

static DEVICE_CONFIGURATION_JSON_SCHEMA: ConfigSchema = ConfigSchema {
  name: "base device configuration",
  schema: include_str!("../../device-config/buttplug-device-config-schema-v5.json"),
};
static USER_DEVICE_CONFIGURATION_JSON_SCHEMA: ConfigSchema = ConfigSchema {
  name: "user device configuration",
  schema: include_str!("../../device-config/buttplug-user-device-config-schema-v5.json"),
};

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

/// Returns a copy of `schema` where every object schema that lists `properties` but doesn't set
/// `additionalProperties` rejects keys it doesn't list. Object schemas that set
/// `additionalProperties`, including `true` for intentionally open objects, are left as is.
fn strict_schema(schema: &Value) -> Value {
  match schema {
    Value::Object(object) => {
      let mut strict: serde_json::Map<String, Value> = object
        .iter()
        .map(|(key, value)| (key.clone(), strict_schema(value)))
        .collect();
      if strict.contains_key("properties") && !strict.contains_key("additionalProperties") {
        strict.insert("additionalProperties".to_owned(), Value::Bool(false));
      }
      Value::Object(strict)
    }
    Value::Array(items) => Value::Array(items.iter().map(strict_schema).collect()),
    other => other.clone(),
  }
}

/// Collects the JSON pointers of keys rejected by `additionalProperties`, including those found
/// while trying `anyOf`/`oneOf` branches.
fn collect_undocumented_keys(error: &ValidationError, keys: &mut BTreeSet<String>) {
  match error.kind() {
    ValidationErrorKind::AdditionalProperties { unexpected } => {
      for key in unexpected {
        keys.insert(format!(
          "{}/{}",
          error.instance_path().as_str(),
          key.replace('~', "~0").replace('/', "~1")
        ));
      }
    }
    ValidationErrorKind::AnyOf { context }
    | ValidationErrorKind::OneOfNotValid { context }
    | ValidationErrorKind::OneOfMultipleValid { context } => {
      for branch_error in context.iter().flatten() {
        collect_undocumented_keys(branch_error, keys);
      }
    }
    _ => {}
  }
}

/// Returns the JSON pointers of keys in `config_str` that `schema` doesn't declare. The schemas
/// accept these keys and the loader ignores them, but a future, stricter schema may reject them.
fn undocumented_keys(schema: &str, config_str: &str) -> BTreeSet<String> {
  let mut keys = BTreeSet::new();
  // Unparseable configs are reported by the regular validation that follows.
  let (Ok(schema), Ok(config)) = (
    serde_json::from_str::<Value>(schema),
    serde_json::from_str::<Value>(config_str),
  ) else {
    return keys;
  };
  let validator = jsonschema::validator_for(&strict_schema(&schema))
    .expect("schema must be valid JSON Schema (validated by build.rs)");
  for error in validator.iter_errors(&config) {
    collect_undocumented_keys(&error, &mut keys);
  }
  keys
}

fn load_protocol_config_from_json<'a, T>(
  config_str: &'a str,
  schema: &ConfigSchema,
  skip_version_check: bool,
) -> Result<T, ButtplugDeviceError>
where
  T: ConfigVersionGetter + Deserialize<'a>,
{
  for key in undocumented_keys(schema.schema, config_str) {
    warn!(
      "Ignoring undocumented key '{}' in {}. Future versions may reject it.",
      key, schema.name
    );
  }
  let config_validator = JSONValidator::new(schema.schema);
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
    &DEVICE_CONFIGURATION_JSON_SCHEMA,
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
        Arc::new(ServerDeviceDefinition::from(features.clone())),
      );
    }

    for config in protocol_def.configurations() {
      if let Some(idents) = config.identifier() {
        let definition: Arc<ServerDeviceDefinition> =
          Arc::new(config.clone().with_defaults(default.as_ref()).into());
        for config_ident in idents {
          let ident = BaseDeviceIdentifier::new_with_identifier(&protocol_name, config_ident);
          dcm_builder.base_device_definition(&ident, definition.clone());
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
  info!("Loading user configuration from string.");
  let user_config_file = load_protocol_config_from_json::<UserConfigFile>(
    user_config_str,
    &USER_DEVICE_CONFIGURATION_JSON_SCHEMA,
    skip_version_check,
  )?;

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
          dcm_builder.base_device_definition(&ident, Arc::new(config.clone().into()));
        }
      }
    }
  }

  // Snapshot taken after user-defined configurations are added so that user device config pairs
  // whose base_id refers to a configuration defined in the same user config file can be resolved.
  let base_dcm = dcm_builder.clone().finish().unwrap();

  for user_device_config_pair in user_config
    .user_device_configs()
    .clone()
    .unwrap_or_default()
  {
    // Use device UUID instead of identifier to match here, otherwise we have to do really weird stuff with identifier hashes.
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

  if let Some(simulated_devices) = user_config.simulated_devices().clone() {
    dcm_builder.simulated_devices(simulated_devices);
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
  let simulated_devices = dcm.simulated_devices();
  let simulated_devices = if simulated_devices.is_empty() {
    None
  } else {
    Some(simulated_devices)
  };
  let user_config_definition = UserConfigDefinition {
    protocols: Some(user_protos.clone()),
    user_device_configs: Some(user_definitions_vec),
    simulated_devices,
  };
  let config_version = get_internal_config_version();
  let mut user_config_file = UserConfigFile::new(config_version.major, config_version.minor);
  user_config_file.set_user_configs(Some(user_config_definition));
  serde_json::to_string_pretty(&user_config_file).map_err(|e| {
    ButtplugError::from(ButtplugDeviceError::DeviceConfigurationError(format!(
      "Cannot save device configuration file: {e:?}",
    )))
  })
}

#[cfg(test)]
mod test {
  use crate::{
    ProtocolCommunicationSpecifier,
    SimulatedDeviceConfigEntry,
    UserDeviceIdentifier,
    WebsocketSpecifier,
    device_config_file::{load_main_config, load_protocol_configs, save_user_config},
  };

  use super::{
    DEVICE_CONFIGURATION_JSON,
    DEVICE_CONFIGURATION_JSON_SCHEMA,
    USER_DEVICE_CONFIGURATION_JSON_SCHEMA,
    base::BaseConfigFile,
    load_protocol_config_from_json,
    strict_schema,
    undocumented_keys,
  };
  use serde_json::json;
  use std::collections::BTreeSet;

  fn undocumented_base_keys(config: &serde_json::Value) -> BTreeSet<String> {
    undocumented_keys(DEVICE_CONFIGURATION_JSON_SCHEMA.schema, &config.to_string())
  }

  fn user_config_with_schema_key(schema_key: serde_json::Value) -> String {
    json!({
      "$schema": schema_key,
      "version": { "major": 5, "minor": 0 },
      "user_configs": {}
    })
    .to_string()
  }

  #[test]
  fn test_config_file_parsing() {
    load_protocol_config_from_json::<BaseConfigFile>(
      DEVICE_CONFIGURATION_JSON,
      &DEVICE_CONFIGURATION_JSON_SCHEMA,
      true,
    )
    .unwrap();
  }

  #[test]
  fn test_main_file_parsing() {
    load_main_config(&None, false).unwrap();
  }

  #[test]
  fn test_unknown_communication_specifier_is_ignored_on_config_load() {
    let mut config: serde_json::Value = serde_json::from_str(DEVICE_CONFIGURATION_JSON).unwrap();
    config["protocols"]["future-protocol"] = json!({
      "communication": [
        {
          "future_connector": {
            "some": "value"
          }
        }
      ]
    });

    load_protocol_config_from_json::<BaseConfigFile>(
      &config.to_string(),
      &DEVICE_CONFIGURATION_JSON_SCHEMA,
      true,
    )
    .unwrap();
  }

  #[test]
  fn test_base_config_accepts_schema_key() {
    let mut config: serde_json::Value = serde_json::from_str(DEVICE_CONFIGURATION_JSON).unwrap();
    config["$schema"] = json!("./buttplug-device-config-schema-v5.json");
    load_main_config(&Some(config.to_string()), false).unwrap();
  }

  #[test]
  fn test_base_config_is_validated_against_base_schema() {
    // serde ignores `$schema`, so only schema validation can reject a non-string value.
    let mut config: serde_json::Value = serde_json::from_str(DEVICE_CONFIGURATION_JSON).unwrap();
    config["$schema"] = json!(5);
    assert!(load_main_config(&Some(config.to_string()), false).is_err());
  }

  #[test]
  fn test_user_config_accepts_schema_key() {
    let user_config =
      user_config_with_schema_key(json!("./buttplug-user-device-config-schema-v5.json"));
    load_protocol_configs(&None, &Some(user_config), false)
      .unwrap()
      .finish()
      .unwrap();
  }

  #[test]
  fn test_user_config_is_validated_against_user_schema() {
    // serde ignores `$schema`, so only schema validation can reject a non-string value.
    let user_config = user_config_with_schema_key(json!(5));
    assert!(load_protocol_configs(&None, &Some(user_config), false).is_err());
  }

  #[test]
  fn test_strict_schema_closes_only_unset_objects() {
    let schema = json!({
      "type": "object",
      "properties": { "closed": { "type": "object", "properties": {} } },
      "$defs": {
        "open": { "type": "object", "properties": {}, "additionalProperties": true },
        "map": { "type": "object", "additionalProperties": { "type": "string" } }
      }
    });
    let strict = strict_schema(&schema);
    assert_eq!(strict["additionalProperties"], json!(false));
    assert_eq!(
      strict["properties"]["closed"]["additionalProperties"],
      json!(false)
    );
    assert_eq!(strict["$defs"]["open"]["additionalProperties"], json!(true));
    assert_eq!(
      strict["$defs"]["map"]["additionalProperties"],
      json!({ "type": "string" })
    );
  }

  #[test]
  fn test_internal_config_has_no_undocumented_keys() {
    let config: serde_json::Value = serde_json::from_str(DEVICE_CONFIGURATION_JSON).unwrap();
    assert_eq!(undocumented_base_keys(&config), BTreeSet::new());
  }

  #[test]
  fn test_undocumented_keys_are_found() {
    let mut config: serde_json::Value = serde_json::from_str(DEVICE_CONFIGURATION_JSON).unwrap();
    config["top_level"] = json!(1);
    config["protocols"]["lovense"]["defaults"]["features"][0]["output"]["vibrate"]["a/b"] =
      json!(1);
    config["protocols"]["lovense"]["communication"][0]["btle"]["extra"] = json!(1);
    assert_eq!(
      undocumented_base_keys(&config),
      BTreeSet::from([
        "/protocols/lovense/communication/0/btle/extra".to_owned(),
        "/protocols/lovense/defaults/features/0/output/vibrate/a~1b".to_owned(),
        "/top_level".to_owned(),
      ])
    );
    // Undocumented keys are only reported, the config still loads.
    load_main_config(&Some(config.to_string()), false).unwrap();
  }

  #[test]
  fn test_documented_keys_are_not_reported() {
    let mut config: serde_json::Value = serde_json::from_str(DEVICE_CONFIGURATION_JSON).unwrap();
    // Declared in the schema, but unused by the loader.
    config["$schema"] = json!("./buttplug-device-config-schema-v5.json");
    config["protocols"]["lovense"]["defaults"]["features"][0]["output"]["vibrate"]["description"] =
      json!("unused");
    // Unknown connector types are open on purpose, and warned about separately.
    config["protocols"]["lovense"]["communication"]
      .as_array_mut()
      .unwrap()
      .push(json!({ "future_connector": { "some": "value" } }));
    assert_eq!(undocumented_base_keys(&config), BTreeSet::new());
  }

  #[test]
  fn test_undocumented_user_config_keys_are_found() {
    let user_config = json!({
      "version": { "major": 5, "minor": 0 },
      "user_configs": {
        "simulated_devices": [{ "identifier": "simulated-1vibe", "extra": 1 }]
      }
    });
    assert_eq!(
      undocumented_keys(
        USER_DEVICE_CONFIGURATION_JSON_SCHEMA.schema,
        &user_config.to_string()
      ),
      BTreeSet::from(["/user_configs/simulated_devices/0/extra".to_owned()])
    );
  }

  #[test]
  fn test_saved_user_config_reloads() {
    let dcm = load_protocol_configs(&None, &None, false)
      .unwrap()
      .finish()
      .unwrap();
    let simulated =
      SimulatedDeviceConfigEntry::new("simulated-stroker", Some("Stroker".to_owned()));
    let identifier = UserDeviceIdentifier::new(
      simulated.address(),
      "simulated",
      &Some(simulated.identifier().clone()),
    );
    dcm.add_simulated_device(simulated).unwrap();
    dcm
      .add_user_communication_specifier(
        "lovense",
        &ProtocolCommunicationSpecifier::Websocket(WebsocketSpecifier::new("LVSDevice")),
      )
      .unwrap();
    dcm
      .device_definition(&identifier)
      .expect("Simulated stroker should resolve a definition");

    let saved = save_user_config(&dcm).unwrap();
    assert_eq!(
      undocumented_keys(USER_DEVICE_CONFIGURATION_JSON_SCHEMA.schema, &saved),
      BTreeSet::new()
    );
    let reloaded = load_protocol_configs(&None, &Some(saved), false)
      .unwrap()
      .finish()
      .unwrap();
    assert_eq!(reloaded.user_device_definitions().len(), 1);
    assert_eq!(reloaded.simulated_devices().len(), 1);
    assert_eq!(reloaded.user_communication_specifiers().len(), 1);
  }
}
