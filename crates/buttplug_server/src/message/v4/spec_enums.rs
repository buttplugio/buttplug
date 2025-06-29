use std::{collections::HashMap, fmt::Debug};

use crate::message::{
  server_device_attributes::TryFromClientMessage,
  v0::ButtplugClientMessageV0,
  v1::ButtplugClientMessageV1,
  v2::ButtplugClientMessageV2,
  v3::ButtplugClientMessageV3,
  ButtplugClientMessageVariant,
  RequestServerInfoV1,
  ServerDeviceAttributes,
  TryFromDeviceAttributes,
};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugClientMessageV4,
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    PingV0,
    RequestDeviceListV0,
    RequestServerInfoV4,
    StartScanningV0,
    StopAllDevicesV0,
    StopDeviceCmdV0,
    StopScanningV0,
  },
};

use super::{
  checked_input_cmd::CheckedInputCmdV4,
  checked_output_cmd::CheckedOutputCmdV4,
  checked_output_vec_cmd::CheckedOutputVecCmdV4,
};

/// An CheckedClientMessage has had its contents verified and should need no further error/validity
/// checking. Processing may still return errors, but should be due to system state, not message
/// contents.
///
/// There should only be one version of CheckedClientMessage in the library, matching the latest
/// version of the message spec. For any messages that don't require error checking, their regular
/// struct can be used as an enum parameter. Any messages requiring error checking or validation
/// will have an alternate Checked[x] form that they will need to be cast as.
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
pub enum ButtplugCheckedClientMessageV4 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV4),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopDeviceCmd(StopDeviceCmdV0),
  StopAllDevices(StopAllDevicesV0),
  OutputCmd(CheckedOutputCmdV4),
  // Sensor commands
  InputCmd(CheckedInputCmdV4),
  // Internal conversions for v1-v3 messages with subcommands
  OutputVecCmd(CheckedOutputVecCmdV4),
}

impl TryFromClientMessage<ButtplugClientMessageV4> for ButtplugCheckedClientMessageV4 {
  fn try_from_client_message(
    value: ButtplugClientMessageV4,
    feature_map: &HashMap<u32, ServerDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    match value {
      // Messages that don't need checking
      ButtplugClientMessageV4::RequestServerInfo(m) => {
        Ok(ButtplugCheckedClientMessageV4::RequestServerInfo(m))
      }
      ButtplugClientMessageV4::Ping(m) => Ok(ButtplugCheckedClientMessageV4::Ping(m)),
      ButtplugClientMessageV4::StartScanning(m) => {
        Ok(ButtplugCheckedClientMessageV4::StartScanning(m))
      }
      ButtplugClientMessageV4::StopScanning(m) => {
        Ok(ButtplugCheckedClientMessageV4::StopScanning(m))
      }
      ButtplugClientMessageV4::RequestDeviceList(m) => {
        Ok(ButtplugCheckedClientMessageV4::RequestDeviceList(m))
      }
      ButtplugClientMessageV4::StopAllDevices(m) => {
        Ok(ButtplugCheckedClientMessageV4::StopAllDevices(m))
      }

      // Messages that need device index checking
      ButtplugClientMessageV4::StopDeviceCmd(m) => {
        if feature_map.get(&m.device_index()).is_some() {
          Ok(ButtplugCheckedClientMessageV4::StopDeviceCmd(m))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }
      }

      // Message that need device index and feature checking
      ButtplugClientMessageV4::OutputCmd(m) => {
        if let Some(features) = feature_map.get(&m.device_index()) {
          Ok(ButtplugCheckedClientMessageV4::OutputCmd(
            CheckedOutputCmdV4::try_from_device_attributes(m, features)?,
          ))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }
      }
      ButtplugClientMessageV4::InputCmd(m) => {
        if let Some(features) = feature_map.get(&m.device_index()) {
          Ok(ButtplugCheckedClientMessageV4::InputCmd(
            CheckedInputCmdV4::try_from_device_attributes(m, features)?,
          ))
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNotAvailable(m.device_index()),
          ))
        }
      }
    }
  }
}

impl From<RequestServerInfoV1> for RequestServerInfoV4 {
  fn from(value: RequestServerInfoV1) -> Self {
    let mut msg = RequestServerInfoV4::new(value.client_name(), value.message_version(), 0);
    msg.set_id(value.id());
    msg
  }
}

// For v3 to v4, all deprecations should be treated as conversions, but will require current
// connected device state, meaning they'll need to be implemented where they can also access the
// device manager.
impl TryFrom<ButtplugClientMessageV3> for ButtplugCheckedClientMessageV4 {
  type Error = ButtplugMessageError;

  fn try_from(value: ButtplugClientMessageV3) -> Result<Self, Self::Error> {
    match value {
      ButtplugClientMessageV3::Ping(m) => Ok(ButtplugCheckedClientMessageV4::Ping(m.clone())),
      ButtplugClientMessageV3::RequestServerInfo(m) => Ok(
        ButtplugCheckedClientMessageV4::RequestServerInfo(RequestServerInfoV4::from(m)),
      ),
      ButtplugClientMessageV3::StartScanning(m) => {
        Ok(ButtplugCheckedClientMessageV4::StartScanning(m.clone()))
      }
      ButtplugClientMessageV3::StopScanning(m) => {
        Ok(ButtplugCheckedClientMessageV4::StopScanning(m.clone()))
      }
      ButtplugClientMessageV3::RequestDeviceList(m) => {
        Ok(ButtplugCheckedClientMessageV4::RequestDeviceList(m.clone()))
      }
      ButtplugClientMessageV3::StopAllDevices(m) => {
        Ok(ButtplugCheckedClientMessageV4::StopAllDevices(m.clone()))
      }
      ButtplugClientMessageV3::StopDeviceCmd(m) => {
        Ok(ButtplugCheckedClientMessageV4::StopDeviceCmd(m.clone()))
      }
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {value:?} to V4 message spec while lacking state."
      ))),
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageVariant> for ButtplugCheckedClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageVariant,
    features: &HashMap<u32, ServerDeviceAttributes>,
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
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

