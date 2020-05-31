mod btleplug_device_impl;
mod btleplug_internal;

use crate::{
  core::{
    ButtplugResultFuture,
    errors::ButtplugDeviceError,
  },
  server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerCreator,
  },
};
use async_std::{
  prelude::StreamExt,
  sync::{channel, Sender},
  task,
};
use btleplug::api::{Central, CentralEvent, Peripheral};
#[cfg(target_os = "linux")]
use btleplug::bluez::{adapter::ConnectedAdapter, manager::Manager};
#[cfg(any(target_os = "macos", target_os = "ios"))]
use btleplug::corebluetooth::{adapter::Adapter, manager::Manager};
#[cfg(target_os = "windows")]
use btleplug::winrtble::{adapter::Adapter, manager::Manager};
use btleplug_device_impl::BtlePlugDeviceImplCreator;

pub struct BtlePlugCommunicationManager {
  // BtlePlug says to only have one manager at a time, so we'll have the comm
  // manager hold it.
  manager: Manager,
  device_sender: Sender<DeviceCommunicationEvent>,
  scanning_sender: Option<Sender<bool>>,
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "ios"))]
impl BtlePlugCommunicationManager {
  fn get_central(&self) -> Adapter {
    let adapters = self.manager.adapters().unwrap();
    adapters.into_iter().nth(0).unwrap()
  }
}

#[cfg(target_os = "linux")]
impl BtlePlugCommunicationManager {
  fn get_central(&self) -> ConnectedAdapter {
    let adapters = self.manager.adapters().unwrap();
    let adapter = adapters.into_iter().next().unwrap();
    adapter.connect().unwrap()
  }
}

impl DeviceCommunicationManagerCreator for BtlePlugCommunicationManager {
  fn new(device_sender: Sender<DeviceCommunicationEvent>) -> Self {
    Self {
      manager: Manager::new().unwrap(),
      device_sender,
      scanning_sender: None,
    }
  }
}

impl DeviceCommunicationManager for BtlePlugCommunicationManager {
  fn start_scanning(&mut self) -> ButtplugResultFuture {
    // get the first bluetooth adapter
    debug!("Bringing up adapter.");
    let central = self.get_central();
    let device_sender = self.device_sender.clone();
    let (sender, mut receiver) = channel(256);
    self.scanning_sender = Some(sender.clone());
    let on_event = move |event: CentralEvent| {
      if let CentralEvent::DeviceDiscovered(_) = event {
        let s = sender.clone();
        task::spawn(async move {
          s.send(true).await;
        });
      }
    };
    // TODO There's no way to unsubscribe central event handlers. That
    // needs to be fixed in rumble somehow, but for now we'll have to
    // make our handlers exit early after dying or something?
    central.on_event(Box::new(on_event));
    info!("Starting scan.");
    if let Err(err) = central.start_scan() {
      // TODO Explain the setcap issue on linux here.
      return ButtplugDeviceError::new(&format!("BTLEPlug cannot start scanning. This may be a permissions error (on linux) or an issue with finding the radio. Reason: {}", err)).into();
    }
    Box::pin(async {
    task::spawn(async move {
      // TODO This should be "tried addresses" probably. Otherwise if we
      // want to connect, say, 2 launches, we're going to have a Bad Time.
      let mut tried_names: Vec<String> = vec![];
      // When stop_scanning is called, this will get false and stop the
      // task.
      while receiver.next().await.unwrap() {
        for p in central.peripherals() {
          // If a device has no discernable name, we can't do anything
          // with it, just ignore it.
          //
          // TODO Should probably at least log this and add it to the
          // tried_addresses thing, once that exists.
          if let Some(name) = p.properties().local_name {
            debug!("Found device {}", name);
            // Names are the only way we really have to test devices
            // at the moment. Most devices don't send services on
            // advertisement.
            if !name.is_empty() && !tried_names.contains(&name) {
              tried_names.push(name.clone());
              let device_creator = Box::new(BtlePlugDeviceImplCreator::new(p, central.clone()));
              device_sender
                .send(DeviceCommunicationEvent::DeviceFound(device_creator))
                .await;
            }
          }
        }
      }
      central.stop_scan().unwrap();
      info!("Exiting rumble scanning");
    });
    Ok(())
  })
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    // TODO This changes struct state and isn't consistent with expectations
    if self.scanning_sender.is_some() {
      let sender = self.scanning_sender.take().unwrap();
      Box::pin(async move {
        sender.send(false).await;
        Ok(())
      })
    } else {
      ButtplugDeviceError::new("Scanning not currently happening.").into()
    }
  }

  fn is_scanning(&self) -> bool {
    false
  }
}

impl Drop for BtlePlugCommunicationManager {
  fn drop(&mut self) {
    info!("Dropping Comm Manager!");
    task::block_on(async {
      if let Err(e) = self.stop_scanning().await {
        error!("Error stopping scanning during comm manager drop: {:?}", e);
      }
    });
  }
}

#[cfg(test)]
mod test {
  use super::BtlePlugCommunicationManager;
  use crate::server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerCreator,
  };
  use async_std::{prelude::StreamExt, sync::channel, task};

  #[test]
  #[ignore]
  pub fn test_rumble() {
    let _ = env_logger::builder().is_test(true).try_init();
    task::block_on(async move {
      let (sender, mut receiver) = channel(256);
      let mut mgr = BtlePlugCommunicationManager::new(sender);
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
