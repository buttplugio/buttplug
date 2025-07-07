// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::ButtplugSerializerError;
use crate::message::{ButtplugMessage, ButtplugMessageFinalizer};
use jsonschema::Validator;
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Value};
use std::fmt::Debug;

static MESSAGE_JSON_SCHEMA: &str =
  include_str!("../../../schema/buttplug-schema.json");

/// Creates a [jsonschema::JSONSchema] validator using the built in buttplug message schema.
pub fn create_message_validator() -> Validator {
  let schema: serde_json::Value =
    serde_json::from_str(MESSAGE_JSON_SCHEMA).expect("Built in schema better be valid");
  Validator::new(&schema).expect("Built in schema better be valid")
}

/// Returns the message as a string in Buttplug JSON Protocol format.
pub fn msg_to_protocol_json<T>(msg: T) -> String
where
  T: ButtplugMessage + Serialize + Deserialize<'static>,
{
  serde_json::to_string(&[&msg]).expect("Infallible serialization")
}

pub fn vec_to_protocol_json<T>(msg: &[T]) -> String
where
  T: ButtplugMessage + Serialize + Deserialize<'static>,
{
  serde_json::to_string(msg).expect("Infallible serialization")
}

pub fn deserialize_to_message<T>(
  validator: Option<&Validator>,
  msg_str: &str,
) -> Result<Vec<T>, ButtplugSerializerError>
where
  T: serde::de::DeserializeOwned + ButtplugMessageFinalizer + Clone + Debug,
{
  // TODO This assumes that we've gotten a full JSON string to deserialize, which may not be the
  // case.
  let stream = Deserializer::from_str(msg_str).into_iter::<Value>();

  let mut result = vec![];

  for msg in stream {
    match msg {
      Ok(json_msg) => {
        if let Some(validator) = validator {
          if !validator.is_valid(&json_msg) {
            // If is_valid fails, re-run validation to get our error message.
            let e = validator
              .validate(&json_msg)
              .expect_err("We can't get here without validity checks failing.");
            return Err(ButtplugSerializerError::JsonSerializerError(format!(
              "Error during JSON Schema Validation - Message: {json_msg} - Error: {e:?}"
            )));
          }
        }
        match serde_json::from_value::<Vec<T>>(json_msg) {
          Ok(mut msg_vec) => {
            for msg in msg_vec.iter_mut() {
              msg.finalize();
            }
            result.append(&mut msg_vec);
          }
          Err(e) => {
            return Err(ButtplugSerializerError::JsonSerializerError(format!(
              "Message: {msg_str} - Error: {e:?}"
            )))
          }
        }
      }
      Err(e) => {
        return Err(ButtplugSerializerError::JsonSerializerError(format!(
          "Message: {msg_str} - Error: {e:?}"
        )))
      }
    }
  }
  Ok(result)
}
