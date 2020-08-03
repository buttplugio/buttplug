mod btleplug_device_impl;
mod btleplug_internal;

use crate::{
  core::{errors::ButtplugDeviceError, ButtplugResultFuture},
  server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerCreator,
  },
  util::async_manager,
};
use async_channel::{bounded, Receiver, Sender};
use futures::StreamExt;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
};

use broadcaster::BroadcastChannel;
use btleplug::api::{Central, CentralEvent, Peripheral};
#[cfg(target_os = "linux")]
use btleplug::bluez::{adapter::ConnectedAdapter as Adapter, manager::Manager};
#[cfg(any(target_os = "macos", target_os = "ios"))]
use btleplug::corebluetooth::{adapter::Adapter, manager::Manager};
#[cfg(target_os = "windows")]
use btleplug::winrtble::{adapter::Adapter, manager::Manager};
use btleplug_device_impl::BtlePlugDeviceImplCreator;
use dashmap::DashMap;

pub struct BtlePlugCommunicationManager {
  // BtlePlug says to only have one manager at a time, so we'll have the comm
  // manager hold it.
  manager: Manager,
  adapter: Option<Adapter>,
  adapter_event_stream: BroadcastChannel<CentralEvent>,
  device_sender: Sender<DeviceCommunicationEvent>,
  scanning_sender: Sender<()>,
  scanning_receiver: Receiver<()>,
  is_scanning: Arc<AtomicBool>,
}

impl BtlePlugCommunicationManager {
  fn get_central(&self) -> Option<Adapter> {
    let adapters = self.manager.adapters().unwrap();
    if adapters.is_empty() {
      return None;
    }

    let adapter = adapters.into_iter().next().unwrap();

    // Have to use return statements here due to multiple cfg calls, otherwise
    // parser gets unhappy?
    #[cfg(not(target_os = "linux"))]
    return Some(adapter);

    #[cfg(target_os = "linux")]
    return Some(adapter.connect().unwrap());
  }

  fn setup_adapter(&mut self) {
    let maybe_adapter = self.get_central();
    if maybe_adapter.is_none() {
      return;
    }
    let adapter = maybe_adapter.unwrap();
    let receiver = adapter.event_receiver().unwrap();
    self.adapter = Some(adapter);
    let event_broadcaster = self.adapter_event_stream.clone();
    thread::spawn(move || {
      // Since this is an std channel receiver, it's mpsc. That means we don't
      // have clone or sync. Therefore we have to wrap it in its own thread for
      // now and block the async calls instead.
      while let Ok(event) = receiver.recv() {
        // Send, then instantly receive and drop so we keep our local channel
        // clean.
        let mut event_broadcaster_clone = event_broadcaster.clone();
        async_manager::spawn(async move {
            // Can't fail, we own both sides
            let _ = event_broadcaster_clone.send(&event).await;
            event_broadcaster_clone.recv().await;
          }
        ).unwrap();
      }
    });
  }
}

impl DeviceCommunicationManagerCreator for BtlePlugCommunicationManager {
  fn new(device_sender: Sender<DeviceCommunicationEvent>) -> Self {
    let (scanning_sender, scanning_receiver) = bounded(256);
    let manager = Manager::new().unwrap();
    let mut comm_mgr = Self {
      manager,
      adapter: None,
      adapter_event_stream: BroadcastChannel::new(),
      device_sender,
      scanning_sender,
      scanning_receiver,
      is_scanning: Arc::new(AtomicBool::new(false)),
    };
    comm_mgr.setup_adapter();
    comm_mgr
  }
}

impl DeviceCommunicationManager for BtlePlugCommunicationManager {
  fn name(&self) -> &'static str {
    "BtlePlugCommunicationManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    // get the first bluetooth adapter
    debug!("Bringing up adapter.");
    // TODO What happens if we don't have a radio?
    if self.adapter.is_none() {
      error!("No adapter, can't scan.");
      return ButtplugDeviceError::UnhandledCommand(
        "Cannot scan, no bluetooth adapters found".to_owned(),
      )
      .into();
    }
    let device_sender = self.device_sender.clone();
    let sender = self.scanning_sender.clone();
    let mut receiver = self.scanning_receiver.clone();
    let is_scanning = self.is_scanning.clone();
    let tried_addresses = Arc::new(DashMap::new());
    let tried_addressses_clone = tried_addresses.clone();

