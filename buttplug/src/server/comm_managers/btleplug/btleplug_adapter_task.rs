use super::btleplug_device_impl::BtlePlugDeviceImplCreator;
use crate::server::comm_managers::DeviceCommunicationEvent;
use btleplug::{
  api::{BDAddr, Central, CentralEvent, Manager as _, Peripheral},
  platform::{Adapter, Manager},
};
use futures::{future::FutureExt, StreamExt};
use futures_timer::Delay;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone, Copy)]
pub enum BtleplugAdapterCommand {
  StartScanning,
  StopScanning,
}

pub struct BtleplugAdapterTask {
  event_sender: Sender<DeviceCommunicationEvent>,
  command_receiver: Receiver<BtleplugAdapterCommand>,

}

impl BtleplugAdapterTask {
  pub fn new(
    event_sender: Sender<DeviceCommunicationEvent>,
    command_receiver: Receiver<BtleplugAdapterCommand>,
  ) -> Self {
    Self {
      event_sender,
      command_receiver,
    }
  }

  async fn maybe_add_peripheral(
    &self,
    bd_addr: &BDAddr,
    adapter: &Adapter,
    tried_addresses: &mut Vec<BDAddr>,
  ) {
    let peripheral = if let Ok(peripheral) = adapter.peripheral(*bd_addr).await {
      peripheral
    } else {
      error!("Peripheral with address {} not found.", bd_addr);
      return;
    };
    // If a device has no discernable name, we can't do anything
    // with it, just ignore it.
    let properties = if let Ok(Some(properties)) = peripheral.properties().await {
      properties
    } else {
      error!("Cannot retreive peripheral properties for {}.", bd_addr);
      return;
    };
    if let Some(name) = properties.local_name {
      let span = info_span!(
        "btleplug enumeration",
        address = tracing::field::display(properties.address),
        name = tracing::field::display(&name)
      );
      let _enter = span.enter();
      // Names are the only way we really have to test devices
      // at the moment. Most devices don't send services on
      // advertisement.
      if !name.is_empty() && !tried_addresses.contains(&properties.address)
      //&& !connected_addresses_handler.contains_key(&properties.address)
      {
        let address = properties.address;
        debug!("Found new bluetooth device: {} {}", name, address);
        tried_addresses.push(address);
        let device_creator = Box::new(BtlePlugDeviceImplCreator::new(
          &name,
          &properties.address,
          peripheral.clone(),
          adapter.clone(),
        ));

        if self
          .event_sender
          .send(DeviceCommunicationEvent::DeviceFound {
            name,
            address: address.to_string(),
            creator: device_creator,
          })
          .await
          .is_err()
        {
          error!("Device manager receiver dropped, cannot send device found message.");
        }
      }
    } else {
      trace!(
        "Device {} found, no advertised name, ignoring.",
        properties.address
      );
    }
  }

  pub async fn run(&mut self) {
    let manager = match Manager::new().await {
      Ok(mgr) => mgr,
      Err(e) => {
        error!("Error creating btleplug manager: {:?}", e);
        return;
      }
    };

    // Start by assuming we'll find the adapter on the first try. If not, we'll print an error
    // message then loop while trying to find it.
    let mut adapter_found = true;

    let adapter;

    loop {
      if !adapter_found {
        Delay::new(Duration::from_secs(1)).await;
      }
      adapter = match manager.adapters().await {
        Ok(adapters) => {
          if let Some(adapter) = adapters.into_iter().nth(0) {
            info!("Bluetooth LE adapter found.");
            adapter
          } else {
            if adapter_found {
              adapter_found = false;
              warn!("Bluetooth LE adapter not found, will not be using bluetooth scanning until found. Buttplug will continue polling for the adapter, but no more warning messages will be posted.");
            }
            continue;
          }
        }
        Err(e) => {
          if adapter_found {
            adapter_found = false;
            error!("Error retreiving BTLE adapters: {:?}", e);
          }
          continue;
        }
      };
      break;
    }

    #[cfg(not(target_os = "linux"))]
    let mut events = adapter.events().await.expect("Should always be able to retreive stream.");

    let mut tried_addresses = vec![];

    loop {
      #[cfg(target_os = "linux")]
      let event_fut = Delay::new(Duration::from_secs(2));
      #[cfg(not(target_os = "linux"))]
      let event_fut = events.next();

      select! {
        event = event_fut.fuse() => {
          #[cfg(not(target_os = "linux"))]
          {
            if let Some(event) = event {
              match event {
                CentralEvent::DeviceDiscovered(bd_addr) | CentralEvent::DeviceUpdated(bd_addr) => {
                  self.maybe_add_peripheral(&bd_addr, &adapter, &mut tried_addresses).await;
                }
                CentralEvent::DeviceDisconnected(addr) => {
                  debug!("BTLEPlug Device disconnected: {:?}", addr);
                  tried_addresses.retain(|bd_addr| addr != *bd_addr);
                }
                event => {
                  trace!("Unhandled btleplug central event: {:?}", event)
                }
              }
            } else {
              error!("Event stream closed. Exiting loop.");
              return;
            }
          }
          #[cfg(target_os = "linux")]
          {
            // We're in a macro block, so it's going to complain that event isn't used but we can't
            // set allow dead code. Therefore, we just copy the event to nothing in order to supress
            // the warning. Ew.
            let _ = event;
            let peripherals = adapter.peripherals().await.expect("Removing this once we update to btleplug v0.9");

            // All peripheral devices in range.
            for peripheral in peripherals.iter() {
              // We'll incur 2 peripheral lookups here but this isn't really a slow call so it's
              // fine.
              let properties = peripheral.properties().await.expect("Removing this once we update to btleplug v0.9").expect("Removing this once we update to btleplug v0.9");
              self.maybe_add_peripheral(&properties.address, &adapter, &mut tried_addresses).await;
            }
          }
        },
        command = self.command_receiver.recv().fuse() => {
          if let Some(cmd) = command {
            match cmd {
              BtleplugAdapterCommand::StartScanning => {
                tried_addresses.clear();
                if let Err(err) = adapter.start_scan().await {
                  error!("Start scanning request failed: {}", err);
                }
              }
              BtleplugAdapterCommand::StopScanning => {
                if let Err(err) = adapter.stop_scan().await {
                  error!("Stop scanning request failed: {}", err);
                }
              }
            }
          }
        }
      }
    }
  }
}
