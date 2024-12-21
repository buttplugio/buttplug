// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugMessageSerializer, ButtplugSerializedMessage, ButtplugSerializerError};
use crate::core::{
  errors::{ButtplugError, ButtplugHandshakeError, ButtplugMessageError},
  message::{
    self,
    ButtplugClientMessageCurrent,
    ButtplugClientMessageV0,
    ButtplugClientMessageV1,
    ButtplugClientMessageV2,
    ButtplugClientMessageV3,
    ButtplugClientMessageV4,
    ButtplugClientMessageVariant,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageSpecVersion,
    ButtplugServerMessageCurrent,
    ButtplugServerMessageV0,
    ButtplugServerMessageV1,
    ButtplugServerMessageV2,
    ButtplugServerMessageV3,
    ButtplugServerMessageV4,
    ButtplugServerMessageVariant,
  },
};
use jsonschema::Validator;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{Deserializer, Value};
use std::fmt::Debug;

static MESSAGE_JSON_SCHEMA: &str =
  include_str!("../../../../buttplug-schema/schema/buttplug-schema.json");

/// Creates a [jsonschema::JSONSchema] validator using the built in buttplug message schema.
pub fn create_message_validator() -> Validator {
  let schema: serde_json::Value =
    serde_json::from_str(MESSAGE_JSON_SCHEMA).expect("Built in schema better be valid");
  Validator::new(&schema).expect("Built in schema better be valid")
}
pub struct ButtplugServerJSONSerializer {
  pub(super) message_version: OnceCell<message::ButtplugMessageSpecVersion>,
  validator: Validator,
}

impl Default for ButtplugServerJSONSerializer {
  fn default() -> Self {
    Self {
      message_version: OnceCell::new(),
      validator: create_message_validator(),
    }
  }
}

impl ButtplugServerJSONSerializer {
  pub fn force_message_version(&self, version: &ButtplugMessageSpecVersion) {
    self
      .message_version
      .set(*version)
      .expect("This should only ever be called once.");
  }
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
  validator: &Validator,
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
        if validator.is_valid(&json_msg) {
          match serde_json::from_value::<Vec<T>>(json_msg) {
            Ok(mut msg_vec) => {
              for msg in msg_vec.iter_mut() {
                msg.finalize();
              }
              result.append(&mut msg_vec);
              //Ok(msg_vec)
            }
            Err(e) => {
              return Err(ButtplugSerializerError::JsonSerializerError(format!(
                "Message: {} - Error: {:?}",
                msg_str, e
              )))
            }
          }
        } else {
          // If is_valid fails, re-run validation to get our error message.
          let e = validator
            .validate(&json_msg)
            .expect_err("We can't get here without validity checks failing.");
          return Err(ButtplugSerializerError::JsonSerializerError(format!(
            "Error during JSON Schema Validation - Message: {} - Error: {:?}",
            json_msg, e
          )));
        }
      }
      Err(e) => {
        return Err(ButtplugSerializerError::JsonSerializerError(format!(
          "Message: {} - Error: {:?}",
          msg_str, e
        )))
      }
    }
  }
  Ok(result)
}

impl ButtplugMessageSerializer for ButtplugServerJSONSerializer {
  type Inbound = ButtplugClientMessageVariant;
  type Outbound = ButtplugServerMessageVariant;

