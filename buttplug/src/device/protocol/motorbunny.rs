use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(Motorbunny);

impl ButtplugProtocolCommandHandler for Motorbunny {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        // Motorbunny only has one vibrator, we can assume the first element is
        // that.
        if let Some(speed) = cmds[0] {
          let mut command_vec: Vec<u8>;
          if speed == 0 {
            command_vec = vec![0xf0, 0x00, 0x00, 0x00, 0x00, 0xec];
          } else {
            command_vec = vec![0xff];
            let mut vibe_commands = [speed as u8, 0x14].repeat(7);
            let crc = vibe_commands
              .iter()
              .fold(0u8, |a, b| a.overflowing_add(*b).0);
            command_vec.append(&mut vibe_commands);
            command_vec.append(&mut vec![crc, 0xec]);
          }
          fut_vec.push(device.write_value(DeviceWriteCmd::new(Endpoint::Tx, command_vec, false)));
        }
      }
      // TODO Just use join_all here
      for fut in fut_vec {
        // TODO Do something about possible errors here
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    server::comm_managers::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
    util::async_manager,
  };

  #[test]
  pub fn test_motorbunny_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("MB Controller")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0xff, 0x80, 0x14, 0x80, 0x14, 0x80, 0x14, 0x80, 0x14, 0x80, 0x14, 0x80, 0x14, 0x80,
            0x14, 0x0c, 0xec,
          ],
          false,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xf0, 0x00, 0x00, 0x00, 0x00, 0xec],
          false,
        )),
      );
    });
  }
}
