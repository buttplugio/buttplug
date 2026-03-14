// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::{fmt, ops::{Deref, DerefMut}};

use crate::message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageValidator,
  InputType,
};
use getset::CopyGetters;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::{SeqAccess, Visitor}, ser::SerializeSeq};
use enumflags2::{BitFlags, bitflags};
use strum_macros::Display;

#[bitflags]
#[repr(u8)]
#[derive(Debug, Display, PartialEq, Eq, Clone, Serialize, Deserialize, Hash, Copy)]
pub enum InputCommandType {
  #[serde(alias = "read")]
  Read,
  #[serde(alias = "subscribe")]
  Subscribe,
  #[serde(alias = "unsubscribe")]
  Unsubscribe,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, CopyGetters, Serialize, Deserialize)]
pub struct InputCmdV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[getset(get_copy = "pub")]
  #[serde(rename = "FeatureIndex")]
  feature_index: u32,
  #[getset(get_copy = "pub")]
  #[serde(rename = "Type")]
  input_type: InputType,
  #[getset(get_copy = "pub")]
  #[serde(rename = "Command")]
  input_command: InputCommandType,
}

impl InputCmdV4 {
  pub fn new(
    device_index: u32,
    feature_index: u32,
    input_type: InputType,
    input_command_type: InputCommandType,
  ) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      input_type,
      input_command: input_command_type,
    }
  }
}

impl ButtplugMessage for InputCmdV4 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugDeviceMessage for InputCmdV4 {
  fn device_index(&self) -> u32 {
    self.device_index
  }
  fn set_device_index(&mut self, device_index: u32) {
    self.device_index = device_index;
  }
}

impl ButtplugMessageValidator for InputCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Hash)]
pub struct InputCommandTypeFlags(BitFlags<InputCommandType>);

impl InputCommandTypeFlags {
  pub fn new(flags: BitFlags<InputCommandType>) -> Self {
    Self(flags)
  }

  pub fn empty() -> Self {
    Self(BitFlags::empty())
  }
}

impl Deref for InputCommandTypeFlags {
  type Target = BitFlags<InputCommandType>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for InputCommandTypeFlags {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl From<BitFlags<InputCommandType>> for InputCommandTypeFlags {
  fn from(flags: BitFlags<InputCommandType>) -> Self {
    Self(flags)
  }
}

impl From<InputCommandTypeFlags> for BitFlags<InputCommandType> {
  fn from(flags: InputCommandTypeFlags) -> Self {
    flags.0
  }
}

impl Serialize for InputCommandTypeFlags {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
    for cmd in self.0.iter() {
      seq.serialize_element(&cmd)?;
    }
    seq.end()
  }
}

impl<'de> Deserialize<'de> for InputCommandTypeFlags {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct FlagsVisitor;

    impl<'de> Visitor<'de> for FlagsVisitor {
      type Value = InputCommandTypeFlags;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an array of InputCommandType values")
      }

      fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
      where
        A: SeqAccess<'de>,
      {
        let mut flags = BitFlags::empty();
        while let Some(cmd) = seq.next_element::<InputCommandType>()? {
          flags |= cmd;
        }
        Ok(InputCommandTypeFlags(flags))
      }
    }

    deserializer.deserialize_seq(FlagsVisitor)
  }
}
