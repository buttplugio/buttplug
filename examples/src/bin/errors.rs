use buttplug_client::ButtplugClientError;
use buttplug_core::errors::ButtplugError;

#[allow(dead_code)]
fn handle_error(error: ButtplugClientError) {
  match error {
    ButtplugClientError::ButtplugConnectorError(_details) => {}
    ButtplugClientError::ButtplugError(error) => match error {
      ButtplugError::ButtplugHandshakeError(_details) => {}
      ButtplugError::ButtplugDeviceError(_details) => {}
      ButtplugError::ButtplugMessageError(_details) => {}
      ButtplugError::ButtplugPingError(_details) => {}
      ButtplugError::ButtplugUnknownError(_details) => {}
    },
    ButtplugClientError::ButtplugOutputCommandConversionError(_details) => {}
  }
}

fn main() {
  // nothing to do here
}
