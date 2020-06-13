use crate::create_buttplug_protocol;
use super::ButtplugProtocolCommandHandler;

create_buttplug_protocol!(
  // Protocol name
  Svakom,
  // Use the default protocol creator implementation. No special init needed.
  true,
  // No extra members
  ()
);

impl ButtplugProtocolCommandHandler for Svakom {
  fn handle_vibrate_cmd(
    &self,
    device: &dyn DeviceImpl,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // TODO Convert to using generic command manager
    let speed = (msg.speeds[0].speed * 19.0) as u8;
    let multiplier: u8 = if speed == 0x00 { 0x00 } else { 0x01 };
    let msg = DeviceWriteCmd::new(
      Endpoint::Tx,
      [0x55, 0x04, 0x03, 0x00, multiplier, speed].to_vec(),
      false,
    );
    let fut = device.write_value(msg.into());
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write Tests
