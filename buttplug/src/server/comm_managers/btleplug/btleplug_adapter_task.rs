use super::btleplug_device_impl::BtlePlugDeviceImplCreator;
use crate::server::comm_managers::DeviceCommunicationEvent;
use btleplug::{
  api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter},
  platform::{Adapter, Manager, PeripheralId},
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
    peripheral_id: &PeripheralId,
    adapter: &Adapter,
    tried_addresses: &mut Vec<PeripheralId>,
  ) {
    let peripheral = if let Ok(peripheral) = adapter.peripheral(peripheral_id).await {
      peripheral
    } else {
      error!("Peripheral with address {:?} not found.", peripheral_id);
      return;
    };
    // If a device has no discernable name, we can't do anything
    // with it, just ignore it.
    let properties = if let Ok(Some(properties)) = peripheral.properties().await {
      properties
    } else {
      error!(
        "Cannot retreive peripheral properties for {:?}.",
        peripheral_id
      );
      return;
    };
    if let Some(name) = properties.local_name {
      let span = info_span!(
        "btleplug enumeration",
        address = tracing::field::display(format!("{:?}", peripheral_id)),
        name = tracing::field::display(&name)
      );
      let _enter = span.enter();
      // Names are the only way we really have to test devices
      // at the moment. Most devices don't send services on
      // advertisement.
      if !name.is_empty() && !tried_addresses.contains(peripheral_id)
      //&& !connected_addresses_handler.contains_key(&properties.address)
      {
        debug!("Found new bluetooth device: {} {:?}", name, peripheral_id);
        tried_addresses.push(peripheral_id.clone());
        let device_creator = Box::new(BtlePlugDeviceImplCreator::new(
          &name,
          peripheral_id,
          peripheral.clone(),
          adapter.clone(),
        ));

        if self
          .event_sender
          .send(DeviceCommunicationEvent::DeviceFound {
            name,
            address: format!("{:?}", peripheral_id),
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
          if let Some(adapter) = adapters.into_iter().next() {
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

    let mut events = adapter
      .events()
      .await
      .expect("Should always be able to retreive stream.");

    let mut tried_addresses = vec![];

    loop {
      let event_fut = events.next();

      select! {
        event = event_fut.fuse() => {
            if let Some(event) = event {
              match event {
                CentralEvent::DeviceDiscovered(peripheral_id) | CentralEvent::DeviceUpdated(peripheral_id) => {
                  self.maybe_add_peripheral(&peripheral_id, &adapter, &mut tried_addresses).await;
                }
                CentralEvent::DeviceDisconnected(peripheral_id) => {
                  debug!("BTLEPlug Device disconnected: {:?}", peripheral_id);
                  tried_addresses.retain(|id| peripheral_id != *id);
                }
                event => {
                  trace!("Unhandled btleplug central event: {:?}", event)
                }
              }
            } else {
              error!("Event stream closed. Exiting loop.");
              return;
            }
        },
        command = self.command_receiver.recv().fuse() => {
          if let Some(cmd) = command {
            match cmd {
              BtleplugAdapterCommand::StartScanning => {
                tried_addresses.clear();
                if let Err(err) = adapter.start_scan(ScanFilter::default()).await {
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
