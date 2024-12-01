// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Message Spec Conversion
//!
//! This module contains code to convert any message from an older spec version up to the current
//! message spec, and then convert any response from the current message spec back down the sending
//! spec. This is handled within the server, as the server is the only portion of Buttplug that
//! needs to handle up/downgrading (the client should never have to care and should only ever talk
//! one version of the spec, preferably the latest). Having this done within the server also allows
//! us to access required state for converting between messages that requires knowledge of ephemeral
//! device structures (i.e. converting from v4 device features to <= v3 message attributes for
//! messages like DeviceAdded).

use crate::core::{
  errors::{ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageSpecVersion,
    ButtplugServerMessageV4,
  },
};

use super::message::{
  BatteryLevelReadingV2,
  ButtplugClientMessageV2,
  ButtplugClientMessageV3,
  ButtplugClientMessageVariant,
  ButtplugServerMessageV0,
  ButtplugServerMessageV1,
  ButtplugServerMessageV2,
  ButtplugServerMessageV3,
  ButtplugServerMessageVariant,
  RSSILevelReadingV2,
  SensorReadingV3,
};

pub struct ButtplugServerMessageConverter {
  original_message: Option<ButtplugClientMessageVariant>,
}

impl ButtplugServerMessageConverter {
  pub fn new(msg: Option<ButtplugClientMessageVariant>) -> Self {
    Self {
      original_message: msg,
    }
  }

  //
  // Outgoing Conversion
  //

  pub fn convert_outgoing(
    &self,
    msg: &ButtplugServerMessageV4,
    version: &ButtplugMessageSpecVersion,
  ) -> Result<ButtplugServerMessageVariant, ButtplugError> {
    let mut outgoing_msg = match version {
      ButtplugMessageSpecVersion::Version0 => {
        ButtplugServerMessageVariant::V0(self.convert_servermessagev4_to_servermessagev0(msg)?)
      }
      ButtplugMessageSpecVersion::Version1 => {
        ButtplugServerMessageVariant::V1(self.convert_servermessagev4_to_servermessagev1(msg)?)
      }
      ButtplugMessageSpecVersion::Version2 => {
        ButtplugServerMessageVariant::V2(self.convert_servermessagev4_to_servermessagev2(msg)?)
      }
      ButtplugMessageSpecVersion::Version3 => {
        ButtplugServerMessageVariant::V3(self.convert_servermessagev4_to_servermessagev3(msg)?)
      }
      ButtplugMessageSpecVersion::Version4 => ButtplugServerMessageVariant::V4(msg.clone()),
    };
    // Always make sure the ID is set after conversion
    outgoing_msg.set_id(msg.id());
    Ok(outgoing_msg)
  }

  fn convert_servermessagev4_to_servermessagev3(
    &self,
    msg: &ButtplugServerMessageV4,
  ) -> Result<ButtplugServerMessageV3, ButtplugError> {
    match msg {
      ButtplugServerMessageV4::SensorReading(m) => {
        let original_msg = self.original_message.as_ref().unwrap();
        if let ButtplugClientMessageVariant::V3(ButtplugClientMessageV3::SensorReadCmd(msg)) =
          &original_msg
        {
          let msg_out = SensorReadingV3::new(
            msg.device_index(),
            *msg.sensor_index(),
            *msg.sensor_type(),
            m.data().clone(),
          );
          Ok(msg_out.into())
        } else {
          Err(ButtplugMessageError::UnexpectedMessageType("SensorReading".to_owned()).into())
        }
      }
      _ => Ok(msg.clone().try_into()?),
    }
  }

  fn convert_servermessagev4_to_servermessagev2(
    &self,
    msg: &ButtplugServerMessageV4,
  ) -> Result<ButtplugServerMessageV2, ButtplugError> {
    let msg_v3 = self.convert_servermessagev4_to_servermessagev3(msg)?;
    match msg_v3 {
      ButtplugServerMessageV3::SensorReading(m) => {
        let original_msg = self.original_message.as_ref().unwrap();
        // Sensor Reading didn't exist in v2, we only had Battery or RSSI. Therefore we need to
        // context of the original message to make sure this conversion happens correctly.
        if let ButtplugClientMessageVariant::V2(ButtplugClientMessageV2::BatteryLevelCmd(msg)) =
          &original_msg
        {
          Ok(BatteryLevelReadingV2::new(msg.device_index(), m.data()[0] as f64 / 100f64).into())
        } else if let ButtplugClientMessageVariant::V2(ButtplugClientMessageV2::RSSILevelCmd(msg)) =
          &original_msg
        {
          Ok(RSSILevelReadingV2::new(msg.device_index(), m.data()[0]).into())
        } else {
          Err(ButtplugMessageError::UnexpectedMessageType("SensorReading".to_owned()).into())
        }
      }
      _ => Ok(msg_v3.into()),
    }
  }

  fn convert_servermessagev4_to_servermessagev1(
    &self,
    msg: &ButtplugServerMessageV4,
  ) -> Result<ButtplugServerMessageV1, ButtplugError> {
    Ok(self.convert_servermessagev4_to_servermessagev2(msg)?.into())
  }

  fn convert_servermessagev4_to_servermessagev0(
    &self,
    msg: &ButtplugServerMessageV4,
  ) -> Result<ButtplugServerMessageV0, ButtplugError> {
    Ok(self.convert_servermessagev4_to_servermessagev1(msg)?.into())
  }

  // Outgoing Conversion Utility Methods
}
