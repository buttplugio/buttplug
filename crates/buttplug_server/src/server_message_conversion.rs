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

use buttplug_core::{
  errors::{ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageSpecVersion, ButtplugServerMessageV4, DeviceListV4, DeviceMessageInfoV4, DeviceRemovedV0, InputTypeData
  },
};

use dashmap::DashSet;

use crate::message::{DeviceAddedV0, DeviceAddedV1, DeviceAddedV2, DeviceAddedV3};

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
  SensorReadingV3,
};

pub struct ButtplugServerDeviceEventMessageConverter {
  device_indexes: DashSet<u32>
}

impl ButtplugServerDeviceEventMessageConverter {
  pub fn new(indexes: Vec<u32>) -> Self {
    let device_indexes = DashSet::new();
    indexes.iter().for_each(|x| { device_indexes.insert(*x); });
    Self {
      device_indexes
    }
  }

  // Due to the way we generate device events, we expect every new DeviceList to only have one
  // change currently.
  pub fn convert_device_list(&self, version: &ButtplugMessageSpecVersion, list: &DeviceListV4) -> ButtplugServerMessageVariant {
    let new_indexes: Vec<u32> = list.devices().iter().map(|x| *x.0).collect();
    if new_indexes.len() > self.device_indexes.len() {
      // Device Added
      let connected_devices: Vec<&DeviceMessageInfoV4> = list.devices().values().filter(|x| !self.device_indexes.contains(&x.device_index())).collect();
      self.device_indexes.insert(connected_devices[0].device_index());
      if *version == ButtplugMessageSpecVersion::Version4 {
        return ButtplugServerMessageVariant::V4(list.clone().into());
      }
      let da3 = DeviceAddedV3::from(connected_devices[0].clone());
      if *version == ButtplugMessageSpecVersion::Version3 {
        return ButtplugServerMessageVariant::V3(da3.into());
      }
      let da2 = DeviceAddedV2::from(da3);
      if *version == ButtplugMessageSpecVersion::Version2 {
        return ButtplugServerMessageVariant::V2(da2.into());
      }
      let da1 = DeviceAddedV1::from(da2);
      if *version == ButtplugMessageSpecVersion::Version1 {
        return ButtplugServerMessageVariant::V1(da1.into());
      }
      let da0 = DeviceAddedV0::from(da1);
      return ButtplugServerMessageVariant::V0(ButtplugServerMessageV0::DeviceAdded(da0));
    } else {
      // Device Removed
      let disconnected_indexes: Vec<u32> = self.device_indexes.iter().filter(|x| !new_indexes.contains(x)).map(|x| *x).collect();
      self.device_indexes.remove(&disconnected_indexes[0]);
      match version {
        ButtplugMessageSpecVersion::Version0 => {
          return ButtplugServerMessageVariant::V0(ButtplugServerMessageV0::DeviceRemoved(DeviceRemovedV0::new(disconnected_indexes[0])))
        }
        ButtplugMessageSpecVersion::Version1 => {
          return ButtplugServerMessageVariant::V1(DeviceRemovedV0::new(disconnected_indexes[0]).into())
        }
        ButtplugMessageSpecVersion::Version2 => {
          return ButtplugServerMessageVariant::V2(DeviceRemovedV0::new(disconnected_indexes[0]).into())
        }
        ButtplugMessageSpecVersion::Version3 => {
          return ButtplugServerMessageVariant::V3(DeviceRemovedV0::new(disconnected_indexes[0]).into())
        }
        ButtplugMessageSpecVersion::Version4 => return ButtplugServerMessageVariant::V4(list.clone().into()),
      }
    }
    // There is no == here because the only way DeviceList would be returned is via a
    // RequestDeviceList call. Events will only ever be additions or deletions.
  }
}

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
      ButtplugServerMessageV4::InputReading(m) => {
        let original_msg = self.original_message.as_ref().unwrap();
        if let ButtplugClientMessageVariant::V3(ButtplugClientMessageV3::SensorReadCmd(msg)) =
          &original_msg
        {
          // We only ever implemented battery in v3, so only accept that.
          if let InputTypeData::Battery(value) = m.data() {
            let msg_out = SensorReadingV3::new(
              msg.device_index(),
              *msg.sensor_index(),
              *msg.sensor_type(),
              vec!(value.data() as i32)
            );
            Ok(msg_out.into())
          } else {
            Err(ButtplugMessageError::UnexpectedMessageType("SensorReading".to_owned()).into())
          }
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
