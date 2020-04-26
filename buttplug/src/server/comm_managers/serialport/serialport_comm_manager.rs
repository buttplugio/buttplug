use super::SerialPortDeviceImplCreator;
use crate::{
  core::errors::ButtplugError,
  server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerCreator,
  },
};
use async_std::sync::Sender;
use async_trait::async_trait;
use serialport::available_ports;

pub struct SerialPortCommunicationManager {
  sender: Sender<DeviceCommunicationEvent>,
}

impl DeviceCommunicationManagerCreator for SerialPortCommunicationManager {
  fn new(sender: Sender<DeviceCommunicationEvent>) -> Self {
    info!("Serial port created!");
    Self { sender }
  }
}

#[async_trait]
impl DeviceCommunicationManager for SerialPortCommunicationManager {
  async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
    info!("Scanning ports!");
    // TODO Does this block? Should it run in one of our threads?
    match available_ports() {
      Ok(ports) => {
        info!("Got {} serial ports back", ports.len());
        for p in ports {
          info!("{:?}", p);
          self
            .sender
            .send(DeviceCommunicationEvent::DeviceFound(Box::new(
              SerialPortDeviceImplCreator::new(&p),
            )))
            .await;
        }
      }
      Err(_) => {
        info!("No serial ports found");
      }
    }
    Ok(())
  }

  async fn stop_scanning(&mut self) -> Result<(), ButtplugError> {
    Ok(())
  }

  fn is_scanning(&mut self) -> bool {
    false
  }
}