    let mut adapter_event_handler = self.adapter_event_stream.clone();
    info!("Setting bluetooth device event handler.");
    async_manager::spawn(async move {
      while let Some(event) = adapter_event_handler.next().await {
        match event {
          CentralEvent::DeviceDiscovered(_) => {
            debug!("BTLEPlug Device discovered: {:?}", event);
            let s = sender.clone();
            if s.send(()).await.is_err() {
              error!("Device scanning receiver dropped!");
            }
          }
          CentralEvent::DeviceDisconnected(addr) => {
            debug!("BTLEPlug Device disconnected: {:?}", event);
            tried_addressses_clone.remove(&addr);
          }
          _ => {}
        }
      }
    })
    .unwrap();

    let central = self.adapter.clone().unwrap();
    let adapter_event_handler_clone = self.adapter_event_stream.clone();
    Box::pin(async move {
      info!("Starting scan.");
      if let Err(err) = central.start_scan() {
        // TODO Explain the setcap issue on linux here.
        return Err(ButtplugDeviceError::DevicePermissionError(format!("BTLEPlug cannot start scanning. This may be a permissions error (on linux) or an issue with finding the radio. Reason: {}", err)).into());
      }
      is_scanning.store(true, Ordering::SeqCst);
      async_manager::spawn(async move {
        // When stop_scanning is called, this will get false and stop the
        // task.
        while is_scanning.load(Ordering::SeqCst) {
          for p in central.peripherals() {
            // If a device has no discernable name, we can't do anything
            // with it, just ignore it.
            //
            // TODO Should probably at least log this and add it to the
            // tried_addresses thing, once that exists.
            if let Some(name) = p.properties().local_name {
              //debug!("Found device {}", name);
              // Names are the only way we really have to test devices
              // at the moment. Most devices don't send services on
              // advertisement.
              if !name.is_empty() && !tried_addresses.contains_key(&p.properties().address) {
                tried_addresses.insert(p.properties().address, ());
                let device_creator = Box::new(BtlePlugDeviceImplCreator::new(
                  p,
                  adapter_event_handler_clone.clone(),
                ));
                if device_sender
                  .send(DeviceCommunicationEvent::DeviceFound(device_creator))
                  .await
                  .is_err()
                {
                  error!("Device manager receiver dropped, cannot send device found message.");
                  return;
                }
              }
            }
          }
          receiver.next().await.unwrap();
        }
        central.stop_scan().unwrap();
        info!("Exiting btleplug scanning");
      })
      .unwrap();
      Ok(())
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    let is_scanning = self.is_scanning.clone();
    let sender = self.scanning_sender.clone();
    Box::pin(async move {
      if is_scanning.load(Ordering::SeqCst) {
        is_scanning.store(false, Ordering::SeqCst);
        sender.send(()).await.map_err(|_| {
          error!("Scanning event loop already shut down");
          ButtplugDeviceError::DeviceScanningAlreadyStopped.into()
        })
      } else {
        Err(ButtplugDeviceError::DeviceScanningAlreadyStopped.into())
      }
    })
  }

  fn scanning_status(&self) -> Arc<AtomicBool> {
    self.is_scanning.clone()
  }
}

impl Drop for BtlePlugCommunicationManager {
  fn drop(&mut self) {
    info!("Dropping Comm Manager!");
    if self.adapter.is_some() {
      if let Err(e) = self.adapter.as_ref().unwrap().stop_scan() {
        info!("Error on scanning shutdown for bluetooth: {:?}", e);
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::BtlePlugCommunicationManager;
  use crate::{
    server::comm_managers::{
      DeviceCommunicationEvent,
      DeviceCommunicationManager,
      DeviceCommunicationManagerCreator,
    },
    util::async_manager,
  };
  use async_channel::bounded;
  use futures::StreamExt;

  // Ignored because it requires a device. Should probably just be a manual integration test.
  #[test]
  #[ignore]
  pub fn test_btleplug() {
    async_manager::block_on(async move {
      let (sender, mut receiver) = bounded(256);
      let mgr = BtlePlugCommunicationManager::new(sender);
      mgr.start_scanning().await.unwrap();
      loop {
        match receiver.next().await.unwrap() {
          DeviceCommunicationEvent::DeviceFound(_device) => {
            info!("Got device!");
            info!("Sending message!");
            // TODO since we don't return full devices as this point
            // anymore, we need to find some other way to test this.
            //
            // match device
            //     .parse_message(
            //         &VibrateCmd::new(1, vec![VibrateSubcommand::new(0, 0.5)]).into(),
            //     )
            //     .await
            // {
            //     Ok(msg) => match msg {
            //         ButtplugMessageUnion::Ok(_) => info!("Returned Ok"),
            //         _ => info!("Returned something other than ok"),
            //     },
            //     Err(_) => {
            //         assert!(false, "Error returned from parse message");
            //     }
            // }
          }
          _ => unreachable!("Shouldn't get other message types!"),
        }
      }
    });
  }
}
