use std::collections::HashMap;

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      ButtplugClientMessageV4,
      ButtplugDeviceMessage,
      ButtplugMessage,
      ButtplugMessageFinalizer,
      ButtplugMessageValidator,
      PingV0,
      RawReadCmdV2,
      RawSubscribeCmdV2,
      RawUnsubscribeCmdV2,
      RawWriteCmdV2,
      RequestDeviceListV0,
      RequestServerInfoV1,
      StartScanningV0,
      StopAllDevicesV0,
      StopDeviceCmdV0,
      StopScanningV0,
    },
  },
  server::message::{
    legacy_device_attributes::TryFromClientMessage,
    v0::ButtplugClientMessageV0,
    v1::ButtplugClientMessageV1,
    v2::ButtplugClientMessageV2,
    v3::ButtplugClientMessageV3,
    ButtplugClientMessageVariant,
    LegacyDeviceAttributes,
    TryFromDeviceAttributes,
  },
};

use super::{checked_level_cmd::CheckedLevelCmdV4, checked_linear_cmd::CheckedLinearCmdV4, checked_sensor_read_cmd::CheckedSensorReadCmdV4, checked_sensor_subscribe_cmd::CheckedSensorSubscribeCmdV4, checked_sensor_unsubscribe_cmd::CheckedSensorUnsubscribeCmdV4};

/// An InternalClientMessage has had its contents verified and should need no further internal error
/// checking. Processing may still return errors, but should be due to system state, not message
/// contents.
///
/// There should only be one version of InternalClientMessage in the library, matching the latest
/// version of the message spec. For any messages that don't require error checking, their regular
/// struct can be used as an enum parameter. Any messages requiring error checking or validation
/// will have an alternate Internal[x] form that they will need to be cast as.
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
pub enum ButtplugInternalClientMessageV4 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV1),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopDeviceCmd(StopDeviceCmdV0),
  StopAllDevices(StopAllDevicesV0),
  LevelCmd(CheckedLevelCmdV4),
  LinearCmd(CheckedLinearCmdV4),
  // Sensor commands
  SensorReadCmd(CheckedSensorReadCmdV4),
  SensorSubscribeCmd(CheckedSensorSubscribeCmdV4),
  SensorUnsubscribeCmd(CheckedSensorUnsubscribeCmdV4),
  // Raw commands
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
}

impl TryFromClientMessage<ButtplugClientMessageV4> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    value: ButtplugClientMessageV4,
    feature_map: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    match value {
      // Messages that don't need checking
      ButtplugClientMessageV4::RequestServerInfo(m) => {
        Ok(ButtplugInternalClientMessageV4::RequestServerInfo(m))
      }
      ButtplugClientMessageV4::Ping(m) => Ok(ButtplugInternalClientMessageV4::Ping(m)),
      ButtplugClientMessageV4::StartScanning(m) => {
        Ok(ButtplugInternalClientMessageV4::StartScanning(m))
      }
      ButtplugClientMessageV4::StopScanning(m) => {
        Ok(ButtplugInternalClientMessageV4::StopScanning(m))
      }
      ButtplugClientMessageV4::RequestDeviceList(m) => {
        Ok(ButtplugInternalClientMessageV4::RequestDeviceList(m))
      }
      ButtplugClientMessageV4::StopAllDevices(m) => {
        Ok(ButtplugInternalClientMessageV4::StopAllDevices(m))
      }

      // Messages that need device index checking
      ButtplugClientMessageV4::StopDeviceCmd(m) => {
        if feature_map.get(&m.device_index()).is_some() {
          Ok(ButtplugInternalClientMessageV4::StopDeviceCmd(m))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }
      }

      // Message that need device index and feature checking
      ButtplugClientMessageV4::LevelCmd(m) => {
        if let Some(features) = feature_map.get(&m.device_index()) {
          Ok(ButtplugInternalClientMessageV4::LevelCmd(
            CheckedLevelCmdV4::try_from_device_attributes(m, features)?,
          ))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }
      }
      ButtplugClientMessageV4::LinearCmd(m) => {
        if let Some(features) = feature_map.get(&m.device_index()) {
          Ok(ButtplugInternalClientMessageV4::LinearCmd(
            CheckedLinearCmdV4::try_from_device_attributes(m, features)?,
          ))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }
      }
      ButtplugClientMessageV4::SensorReadCmd(m) => {
        if let Some(features) = feature_map.get(&m.device_index()) {
          Ok(ButtplugInternalClientMessageV4::SensorReadCmd(
            CheckedSensorReadCmdV4::try_from_device_attributes(m, features)?,
          ))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }
      }
      ButtplugClientMessageV4::SensorSubscribeCmd(m) => {
        if let Some(features) = feature_map.get(&m.device_index()) {
          Ok(ButtplugInternalClientMessageV4::SensorSubscribeCmd(
            CheckedSensorSubscribeCmdV4::try_from_device_attributes(m, features)?,
          ))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }
      }
      ButtplugClientMessageV4::SensorUnsubscribeCmd(m) => {
        if let Some(features) = feature_map.get(&m.device_index()) {
          Ok(ButtplugInternalClientMessageV4::SensorUnsubscribeCmd(
            CheckedSensorUnsubscribeCmdV4::try_from_device_attributes(m, features)?,
          ))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }      }

      // Message that need device index and hardware endpoint checking
      ButtplugClientMessageV4::RawWriteCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::RawWriteCmd(m))
      }
      ButtplugClientMessageV4::RawReadCmd(m) => Ok(ButtplugInternalClientMessageV4::RawReadCmd(m)),
      ButtplugClientMessageV4::RawSubscribeCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::RawSubscribeCmd(m))
      }
      ButtplugClientMessageV4::RawUnsubscribeCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::RawUnsubscribeCmd(m))
      }
    }
  }
}

