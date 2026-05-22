// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;

use super::device::ConfigBaseDeviceDefinition;

use crate::ProtocolCommunicationSpecifier;

const KNOWN_COMMUNICATION_SPECIFIERS: &[&str] = &[
  "btle",
  "hid",
  "usb",
  "serial",
  "xinput",
  "lovense_connect_service",
  "websocket",
  "simulated",
];

fn deserialize_communication_specifiers<'de, D>(
  deserializer: D,
) -> Result<Option<Vec<ProtocolCommunicationSpecifier>>, D::Error>
where
  D: Deserializer<'de>,
{
  let Some(entries) = Option::<Vec<Value>>::deserialize(deserializer)? else {
    return Ok(None);
  };

  let mut specifiers = Vec::with_capacity(entries.len());
  for entry in entries {
    let object = entry
      .as_object()
      .ok_or_else(|| de::Error::custom("communication specifier must be an object"))?;
    if object.len() != 1 {
      return Err(de::Error::custom(
        "communication specifier must contain exactly one connector type",
      ));
    }

    let connector_type = object
      .keys()
      .next()
      .expect("already checked object has exactly one key");
    if !KNOWN_COMMUNICATION_SPECIFIERS.contains(&connector_type.as_str()) {
      warn!(
        "Ignoring unknown communication specifier type '{}'",
        connector_type
      );
      continue;
    }

    specifiers.push(serde_json::from_value(entry).map_err(de::Error::custom)?);
  }

  Ok((!specifiers.is_empty()).then_some(specifiers))
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub(super) struct ProtocolDefinition {
  #[serde(
    default,
    deserialize_with = "deserialize_communication_specifiers",
    skip_serializing_if = "Option::is_none"
  )]
  pub communication: Option<Vec<ProtocolCommunicationSpecifier>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub defaults: Option<ConfigBaseDeviceDefinition>,
  #[serde(default)]
  pub configurations: Vec<ConfigBaseDeviceDefinition>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_unknown_communication_specifier_is_ignored() {
    let protocol: ProtocolDefinition = serde_json::from_value(json!({
      "communication": [
        {
          "future_connector": {
            "some": "value"
          }
        },
        {
          "websocket": {
            "name": "test"
          }
        }
      ]
    }))
    .unwrap();

    let specifiers = protocol.communication().as_ref().unwrap();
    assert_eq!(specifiers.len(), 1);
    assert!(matches!(
      specifiers[0],
      ProtocolCommunicationSpecifier::Websocket(_)
    ));
  }

  #[test]
  fn test_malformed_known_communication_specifier_errors() {
    let result = serde_json::from_value::<ProtocolDefinition>(json!({
      "communication": [
        {
          "websocket": {
            "unexpected": "field"
          }
        }
      ]
    }));

    assert!(result.is_err());
  }
}
