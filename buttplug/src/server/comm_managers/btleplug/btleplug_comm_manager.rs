use super::btleplug_adapter_task::{BtleplugAdapterCommand, BtleplugAdapterTask};
use crate::{
  core::{errors::ButtplugDeviceError, ButtplugResultFuture},
  server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerBuilder,
  },
  util::async_manager,
};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};

use tokio::sync::mpsc::{channel, Sender};

#[derive(Default)]
pub struct BtlePlugCommunicationManagerBuilder {
  sender: Option<Sender<DeviceCommunicationEvent>>,
}

impl DeviceCommunicationManagerBuilder for BtlePlugCommunicationManagerBuilder {
  fn event_sender(mut self, sender: Sender<DeviceCommunicationEvent>) -> Self {
    self.sender = Some(sender);
    self
  }

  fn finish(mut self) -> Box<dyn DeviceCommunicationManager> {
    Box::new(BtlePlugCommunicationManager::new(
      self
        .sender
        .take()
        .expect("Device Manager will set this during initialization."),
    ))
  }
}

pub struct BtlePlugCommunicationManager {
  adapter_event_sender: Sender<BtleplugAdapterCommand>,
  scanning_status: Arc<AtomicBool>,
}

impl BtlePlugCommunicationManager {
  pub fn new(event_sender: Sender<DeviceCommunicationEvent>) -> Self {
    let (sender, receiver) = channel(256);
    async_manager::spawn(async move {
      let mut task = BtleplugAdapterTask::new(event_sender, receiver);
      task.run().await;
    });
    Self {
      adapter_event_sender: sender,
      scanning_status: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl DeviceCommunicationManager for BtlePlugCommunicationManager {
  fn name(&self) -> &'static str {
    "BtlePlugCommunicationManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    let adapter_event_sender = self.adapter_event_sender.clone();
    let scanning_status = self.scanning_status.clone();
    // Set to true just to make sure we don't call ScanningFinished too early.
    scanning_status.store(true, Ordering::SeqCst);
    Box::pin(async move {
      if adapter_event_sender
        .send(BtleplugAdapterCommand::StartScanning)
        .await
        .is_err()
      {
        error!("Error starting scan, cannot send to btleplug event loop.");
        scanning_status.store(false, Ordering::SeqCst);
        Err(
          ButtplugDeviceError::DeviceConnectionError(
            "Cannot send start scanning request to event loop.".to_owned(),
          )
          .into(),
        )
      } else {
        Ok(())
      }
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    let adapter_event_sender = self.adapter_event_sender.clone();
    // Just assume any outcome of this means we're done scanning.
    self.scanning_status.store(false, Ordering::SeqCst);
    Box::pin(async move {
      if adapter_event_sender
        .send(BtleplugAdapterCommand::StopScanning)
        .await
        .is_err()
      {
        error!("Error stopping scan, cannot send to btleplug event loop.");
        Err(
          ButtplugDeviceError::DeviceConnectionError(
            "Cannot send stop scanning request to event loop.".to_owned(),
          )
          .into(),
        )
      } else {
        Ok(())
      }
    })
  }

  fn scanning_status(&self) -> Arc<AtomicBool> {
    self.scanning_status.clone()
  }
}
/*
impl Drop for BtlePlugCommunicationManager {
  fn drop(&mut self) {
    info!("Dropping btleplug comm manager.");
    if self.adapter.is_some() {
      if let Err(e) = self.adapter.as_ref().expect("Already checked validity").stop_scan() {
        info!("Error on scanning shutdown for bluetooth: {:?}", e);
      }
    }
  }
}
 */
