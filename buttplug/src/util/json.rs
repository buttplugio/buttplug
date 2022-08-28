// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! JSON Schema validator structure, used by the
//! [DeviceConfigurationManager][crate::server::device::configuration::DeviceConfigurationManager] and
//! buttplug message de/serializers in both the client and server. Uses the
//! jsonschema library.

use crate::core::message::serializer::ButtplugSerializerError;
use jsonschema::JSONSchema;

pub struct JSONValidator {
  schema: JSONSchema,
}

impl JSONValidator {
  /// Create a new validator.
  ///
  /// # Parameters
  ///
  /// - `schema`: JSON Schema that the validator should use.
  pub fn new(schema: &str) -> Self {
    let schema_json: serde_json::Value =
      serde_json::from_str(schema).expect("Built in schema better be valid");
    let schema = JSONSchema::compile(&schema_json).expect("Built in schema better be valid");
    Self { schema }
  }

  /// Validates a json string, based on the schema the validator was created
  /// with.
  ///
  /// # Parameters
  ///
  /// - `json_str`: JSON string to validate.
  pub fn validate(&self, json_str: &str) -> Result<(), ButtplugSerializerError> {
    let check_value = serde_json::from_str(json_str).map_err(|err| {
      ButtplugSerializerError::JsonSerializerError(format!(
        "Message: {} - Error: {:?}",
        json_str, err
      ))
    })?;
    self.schema.validate(&check_value).map_err(|err| {
      let err_vec: Vec<jsonschema::ValidationError> = err.collect();
      ButtplugSerializerError::JsonSerializerError(format!(
        "Error during JSON Schema Validation: {:?}",
        err_vec
      ))
    })
  }
}
