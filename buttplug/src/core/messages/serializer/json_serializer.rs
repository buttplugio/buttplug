// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugMessageSerializer, ButtplugSerializedMessage, ButtplugSerializerError};
use crate::core::{
  errors::{ButtplugError, ButtplugHandshakeError},
  messages::{
    self,
    ButtplugClientMessage,
    ButtplugCurrentSpecClientMessage,
    ButtplugCurrentSpecServerMessage,
    ButtplugMessage,
    ButtplugMessageSpecVersion,
    ButtplugServerMessage,
    ButtplugSpecV0ClientMessage,
    ButtplugSpecV0ServerMessage,
    ButtplugSpecV1ClientMessage,
    ButtplugSpecV1ServerMessage,
    ButtplugSpecV2ClientMessage,
    ButtplugSpecV2ServerMessage,
    ButtplugSpecV3ClientMessage,
    ButtplugSpecV3ServerMessage,
  },
};
use jsonschema::JSONSchema;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

static MESSAGE_JSON_SCHEMA: &str =
  include_str!("../../../../buttplug-schema/schema/buttplug-schema.json");

/// Creates a [jsonschema::JSONSchema] validator using the built in buttplug message schema.
pub fn create_message_validator() -> JSONSchema {
  let schema: serde_json::Value =
    serde_json::from_str(MESSAGE_JSON_SCHEMA).expect("Built in schema better be valid");
  JSONSchema::compile(&schema).expect("Built in schema better be valid")
}
pub struct ButtplugServerJSONSerializer {
  pub(super) message_version: OnceCell<messages::ButtplugMessageSpecVersion>,
  validator: JSONSchema,
}

impl Default for ButtplugServerJSONSerializer {
  fn default() -> Self {
    Self {
      message_version: OnceCell::new(),
      validator: create_message_validator(),
    }
  }
}

/// Returns the message as a string in Buttplug JSON Protocol format.
pub fn msg_to_protocol_json<T>(msg: T) -> String
where
  T: ButtplugMessage + Serialize + Deserialize<'static>,
{
  serde_json::to_string(&[&msg]).expect("Infallible serialization")
}

pub fn vec_to_protocol_json<T>(msg: Vec<T>) -> String
where
  T: ButtplugMessage + Serialize + Deserialize<'static>,
{
  serde_json::to_string(&msg).expect("Infallible serialization")
}

fn deserialize_to_message<T>(
  validator: &JSONSchema,
  msg: String,
) -> Result<Vec<T>, ButtplugSerializerError>
where
  T: serde::de::DeserializeOwned + Clone,
{
  // We have to pass back a string formatted error, as SerdeJson's error type
  // isn't clonable.
  serde_json::from_str::<serde_json::Value>(&msg)
    .map_err(|e| {
      ButtplugSerializerError::JsonSerializerError(format!("Message: {} - Error: {:?}", msg, e))
    })
    .and_then(|json_msg| match validator.validate(&json_msg) {
      Ok(_) => serde_json::from_value(json_msg.clone()).map_err(|e| {
        ButtplugSerializerError::JsonSerializerError(format!("Message: {} - Error: {:?}", msg, e))
      }),
      Err(e) => {
        let err_vec: Vec<jsonschema::ValidationError> = e.collect();
        Err(ButtplugSerializerError::JsonSerializerError(format!(
          "Error during JSON Schema Validation - Message: {} - Error: {:?}",
          json_msg,
          err_vec
        )))
      }
    })
}