  fn deserialize(
    &self,
    serialized_msg: &ButtplugSerializedMessage,
  ) -> Result<Vec<ButtplugClientMessageVariant>, ButtplugSerializerError> {
    let msg = if let ButtplugSerializedMessage::Text(text_msg) = serialized_msg {
      text_msg
    } else {
      return Err(ButtplugSerializerError::BinaryDeserializationError);
    };

    if let Some(version) = self.message_version.get() {
      return Ok(match version {
        ButtplugMessageSpecVersion::Version0 => {
          deserialize_to_message::<ButtplugClientMessageV0>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version1 => {
          deserialize_to_message::<ButtplugClientMessageV1>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version2 => {
          deserialize_to_message::<ButtplugClientMessageV2>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version3 => {
          deserialize_to_message::<ButtplugClientMessageV3>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version4 => {
          deserialize_to_message::<ButtplugClientMessageV4>(&self.validator, msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
      });
    }
    // If we don't have a message version yet, we need to parse this as a RequestServerInfo message
    // to get the version. RequestServerInfo can always be parsed as the latest message version, as
    // we keep it compatible across versions via serde options.
    let msg_union = deserialize_to_message::<ButtplugClientMessageV4>(&self.validator, msg)?;
    // If the message is malformed, just return an spec version not received error.
    if msg_union.is_empty() {
      return Err(ButtplugSerializerError::MessageSpecVersionNotReceived);
    }
    if let ButtplugClientMessageV4::RequestServerInfo(rsi) = &msg_union[0] {
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
    // Now that we know our version, parse the message again.
    self.deserialize(serialized_msg)
  }

  fn serialize(&self, msgs: &[ButtplugServerMessageVariant]) -> ButtplugSerializedMessage {
    if let Some(version) = self.message_version.get() {
      ButtplugSerializedMessage::Text(match version {
        ButtplugMessageSpecVersion::Version0 => {
          let msg_vec: Vec<ButtplugServerMessageV0> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V0(msgv0) => msgv0.clone(),
              _ => ButtplugServerMessageV0::Error(
                message::ErrorV0::from(ButtplugError::from(
                  ButtplugMessageError::MessageConversionError(format!(
                    "Message {:?} not in Spec V0! This is a server bug.",
                    msg
                  )),
                ))
                .into(),
              ),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
        ButtplugMessageSpecVersion::Version1 => {
          let msg_vec: Vec<ButtplugServerMessageV1> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V1(msgv1) => msgv1.clone(),
              _ => ButtplugServerMessageV1::Error(
                message::ErrorV0::from(ButtplugError::from(
                  ButtplugMessageError::MessageConversionError(format!(
                    "Message {:?} not in Spec V1! This is a server bug.",
                    msg
                  )),
                ))
                .into(),
              ),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
        ButtplugMessageSpecVersion::Version2 => {
          let msg_vec: Vec<ButtplugServerMessageV2> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V2(msgv2) => msgv2.clone(),
              _ => ButtplugServerMessageV2::Error(
                message::ErrorV0::from(ButtplugError::from(
                  ButtplugMessageError::MessageConversionError(format!(
                    "Message {:?} not in Spec V2! This is a server bug.",
                    msg
                  )),
                ))
                .into(),
              ),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
        ButtplugMessageSpecVersion::Version3 => {
          let msg_vec: Vec<ButtplugServerMessageV3> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V3(msgv3) => msgv3.clone(),
              _ => ButtplugServerMessageV3::Error(
                message::ErrorV0::from(ButtplugError::from(
                  ButtplugMessageError::MessageConversionError(format!(
                    "Message {:?} not in Spec V3! This is a server bug.",
                    msg
                  )),
                ))
                .into(),
              ),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
        ButtplugMessageSpecVersion::Version4 => {
          let msg_vec: Vec<ButtplugServerMessageV4> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V4(msgv4) => msgv4.clone(),
              _ => ButtplugServerMessageV4::Error(
                message::ErrorV0::from(ButtplugError::from(
                  ButtplugMessageError::MessageConversionError(format!(
                    "Message {:?} not in Spec V4! This is a server bug.",
                    msg
                  )),
                ))
                .into(),
              ),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
      })
    } else {
      // If we don't even have enough info to know which message
      // version to convert to, consider this a handshake error.
      ButtplugSerializedMessage::Text(msg_to_protocol_json(ButtplugServerMessageCurrent::Error(
        ButtplugError::from(ButtplugHandshakeError::RequestServerInfoExpected).into(),
      )))
    }
  }
}

pub struct ButtplugClientJSONSerializerImpl {
  validator: Validator,
}

impl Default for ButtplugClientJSONSerializerImpl {
  fn default() -> Self {
    Self {
      validator: create_message_validator(),
    }
  }
}

impl ButtplugClientJSONSerializerImpl {
  pub fn deserialize<T>(
    &self,
    msg: &ButtplugSerializedMessage,
  ) -> Result<Vec<T>, ButtplugSerializerError>
  where
    T: serde::de::DeserializeOwned + ButtplugMessageFinalizer + Clone + Debug,
  {
    if let ButtplugSerializedMessage::Text(text_msg) = msg {
      deserialize_to_message::<T>(&self.validator, text_msg)
    } else {
      Err(ButtplugSerializerError::BinaryDeserializationError)
    }
  }

  pub fn serialize<T>(&self, msg: &[T]) -> ButtplugSerializedMessage
  where
    T: ButtplugMessage + Serialize + Deserialize<'static>,
  {
    ButtplugSerializedMessage::Text(vec_to_protocol_json(msg))
  }
}

#[derive(Default)]
pub struct ButtplugClientJSONSerializer {
  serializer_impl: ButtplugClientJSONSerializerImpl,
}

impl ButtplugMessageSerializer for ButtplugClientJSONSerializer {
  type Inbound = ButtplugServerMessageCurrent;
  type Outbound = ButtplugClientMessageCurrent;

  fn deserialize(
    &self,
    msg: &ButtplugSerializedMessage,
  ) -> Result<Vec<Self::Inbound>, ButtplugSerializerError> {
    self.serializer_impl.deserialize(msg)
  }

  fn serialize(&self, msg: &[Self::Outbound]) -> ButtplugSerializedMessage {
    self.serializer_impl.serialize(msg)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::core::message::{RequestServerInfoV1, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION};

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
      .deserialize(&ButtplugSerializedMessage::Text(json.to_owned()))
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
    let msg = serializer.deserialize(&ButtplugSerializedMessage::Text(json.to_owned()));
    assert!(msg.is_err());
  }

  #[test]
  fn test_message_array() {
    let json = r#"[
        {
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
          }
        },
        {
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
          }
        },
        {
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
        }
    }]"#;
    let serializer = ButtplugServerJSONSerializer::default();
    let messages = serializer
      .deserialize(&ButtplugSerializedMessage::Text(json.to_owned()))
      .expect("Infallible deserialization");
    assert_eq!(messages.len(), 3);
  }

  #[test]
  fn test_streamed_message_array() {
    let json = r#"[
        {
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
          }
        }]
        [{
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
          }
        }]
        [{
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
          }
        }]
    "#;
    let serializer = ButtplugServerJSONSerializer::default();
    let messages = serializer
      .deserialize(&ButtplugSerializedMessage::Text(json.to_owned()))
      .expect("Infallible deserialization");
    assert_eq!(messages.len(), 3);
  }

  #[test]
  fn test_invalid_streamed_message_array() {
    // Missing a } in the second message.
    let json = r#"[
        {
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
          }
        }]
        [{
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
        }]
        [{
          "RequestServerInfo": {
              "Id": 1,
              "ClientName": "Test Client",
              "MessageVersion": 3
          }
        }]
    "#;
    let serializer = ButtplugServerJSONSerializer::default();
    assert!(matches!(
      serializer.deserialize(&ButtplugSerializedMessage::Text(json.to_owned())),
      Err(_)
    ));
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
    let _ = serializer.serialize(&vec![RequestServerInfoV1::new(
      "test client",
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    )
    .into()]);
    for msg in incorrect_incoming_messages {
      let res = serializer.deserialize(&ButtplugSerializedMessage::Text(msg.to_owned()));
      assert!(res.is_err(), "{} should be an error", msg);
      if let Err(ButtplugSerializerError::MessageSpecVersionNotReceived) = res {
        assert!(false, "Wrong error!");
      }
    }
  }
}
