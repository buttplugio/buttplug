use super::{
  fleshlight_launch_helper::calculate_speed,
  ButtplugDeviceResultFuture,
  ButtplugProtocol,
  ButtplugProtocolCommandHandler,
};
use crate::{
  core::messages::{
    self,
    ButtplugDeviceCommandMessageUnion,
    ButtplugDeviceMessage,
    FleshlightLaunchFW12Cmd,
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, DeviceAttributesBuilder},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::{
  atomic::{AtomicU8, Ordering::SeqCst},
  Arc,
};
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct KiirooV2 {
  device_attributes: ProtocolDeviceAttributes,
  _manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  previous_position: Arc<AtomicU8>,
}

impl KiirooV2 {
  fn new(device_attributes: ProtocolDeviceAttributes) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);

    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      _manager: Arc::new(Mutex::new(manager)),
      previous_position: Arc::new(AtomicU8::new(0)),
    }
  }
}

impl ButtplugProtocol for KiirooV2 {
  fn try_create(
    device_impl: Arc<crate::device::DeviceImpl>,
    builder: DeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    let msg = DeviceWriteCmd::new(Endpoint::Firmware, vec![0x0u8], true);
    let info_fut = device_impl.write_value(msg);
    Box::pin(async move {
      info_fut.await?;
      let device_attributes = builder.create_from_impl(&device_impl)?;
      Ok(Box::new(Self::new(device_attributes)) as Box<dyn ButtplugProtocol>)
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
      (calculate_speed(distance, v.duration) * 99f64) as u8,
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
    server::comm_managers::test::{check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_kiiroov2_fleshlight_fw12cmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Launch")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(FleshlightLaunchFW12Cmd::new(0, 50, 50).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![50, 50], false)),
      );
    });
  }

  #[test]
  pub fn test_kiiroov2_linearcmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Launch")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 500, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![49, 19], false)),
      );
    });
  }
}
