use crate::server::comm_managers::DeviceCommunicationEvent;
use btleplug::{
  api::{Central, Manager as _, CentralEvent, Peripheral},
  platform::Manager,
};
use futures::{
  future::{BoxFuture, FutureExt},
  StreamExt,
};
use super::btleplug_device_impl::BtlePlugDeviceImplCreator;
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

  pub async fn run(&mut self) {
    let manager = match Manager::new().await {
      Ok(mgr) => mgr,
      Err(e) => {
        error!("Error creating btleplug manager: {:?}", e);
        return;
      }
    };
    let adapter = match manager.adapters().await {
      Ok(adapters) => adapters.into_iter().nth(0).unwrap(),
      Err(e) => {
        error!("Error retreiving BTLE adapters: {:?}", e);
        return;
      }
    };

    let mut events = adapter.events().await.unwrap();

    let mut tried_addresses = vec!();

    loop {
      select! {
        event = events.next().fuse() => {
          match event.unwrap() {
            CentralEvent::DeviceDiscovered(bd_addr) => {
              let peripheral = adapter.peripheral(bd_addr).await.unwrap();
              // If a device has no discernable name, we can't do anything
              // with it, just ignore it.
              let properties = peripheral.properties().await.unwrap();
              if let Some(name) = properties.local_name {
                let span = info_span!(
                  "btleplug enumeration",
                  address = tracing::field::display(properties.address),
                  name = tracing::field::display(&name)
                );
                let _enter = span.enter();
                debug!("Found device {}", name);
                // Names are the only way we really have to test devices
                // at the moment. Most devices don't send services on
                // advertisement.
                if !name.is_empty()
                  && !tried_addresses.contains(&properties.address)
                  //&& !connected_addresses_handler.contains_key(&properties.address)
                {
                  let address = properties.address;
                  debug!("Found new bluetooth device: {} {}", name, address);
                  tried_addresses.push(address);

                  let device_creator = Box::new(BtlePlugDeviceImplCreator::new(
                    &name,
                    &properties.address,
                    manager.clone(),
                    peripheral.clone()
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
                    return;
                  }
                }
              } else {
                trace!(
                  "Device {} found, no advertised name, ignoring.",
                  properties.address
                );
              }
            }
            _ => {}
          }
        },
        command = self.command_receiver.recv().fuse() => {
          if let Some(cmd) = command {
            match cmd {
              BtleplugAdapterCommand::StartScanning => {
                tried_addresses.clear();
                adapter.start_scan().await.unwrap();
              }
              BtleplugAdapterCommand::StopScanning => adapter.stop_scan().await.unwrap(),
            }
          }
        }
      }
    }
  }
}