// For v3 to v4, all deprecations should be treated as conversions, but will require current
// connected device state, meaning they'll need to be implemented where they can also access the
// device manager.
impl TryFrom<ButtplugClientMessageV3> for ButtplugInternalClientMessageV4 {
  type Error = ButtplugMessageError;

  fn try_from(value: ButtplugClientMessageV3) -> Result<Self, Self::Error> {
    match value {
      ButtplugClientMessageV3::Ping(m) => Ok(ButtplugInternalClientMessageV4::Ping(m.clone())),
      ButtplugClientMessageV3::RequestServerInfo(m) => Ok(
        ButtplugInternalClientMessageV4::RequestServerInfo(m.clone()),
      ),
      ButtplugClientMessageV3::StartScanning(m) => {
        Ok(ButtplugInternalClientMessageV4::StartScanning(m.clone()))
      }
      ButtplugClientMessageV3::StopScanning(m) => {
        Ok(ButtplugInternalClientMessageV4::StopScanning(m.clone()))
      }
      ButtplugClientMessageV3::RequestDeviceList(m) => Ok(
        ButtplugInternalClientMessageV4::RequestDeviceList(m.clone()),
      ),
      ButtplugClientMessageV3::StopAllDevices(m) => {
        Ok(ButtplugInternalClientMessageV4::StopAllDevices(m.clone()))
      }
      ButtplugClientMessageV3::StopDeviceCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::StopDeviceCmd(m.clone()))
      }
      ButtplugClientMessageV3::RawReadCmd(m) => Ok(ButtplugInternalClientMessageV4::RawReadCmd(m)),
      ButtplugClientMessageV3::RawWriteCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::RawWriteCmd(m))
      }
      ButtplugClientMessageV3::RawSubscribeCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::RawSubscribeCmd(m))
      }
      ButtplugClientMessageV3::RawUnsubscribeCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::RawUnsubscribeCmd(m))
      }
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {:?} to V4 message spec while lacking state.",
        value
      ))),
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageVariant> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageVariant,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let id = msg.id();
    let mut converted_msg = match msg {
      ButtplugClientMessageVariant::V0(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V1(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V2(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V3(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V4(m) => Self::try_from_client_message(m, features),
    }?;
    // Always make sure the ID is set after conversion
    converted_msg.set_id(id);
    Ok(converted_msg)
  }
}

impl TryFromClientMessage<ButtplugClientMessageV0> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV0,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    // All v0 messages can be converted to v1 messages.
    Self::try_from_client_message(ButtplugClientMessageV1::from(msg), features)
  }
}

fn check_device_index_and_convert<T, U>(
  msg: T,
  features: &HashMap<u32, LegacyDeviceAttributes>,
) -> Result<U, ButtplugError>
where
  T: ButtplugDeviceMessage,
  U: TryFromDeviceAttributes<T>,
{
  // Vorze and RotateCmd are equivalent, so this is an ok conversion.
  if let Some(attrs) = features.get(&msg.device_index()) {
    Ok(U::try_from_device_attributes(msg.clone(), attrs)?.into())
  } else {
    Err(ButtplugError::from(
      ButtplugDeviceError::DeviceNotAvailable(msg.device_index()),
    ))
  }
}

