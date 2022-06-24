// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolFactory};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugMessageError},
    messages::{
      self, BatteryLevelReading, ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessage,
      ButtplugServerMessage, Endpoint,
    },
  },
  server::{
    device::{
      configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
      hardware::{Hardware, HardwareReadCmd, HardwareWriteCmd},
      protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    },
    ButtplugServerResultFuture,
  },
};
use byteorder::{LittleEndian, WriteBytesExt};
use std::sync::Arc;

super::default_protocol_definition!(XInput, "xinput");

#[derive(Default, Debug)]
pub struct XInputFactory {}

impl ButtplugProtocolFactory for XInputFactory {
  fn try_create(
    &self,
    hardware: Arc<Hardware>,
    builder: ProtocolDeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    Box::pin(async move {
      // This must match the identifier in the device config, otherwise we'll fail to load controllers.
      let device_attributes = builder.create_from_hardware(&hardware)?;
      /*
      let name = format!(
        "{} {}",
        name,
        hardware
          .address()
          .chars()
          .last()
          .expect("We already set the address before getting here")
      );
      */
      Ok(Box::new(XInput::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    "xinput"
  }
}

impl ProtocolHandler for XInput {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      // Store off result before the match, so we drop the lock ASAP.
      let result = manager.lock().await.update_vibration(&msg, true);
      // My life for an async closure so I could just do this via and_then(). :(
      match result {
        Ok(cmds_option) => {
          let mut fut_vec = vec![];
          if let Some(cmds) = cmds_option {
            // XInput is fast enough that we can ignore the commands handed
            // back by the manager and just form our own packet. This means
            // we'll just use the manager's return for command validity
            // checking.
            let mut cmd = vec![];
            if cmd
              .write_u16::<LittleEndian>(
                cmds[1].expect("GCM uses match_all, we'll always get 2 values") as u16,
              )
              .is_err()
              || cmd
                .write_u16::<LittleEndian>(
                  cmds[0].expect("GCM uses match_all, we'll always get 2 values") as u16,
                )
                .is_err()
            {
              return Err(
                ButtplugMessageError::MessageConversionError(
                  "Cannot convert XInput value for processing".to_owned(),
                )
                .into(),
              );
            }
            fut_vec.push(device.write_value(HardwareWriteCmd::new(Endpoint::Tx, cmd, false)));
          }

          for fut in fut_vec {
            fut.await?;
          }
          Ok(messages::Ok::default().into())
        }
        Err(e) => Err(e),
      }
    })
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    _message: messages::BatteryLevelCmd,
  ) -> ButtplugServerResultFuture {
    Box::pin(async move {
      let rawreading = device
        .read_value(HardwareReadCmd::new(Endpoint::Rx, 0, 0))
        .await?;
      let id = rawreading.device_index();
      let battery = match rawreading.data()[0] {
        0 => 0.0,
        1 => 0.33,
        2 => 0.66,
        3 => 1.0,
        _ => {
          return Err(
            ButtplugDeviceError::DeviceCommunicationError(format!("something went wrong")).into(),
          )
        }
      };
      Ok(ButtplugServerMessage::BatteryLevelReading(
        BatteryLevelReading::new(id, battery),
      ))
    })
  }
}