impl TryFromClientMessage<ButtplugClientMessageV0> for ButtplugCheckedClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV0,
    features: &HashMap<u32, ServerDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    // All v0 messages can be converted to v1 messages.
    Self::try_from_client_message(ButtplugClientMessageV1::from(msg), features)
  }
}

fn check_device_index_and_convert<T, U>(
  msg: T,
  features: &HashMap<u32, ServerDeviceAttributes>,
) -> Result<U, ButtplugError>
where
  T: ButtplugDeviceMessage + Debug,
  U: TryFromDeviceAttributes<T> + Debug,
{
  // Vorze and RotateCmd are equivalent, so this is an ok conversion.
  if let Some(attrs) = features.get(&msg.device_index()) {
    Ok(U::try_from_device_attributes(msg.clone(), attrs)?)
  } else {
    Err(ButtplugError::from(
      ButtplugDeviceError::DeviceNotAvailable(msg.device_index()),
    ))
  }
}

impl TryFromClientMessage<ButtplugClientMessageV1> for ButtplugCheckedClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV1,
    features: &HashMap<u32, ServerDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    // Instead of converting to v2 message attributes then to v4 device features, we move directly
    // from v0 command messages to v4 device features here. There's no reason to do the middle step.
    match msg {
      ButtplugClientMessageV1::VorzeA10CycloneCmd(_) => {
        // Vorze and RotateCmd are equivalent, so this is an ok conversion.
        Err(ButtplugError::ButtplugMessageError(ButtplugMessageError::MessageConversionError("VorzeA10CycloneCmd is considered unused, and no longer supported. If you are seeing this message and need VorzeA10CycloneCmd, file an issue in the Buttplug repo.".to_owned())))
      }
      ButtplugClientMessageV1::SingleMotorVibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedOutputVecCmdV4>(m, features)?.into())
      }
      _ => Self::try_from_client_message(ButtplugClientMessageV2::try_from(msg)?, features),
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageV2> for ButtplugCheckedClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV2,
    features: &HashMap<u32, ServerDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    match msg {
      // Convert v2 specific queries to v3 generic sensor queries
      ButtplugClientMessageV2::BatteryLevelCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedInputCmdV4>(m, features)?.into())
      }
      // Convert VibrateCmd to a ScalarCmd command
      ButtplugClientMessageV2::VibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedOutputVecCmdV4>(m, features)?.into())
      }
      _ => Self::try_from_client_message(ButtplugClientMessageV3::try_from(msg)?, features),
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageV3> for ButtplugCheckedClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV3,
    features: &HashMap<u32, ServerDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    match msg {
      // Convert v1/v2 message attribute commands into device feature commands
      ButtplugClientMessageV3::VibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedOutputVecCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::ScalarCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedOutputVecCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::RotateCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedOutputVecCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::LinearCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedOutputVecCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::SensorReadCmd(m) => {
        Ok(check_device_index_and_convert::<_, CheckedInputCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::SensorSubscribeCmd(_) => {
        // Always reject v3 sub/unsub. It was never implemented or indexed correctly.
        Err(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported("SensorSubscribeCmdV3".to_owned()),
        ))
      }
      ButtplugClientMessageV3::SensorUnsubscribeCmd(_) => {
        // Always reject v3 sub/unsub. It was never implemented or indexed correctly.
        Err(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported("SensorUnsubscribeCmdV3".to_owned()),
        ))
      }
      _ => {
        ButtplugCheckedClientMessageV4::try_from(msg).map_err(|e: ButtplugMessageError| e.into())
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

impl TryFrom<ButtplugCheckedClientMessageV4> for ButtplugDeviceManagerMessageUnion {
  type Error = ();

  fn try_from(value: ButtplugCheckedClientMessageV4) -> Result<Self, Self::Error> {
    match value {
      ButtplugCheckedClientMessageV4::RequestDeviceList(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::RequestDeviceList(m))
      }
      ButtplugCheckedClientMessageV4::StopAllDevices(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StopAllDevices(m))
      }
      ButtplugCheckedClientMessageV4::StartScanning(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StartScanning(m))
      }
      ButtplugCheckedClientMessageV4::StopScanning(m) => {
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
pub enum ButtplugDeviceCommandMessageUnionV4 {
  StopDeviceCmd(StopDeviceCmdV0),
  OutputCmd(CheckedOutputCmdV4),
  OutputVecCmd(CheckedOutputVecCmdV4),
  InputCmd(CheckedInputCmdV4),
}

impl TryFrom<ButtplugCheckedClientMessageV4> for ButtplugDeviceCommandMessageUnionV4 {
  type Error = ();

  fn try_from(value: ButtplugCheckedClientMessageV4) -> Result<Self, Self::Error> {
    match value {
      ButtplugCheckedClientMessageV4::StopDeviceCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnionV4::StopDeviceCmd(m))
      }
      ButtplugCheckedClientMessageV4::OutputCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnionV4::OutputCmd(m))
      }
      ButtplugCheckedClientMessageV4::OutputVecCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnionV4::OutputVecCmd(m))
      }
      ButtplugCheckedClientMessageV4::InputCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnionV4::InputCmd(m))
      }
      _ => Err(()),
    }
  }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum ButtplugDeviceMessageNameV4 {
  StopDeviceCmd,
  InputCmd,
  OutputCmd,
}
