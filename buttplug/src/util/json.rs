// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! JSON Schema validator structure, used by the
//! [DeviceConfigurationManager][crate::device::configuration_manager::DeviceConfigurationManager] and
//! buttplug message de/serializers in both the client and server. Uses the
//! Valico library.

use crate::core::messages::serializer::ButtplugSerializerError;
use serde_json::Value;
use valico::json_schema;

pub struct JSONValidator {
  /// Valico's scope object, used for holding the schema.
  scope: json_schema::scope::Scope,
  /// Valico's id URL, used for accessing the schema.
  id: url::Url,
}

impl JSONValidator {
  /// Create a new validator.
  ///
  /// # Parameters
  ///
  /// - `schema`: JSON Schema that the validator should use.
  pub fn new(schema: &str) -> Self {
    let schema_json: Value =
      serde_json::from_str(schema).expect("If this fails, the library is going with it.");
    let mut scope = json_schema::Scope::new();
    let id = scope
      .compile(schema_json, false)
      .expect("If this fails, the library is going with it.");
    Self { id, scope }
  }

  /// Validates a json string, based on the schema the validator was created
  /// with.
  ///
  /// # Parameters
  ///
  /// - `json_str`: JSON string to validate.
  pub fn validate(&self, json_str: &str) -> Result<(), ButtplugSerializerError> {
    let schema = self
      .scope
      .resolve(&self.id)
      .expect("id generated on creation.");
    let check_value = serde_json::from_str(json_str).map_err(|err| {
      ButtplugSerializerError::JsonSerializerError(format!(
        "Message: {} - Error: {:?}",
        json_str, err
      ))
    })?;
    let state = schema.validate(&check_value);
    if state.is_valid() {
      Ok(())
    } else {
      // Our errors need to be clonable, and validation state isn't. We can't do
      // much with it anyways, so just convert it to its display and hand that
      // back.
      Err(ButtplugSerializerError::JsonValidatorError(format!(
        "Message: {} - Error: {:?}",
        json_str, state
      )))
    }
  }
}