impl TryFromClientMessage<ButtplugClientMessageV1> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV1,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    // Instead of converting to v2 message attributes then to v4 device features, we move directly
    // from v0 command messages to v4 device features here. There's no reason to do the middle step.
    match msg {
      ButtplugClientMessageV1::VorzeA10CycloneCmd(m) => {
        // Vorze and RotateCmd are equivalent, so this is an ok conversion.
        Ok(check_device_index_and_convert::<_, CheckedLevelCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV1::SingleMotorVibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedLevelCmdV4>(m, features)?.into())
      }
      _ => Self::try_from_client_message(ButtplugClientMessageV2::try_from(msg)?, features),
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageV2> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV2,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    match msg {
      // Convert v2 specific queries to v3 generic sensor queries
      ButtplugClientMessageV2::BatteryLevelCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedSensorReadCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV2::RSSILevelCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedSensorReadCmdV4>(m, features)?.into())
      }
      // Convert VibrateCmd to a ScalarCmd command
      ButtplugClientMessageV2::VibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedLevelCmdV4>(m, features)?.into())
      }
      _ => Self::try_from_client_message(ButtplugClientMessageV3::try_from(msg)?, features),
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageV3> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV3,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    match msg {
      // Convert v1/v2 message attribute commands into device feature commands
      ButtplugClientMessageV3::VibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedLevelCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::ScalarCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedLevelCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::RotateCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedLevelCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::LinearCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedLinearCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::SensorReadCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedSensorReadCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::SensorSubscribeCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedSensorSubscribeCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::SensorUnsubscribeCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedSensorUnsubscribeCmdV4>(m, features)?.into())
      }
      _ => {
        ButtplugInternalClientMessageV4::try_from(msg).map_err(|e: ButtplugMessageError| e.into())
      }
    }
  }
}

/// Represents messages that should go to the
/// [DeviceManager][crate::server::device_manager::DeviceManager] of a
/// [ButtplugServer](crate::server::ButtplugServer)
#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
pub(crate) enum ButtplugDeviceManagerMessageUnion {
  RequestDeviceList(RequestDeviceListV0),
  StopAllDevices(StopAllDevicesV0),
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
}

impl TryFrom<ButtplugInternalClientMessageV4> for ButtplugDeviceManagerMessageUnion {
  type Error = ();

  fn try_from(value: ButtplugInternalClientMessageV4) -> Result<Self, Self::Error> {
    match value {
      ButtplugInternalClientMessageV4::RequestDeviceList(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::RequestDeviceList(m))
      }
      ButtplugInternalClientMessageV4::StopAllDevices(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StopAllDevices(m))
      }
      ButtplugInternalClientMessageV4::StartScanning(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StartScanning(m))
      }
      ButtplugInternalClientMessageV4::StopScanning(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StopScanning(m))
      }
      _ => Err(()),
    }
  }
}

/// Represents all possible device command message types.
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugDeviceMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
pub enum ButtplugDeviceCommandMessageUnion {
  StopDeviceCmd(StopDeviceCmdV0),
  LinearCmd(CheckedLinearCmdV4),
  LevelCmd(CheckedLevelCmdV4),
  SensorReadCmd(CheckedSensorReadCmdV4),
  SensorSubscribeCmd(CheckedSensorSubscribeCmdV4),
  SensorUnsubscribeCmd(CheckedSensorUnsubscribeCmdV4),
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
}

impl TryFrom<ButtplugInternalClientMessageV4> for ButtplugDeviceCommandMessageUnion {
  type Error = ();

  fn try_from(value: ButtplugInternalClientMessageV4) -> Result<Self, Self::Error> {
    match value {
      ButtplugInternalClientMessageV4::StopDeviceCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::StopDeviceCmd(m))
      }
      ButtplugInternalClientMessageV4::LinearCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::LinearCmd(m))
      }
      ButtplugInternalClientMessageV4::LevelCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::LevelCmd(m))
      }
      ButtplugInternalClientMessageV4::SensorReadCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorReadCmd(m))
      }
      ButtplugInternalClientMessageV4::SensorSubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorSubscribeCmd(m))
      }
      ButtplugInternalClientMessageV4::SensorUnsubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorUnsubscribeCmd(m))
      }
      ButtplugInternalClientMessageV4::RawWriteCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawWriteCmd(m))
      }
      ButtplugInternalClientMessageV4::RawReadCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawReadCmd(m))
      }
      ButtplugInternalClientMessageV4::RawSubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(m))
      }
      ButtplugInternalClientMessageV4::RawUnsubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(m))
      }
      _ => Err(()),
    }
  }
}
