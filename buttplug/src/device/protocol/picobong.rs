use crate::create_buttplug_protocol;
use super::ButtplugProtocolCommandHandler;

create_buttplug_protocol!(
  // Protocol name
  Picobong,
  // Use the default protocol creator implementation. No special init needed.
  true,
  // No extra members
  ()
);

impl ButtplugProtocolCommandHandler for Picobong {
  fn handle_vibrate_cmd(
    &self,
    device: &dyn DeviceImpl,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // TODO Convert to using generic command manager
    let speed = (msg.speeds[0].speed * 10.0) as u8;
    let mode: u8 = if speed == 0 { 0xff } else { 0x01 };
    let msg = DeviceWriteCmd::new(Endpoint::Tx, [0x01, mode, speed].to_vec(), false);
    let fut = device.write_value(msg.into());
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write tests for protocol
