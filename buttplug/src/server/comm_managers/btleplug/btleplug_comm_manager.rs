
use crate::{
  core::{errors::ButtplugDeviceError, ButtplugResultFuture},
  server::comm_managers::{
    DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerBuilder,
  },
  util::async_manager,
};
use super::btleplug_adapter_task::{BtleplugAdapterCommand, BtleplugAdapterTask};
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
};

use btleplug::api::{BDAddr, bleuuid::uuid_from_u16, CentralEvent, Central, Manager as _, Peripheral as _, WriteType};
use btleplug::platform::{Adapter, Manager};
// use btleplug_device_impl::BtlePlugDeviceImplCreator;
use dashmap::DashMap;
use tokio::{
  sync::mpsc::{Sender, channel},
  runtime::Handle,
};


#[derive(Default)]
pub struct BtlePlugCommunicationManagerBuilder {
  sender: Option<Sender<DeviceCommunicationEvent>>
}

impl DeviceCommunicationManagerBuilder for BtlePlugCommunicationManagerBuilder {
  fn set_event_sender(&mut self, sender: Sender<DeviceCommunicationEvent>) {
    self.sender = Some(sender)
  }

  fn finish(mut self) -> Box<dyn DeviceCommunicationManager> {
    Box::new(BtlePlugCommunicationManager::new(self.sender.take().unwrap()))
  }
}

pub struct BtlePlugCommunicationManager {
  event_sender: Sender<DeviceCommunicationEvent>,
  adapter_event_sender: Sender<BtleplugAdapterCommand>,
}

impl BtlePlugCommunicationManager {
  pub fn new(event_sender: Sender<DeviceCommunicationEvent>) -> Self {
    let (sender, receiver) = channel(256);
    let event_sender_clone = event_sender.clone();
    async_manager::spawn(async move {
      let mut task = BtleplugAdapterTask::new(event_sender_clone, receiver);
      task.run().await;
    }).unwrap();
    Self {
      event_sender,
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