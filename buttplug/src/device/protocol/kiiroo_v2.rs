use super::{
  fleshlight_launch_helper::get_speed, ButtplugDeviceResultFuture, ButtplugProtocol,
  ButtplugProtocolCommandHandler,
};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{
      self, ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessage, DeviceMessageAttributesMap,
      FleshlightLaunchFW12Cmd,
    },
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
};
use futures::future::BoxFuture;
use std::sync::{
  atomic::{AtomicU8, Ordering::SeqCst},
  Arc,
};
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct KiirooV2 {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  _manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  previous_position: Arc<AtomicU8>,
}

impl ButtplugProtocol for KiirooV2 {
  fn new_protocol(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol> {
    let manager = GenericCommandManager::new(&message_attributes);

    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      _manager: Arc::new(Mutex::new(manager)),
      previous_position: Arc::new(AtomicU8::new(0)),
    })
  }

  fn initialize(
    device_impl: Arc<DeviceImpl>,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    let msg = DeviceWriteCmd::new(Endpoint::Firmware, vec![0x0u8], true);
    let info_fut = device_impl.write_value(msg);
    Box::pin(async move {
      info_fut.await?;
      Ok(None)
    })
  }
}

impl ButtplugProtocolCommandHandler for KiirooV2 {
  fn handle_linear_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    let v = message.vectors()[0].clone();
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(SeqCst);
    let distance = (previous_position as f64 - (v.position * 99f64)).abs() / 99f64;
    let fl_cmd = FleshlightLaunchFW12Cmd::new(
      message.device_index(),
      (v.position * 99f64) as u8,
      (get_speed(distance, v.duration) * 99f64) as u8,
    );
    self.handle_fleshlight_launch_fw12_cmd(device, fl_cmd)
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugDeviceResultFuture {
    let previous_position = self.previous_position.clone();
    let position = message.position();
    let msg = DeviceWriteCmd::new(
      Endpoint::Tx,
      [message.position(), message.speed()].to_vec(),
      false,
    );
    let fut = device.write_value(msg);
    Box::pin(async move {
      previous_position.store(position, SeqCst);
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{FleshlightLaunchFW12Cmd, LinearCmd, VectorSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    test::{check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_kiiroov2_fleshlight_fw12cmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Launch").await.unwrap();
      let command_receiver = test_device.get_endpoint_receiver(&Endpoint::Tx).unwrap();
      device
        .parse_message(FleshlightLaunchFW12Cmd::new(0, 50, 50).into())
        .await
        .unwrap();
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![50, 50], false)),
      );
    });
  }

  #[test]
  pub fn test_kiiroov2_linearcmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Launch").await.unwrap();
      let command_receiver = test_device.get_endpoint_receiver(&Endpoint::Tx).unwrap();
      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 500, 0.5)]).into())
        .await
        .unwrap();
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![49, 19], false)),
      );
    });
  }
}
