use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::{
    errors:: ButtplugMessageError,
    messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use byteorder::{LittleEndian, WriteBytesExt};
use std::sync::Arc;

super::default_protocol_definition!(XInput);

impl ButtplugProtocol for XInput {
  fn try_create(
    device_impl: Arc<crate::device::DeviceImpl>,
    config: crate::device::protocol::DeviceProtocolConfiguration,
  ) -> futures::future::BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>>
  {
    Box::pin(async move {
      // This must match the identifier in the device config, otherwise we'll fail to load controllers.
      let (name, attrs) = crate::device::protocol::get_protocol_features(device_impl, Some("XInput Gamepad".to_owned()), config)?;
      Ok(Box::new(Self::new(&name, attrs)) as Box<dyn ButtplugProtocol>)
    })
  }
}

impl ButtplugProtocolCommandHandler for XInput {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
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
            fut_vec.push(device.write_value(DeviceWriteCmd::new(Endpoint::Tx, cmd, false)));
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
}
