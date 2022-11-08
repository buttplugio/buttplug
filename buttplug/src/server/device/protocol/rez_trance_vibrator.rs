use crate::core::errors::ButtplugDeviceError;
use crate::core::message::Endpoint;
use crate::server::device::hardware::{HardwareCommand, HardwareWriteCmd};
use crate::server::device::protocol::{generic_protocol_setup, ProtocolHandler};

generic_protocol_setup!(RezTranceVibrator, "rez-trancevibrator");

#[derive(Default)]
pub struct RezTranceVibrator {}

impl ProtocolHandler for RezTranceVibrator {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let r_speed = scalar as u8;
    let data = vec![r_speed];
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::TxVendorControl,
      data,
      false,
    )
    .into()])
  }
}