fn serialize_to_version(
  version: ButtplugMessageSpecVersion,
  msgs: Vec<ButtplugServerMessage>,
) -> ButtplugSerializedMessage {
  ButtplugSerializedMessage::Text(match version {
    ButtplugMessageSpecVersion::Version0 => {
      let msg_vec: Vec<ButtplugSpecV0ServerMessage> = msgs
        .iter()
        .cloned()
        .map(|msg| match ButtplugSpecV0ServerMessage::try_from(msg) {
          Ok(msgv0) => msgv0,
          Err(err) => ButtplugSpecV0ServerMessage::Error(
            messages::Error::from(ButtplugError::from(err)).into(),
          ),
        })
        .collect();
      vec_to_protocol_json(msg_vec)
    }
    ButtplugMessageSpecVersion::Version1 => {
      let msg_vec: Vec<ButtplugSpecV1ServerMessage> = msgs
        .iter()
        .cloned()
        .map(|msg| match ButtplugSpecV1ServerMessage::try_from(msg) {
          Ok(msgv0) => msgv0,
          Err(err) => ButtplugSpecV1ServerMessage::Error(
            messages::Error::from(ButtplugError::from(err)).into(),
          ),
        })
        .collect();
      vec_to_protocol_json(msg_vec)
    }
    ButtplugMessageSpecVersion::Version2 => {
      let msg_vec: Vec<ButtplugSpecV2ServerMessage> = msgs
        .iter()
        .cloned()
        .map(|msg| match ButtplugSpecV2ServerMessage::try_from(msg) {
          Ok(msgv0) => msgv0,
          Err(err) => ButtplugSpecV2ServerMessage::Error(ButtplugError::from(err).into()),
        })
        .collect();
      vec_to_protocol_json(msg_vec)
    }
    ButtplugMessageSpecVersion::Version3 => {
      let msg_vec: Vec<ButtplugSpecV3ServerMessage> = msgs
        .iter()
        .cloned()
        .map(|msg| match ButtplugSpecV3ServerMessage::try_from(msg) {
          Ok(msgv0) => msgv0,
          Err(err) => ButtplugSpecV3ServerMessage::Error(ButtplugError::from(err).into()),
        })
        .collect();
      vec_to_protocol_json(msg_vec)
    }
  })
}

impl ButtplugMessageSerializer for ButtplugServerJSONSerializer {
  type Inbound = ButtplugClientMessage;
  type Outbound = ButtplugServerMessage;

