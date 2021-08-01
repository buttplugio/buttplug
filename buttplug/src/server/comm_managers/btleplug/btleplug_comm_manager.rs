
use crate::{
  core::ButtplugResultFuture,
  server::comm_managers::{
    DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerBuilder,
  },
  util::async_manager,
};
use super::btleplug_adapter_task::{BtleplugAdapterCommand, BtleplugAdapterTask};
use std::{
  sync::{
    atomic::AtomicBool,
    Arc,
  },
};

use tokio::{
  sync::mpsc::{Sender, channel},
};


#[derive(Default)]
pub struct BtlePlugCommunicationManagerBuilder {
  sender: Option<Sender<DeviceCommunicationEvent>>
}

impl DeviceCommunicationManagerBuilder for BtlePlugCommunicationManagerBuilder {
  fn event_sender(mut self, sender: Sender<DeviceCommunicationEvent>) -> Self {
    self.sender = Some(sender);
    self
  }

  fn finish(mut self) -> Box<dyn DeviceCommunicationManager> {
    Box::new(BtlePlugCommunicationManager::new(self.sender.take().unwrap()))
  }
}

pub struct BtlePlugCommunicationManager {
  adapter_event_sender: Sender<BtleplugAdapterCommand>,
}

impl BtlePlugCommunicationManager {
  pub fn new(event_sender: Sender<DeviceCommunicationEvent>) -> Self {
    let (sender, receiver) = channel(256);
    async_manager::spawn(async move {
      let mut task = BtleplugAdapterTask::new(event_sender, receiver);
      task.run().await;
    }).unwrap();
    Self {
      adapter_event_sender: sender
    }
  }
}

impl DeviceCommunicationManager for BtlePlugCommunicationManager {
  fn name(&self) -> &'static str {
    "BtlePlugCommunicationManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    let adapter_event_sender = self.adapter_event_sender.clone();
    Box::pin(async move {
      adapter_event_sender.send(BtleplugAdapterCommand::StartScanning).await.unwrap();
      Ok(())
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    let adapter_event_sender = self.adapter_event_sender.clone();
    Box::pin(async move {
      adapter_event_sender.send(BtleplugAdapterCommand::StopScanning).await.unwrap();
      Ok(())
    })
  }

  fn scanning_status(&self) -> Arc<AtomicBool> {
    return Arc::new(AtomicBool::new(false));
  }
}
/*
impl Drop for BtlePlugCommunicationManager {
  fn drop(&mut self) {
    info!("Dropping btleplug comm manager.");
    if self.adapter.is_some() {
      if let Err(e) = self.adapter.as_ref().unwrap().stop_scan() {
        info!("Error on scanning shutdown for bluetooth: {:?}", e);
      }
    }
  }
}
 */