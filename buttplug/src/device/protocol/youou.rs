use super::ButtplugProtocolCommandHandler;
use crate::create_buttplug_protocol;

create_buttplug_protocol!(
  // Protocol name,
  Youou,
  // Use the default protocol creator implementation. No special init needed.
  true,
  // Protocol members
  ((packet_id: RefCell<u8> = RefCell::new(0)))
);

impl ButtplugProtocolCommandHandler for Youou {
  fn handle_vibrate_cmd(
    &self,
    device: &dyn DeviceImpl,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // TODO Convert to using generic command manager

    // Byte 2 seems to be a monotonically increasing packet id of some kind
    //
    // Speed seems to be 0-247 or so.
    //
    // Anything above that sets a pattern which isn't what we want here.
    let max_value: f64 = 247.0;
    let speed: u8 = (msg.speeds[0].speed * max_value) as u8;
    let state: u8 = if speed > 0 { 1 } else { 0 };

    let mut data;
    // Scope the packet id set so we can unlock ASAP.
    {
      data = vec![
        0xaa,
        0x55,
        *self.packet_id.borrow(),
        0x02,
        0x03,
        0x01,
        speed,
        state,
      ];
      let new_val = self.packet_id.borrow().wrapping_add(1);
      *self.packet_id.borrow_mut() = new_val;
    }
    let mut crc: u8 = 0;

    // Simple XOR of everything up to the 9th byte for CRC.
    for b in data.clone() {
      crc ^= b;
    }

    let mut data2 = vec![crc, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    data.append(&mut data2);

    let msg = DeviceWriteCmd::new(Endpoint::Tx, data, false);
    let fut = device.write_value(msg.into());
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    test::{check_recv_value, TestDevice},
    util::async_manager,
  };

  #[test]
  pub fn test_youou_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = TestDevice::new_bluetoothle_test_device("VX001_")
        .await
        .unwrap();
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      let (_, command_receiver) = test_device.get_endpoint_channel_clone(Endpoint::Tx).await;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0xaa,
            0x55,
            0x00,
            0x02,
            0x03,
            0x01,
            (247.0f32 / 2.0f32) as u8,
            0x01,
            0x85,
            0xff,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
          ],
          false,
        )),
      )
      .await;
      // Test to make sure we handle packet IDs across protocol clones correctly.
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0xaa, 0x55, 0x01, 0x02, 0x03, 0x01, 0x00, 0x00, 0xfe, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,
          ],
          false,
        )),
      )
      .await;
    });
  }
}
