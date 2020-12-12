use super::SerialPortDeviceImplCreator;
use crate::{
  core::ButtplugResultFuture,
  server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerCreator,
  },
};
use async_channel::Sender;
use futures::future;
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

impl DeviceCommunicationManager for SerialPortCommunicationManager {
  fn name(&self) -> &'static str {
    "SerialPortCommunicationManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    info!("Scanning ports!");
    // TODO Does this block? Should it run in one of our threads?
    let sender = self.sender.clone();
    Box::pin(async move {
      match available_ports() {
        Ok(ports) => {
          info!("Got {} serial ports back", ports.len());
          for p in ports {
            info!("{:?}", p);
            if sender
              .send(DeviceCommunicationEvent::DeviceFound(Box::new(
                SerialPortDeviceImplCreator::new(&p),
              )))
              .await
              .is_err()
            {
              error!("Device manager disappeared, exiting.");
              break;
            }
          }
        }
        Err(_) => {
          info!("No serial ports found");
        }
      }
      if sender
        .send(DeviceCommunicationEvent::ScanningFinished)
        .await
        .is_err()
      {
        error!("Error sending scanning finished from Xinput.");
      }
      Ok(())
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }
}