  fn deserialize(
    &self,
    serialized_msg: ButtplugSerializedMessage,
  ) -> Result<Vec<ButtplugClientMessage>, ButtplugSerializerError> {
    let msg = if let ButtplugSerializedMessage::Text(text_msg) = serialized_msg {
      text_msg
    } else {
      return Err(ButtplugSerializerError::BinaryDeserializationError);
    };
    // If we don't have a message version yet, we need to parse this as a
    // RequestServerInfo message to get the version. RequestServerInfo can
    // always be parsed as the latest message version, as we keep it
    // compatible across versions via serde options.
    if let Some(version) = self.message_version.get() {
      return Ok(match version {
        ButtplugMessageSpecVersion::Version0 => {
          deserialize_to_message::<ButtplugSpecV0ClientMessage>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version1 => {
          deserialize_to_message::<ButtplugSpecV1ClientMessage>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version2 => {
          deserialize_to_message::<ButtplugSpecV2ClientMessage>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version3 => {
          deserialize_to_message::<ButtplugSpecV3ClientMessage>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
      });
    }
    // instead of using if/else here, return in the if, which drops the borrow.
    // so we can possibly mutate it now.
    let msg_union = deserialize_to_message::<ButtplugSpecV2ClientMessage>(&self.validator, msg)?;
    // If the message is malformed, just return an spec version not received error.
    if msg_union.is_empty() {
      return Err(ButtplugSerializerError::MessageSpecVersionNotReceived);
    }
    if let ButtplugSpecV2ClientMessage::RequestServerInfo(rsi) = &msg_union[0] {
      info!(
        "Setting JSON Wrapper message version to {}",
        rsi.message_version()
      );
      self
        .message_version
        .set(rsi.message_version())
        .expect("This should only ever be called once.");
    } else {
      return Err(ButtplugSerializerError::MessageSpecVersionNotReceived);
    }
    Ok(msg_union.iter().cloned().map(|m| m.into()).collect())
  }

  fn serialize(&self, msgs: Vec<ButtplugServerMessage>) -> ButtplugSerializedMessage {
    if let Some(version) = self.message_version.get() {
      serialize_to_version(*version, msgs)
    } else {
      // In the rare event that there is a problem with the
      // RequestServerInfo message (so we can't set up our known spec
      // version), just encode to the latest and return.
      if let ButtplugServerMessage::Error(_) = &msgs[0] {
        serialize_to_version(ButtplugMessageSpecVersion::Version2, msgs)
      } else {
        // If we don't even have enough info to know which message
        // version to convert to, consider this a handshake error.
        ButtplugSerializedMessage::Text(msg_to_protocol_json(
          ButtplugCurrentSpecServerMessage::Error(
            ButtplugError::from(ButtplugHandshakeError::RequestServerInfoExpected).into(),
          ),
        ))
      }
    }
  }
}

pub struct ButtplugClientJSONSerializer {
  validator: JSONSchema,
}

impl Default for ButtplugClientJSONSerializer {
  fn default() -> Self {
    Self {
      validator: create_message_validator(),
    }
  }
}

impl ButtplugMessageSerializer for ButtplugClientJSONSerializer {
  type Inbound = ButtplugCurrentSpecServerMessage;
  type Outbound = ButtplugCurrentSpecClientMessage;

  fn deserialize(
    &self,
    msg: ButtplugSerializedMessage,
  ) -> Result<Vec<ButtplugCurrentSpecServerMessage>, ButtplugSerializerError> {
    if let ButtplugSerializedMessage::Text(text_msg) = msg {
      deserialize_to_message::<Self::Inbound>(&self.validator, text_msg)
    } else {
      Err(ButtplugSerializerError::BinaryDeserializationError)
    }
  }

  fn serialize(&self, msg: Vec<ButtplugCurrentSpecClientMessage>) -> ButtplugSerializedMessage {
    ButtplugSerializedMessage::Text(vec_to_protocol_json(msg))
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::core::messages::{RequestServerInfo, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION};

  #[test]
  fn test_correct_message_version() {
    let json = r#"[{
            "RequestServerInfo": {
                "Id": 1,
                "ClientName": "Test Client",
                "MessageVersion": 2
            }
        }]"#;
    let serializer = ButtplugServerJSONSerializer::default();
    serializer
      .deserialize(ButtplugSerializedMessage::Text(json.to_owned()))
      .expect("Infallible deserialization");
    assert_eq!(
      *serializer.message_version.get().unwrap(),
      ButtplugMessageSpecVersion::Version2
    );
  }

  #[test]
  fn test_wrong_message_version() {
    let json = r#"[{
            "RequestServerInfo": {
                "Id": 1,
                "ClientName": "Test Client",
                "MessageVersion": 100
            }
        }]"#;
    let serializer = ButtplugServerJSONSerializer::default();
    let msg = serializer.deserialize(ButtplugSerializedMessage::Text(json.to_owned()));
    assert!(msg.is_err());
  }

  #[test]
  fn test_client_incorrect_messages() {
    let incorrect_incoming_messages = vec![
      // Not valid JSON
      "not a json message",
      // Valid json object but no contents
      "{}",
      // Valid json but not an object
      "[]",
      // Not a message type
      "[{\"NotAMessage\":{}}]",
      // Valid json and message type but not in correct format
      "[{\"Ok\":[]}]",
      // Valid json and message type but not in correct format
      "[{\"Ok\":{}}]",
      // Valid json and message type but not an array.
      "{\"Ok\":{\"Id\":0}}",
      // Valid json and message type but not an array.
      "[{\"Ok\":{\"Id\":0}}]",
      // Valid json and message type but with extra content
      "[{\"Ok\":{\"NotAField\":\"NotAValue\",\"Id\":1}}]",
    ];
    let serializer = ButtplugClientJSONSerializer::default();
    let _ = serializer.serialize(vec![RequestServerInfo::new(
      "test client",
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    )
    .into()]);
    for msg in incorrect_incoming_messages {
      let res = serializer.deserialize(ButtplugSerializedMessage::Text(msg.to_owned()));
      assert!(res.is_err(), "{} should be an error", msg);
      if let Err(ButtplugSerializerError::MessageSpecVersionNotReceived) = res {
        assert!(false, "Wrong error!");
      }
    }
  }
}
