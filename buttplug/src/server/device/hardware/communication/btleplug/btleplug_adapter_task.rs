// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::btleplug_hardware::BtleplugHardwareConnector;
use crate::server::device::hardware::communication::HardwareCommunicationManagerEvent;
use btleplug::{
  api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter},
  platform::{Adapter, Manager, PeripheralId},
};
use futures::{future::FutureExt, StreamExt};
use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::{
  sync::mpsc::{Receiver, Sender},
  time::sleep,
};

#[derive(Debug, Clone, Copy)]
pub enum BtleplugAdapterCommand {
  StartScanning,
  StopScanning,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct PeripheralInfo {
  name: Option<String>,
  peripheral_id: PeripheralId,
  manufacturer_data: HashMap<u16, Vec<u8>>,
  services: Vec<uuid::Uuid>,
}

pub struct BtleplugAdapterTask {
  event_sender: Sender<HardwareCommunicationManagerEvent>,
  command_receiver: Receiver<BtleplugAdapterCommand>,
  adapter_connected: Arc<AtomicBool>,
  requires_keepalive: bool,
}

impl BtleplugAdapterTask {
  pub fn new(
    event_sender: Sender<HardwareCommunicationManagerEvent>,
    command_receiver: Receiver<BtleplugAdapterCommand>,
    adapter_connected: Arc<AtomicBool>,
    requires_keepalive: bool,
  ) -> Self {
    Self {
      event_sender,
      command_receiver,
      adapter_connected,
      requires_keepalive,
    }
  }

  async fn maybe_add_peripheral(
    &self,
    peripheral_id: &PeripheralId,
    adapter: &Adapter,
    tried_addresses: &mut Vec<PeripheralInfo>,
  ) {
    let peripheral = if let Ok(peripheral) = adapter.peripheral(peripheral_id).await {
      peripheral
    } else {
      error!("Peripheral with address {:?} not found.", peripheral_id);
      return;
    };
    // If a device has no discernable name, we can't do anything with it, just ignore it.
    let properties = if let Ok(Some(properties)) = peripheral.properties().await {
      properties
    } else {
      error!(
        "Cannot retreive peripheral properties for {:?}.",
        peripheral_id
      );
      return;
    };

    let device_name = if let Some(name) = &properties.local_name {
      name.clone()
    } else {
      String::new()
    };

    let peripheral_info = PeripheralInfo {
      name: properties.local_name.clone(),
      peripheral_id: peripheral_id.clone(),
      manufacturer_data: properties.manufacturer_data.clone(),
      services: properties.services.clone(),
    };

    if (!device_name.is_empty() || !properties.services.is_empty())
      && !tried_addresses.contains(&peripheral_info)
    {
      let span = info_span!(
        "btleplug enumeration",
        address = tracing::field::display(format!("{:?}", peripheral_id)),
        name = tracing::field::display(&device_name)
      );
      let _enter = span.enter();

      debug!(
        "Found new bluetooth device advertisement: {:?}",
        peripheral_info
      );
      tried_addresses.push(peripheral_info.clone());
      let device_creator = Box::new(BtleplugHardwareConnector::new(
        &device_name,
        &properties.manufacturer_data,
        &properties.services,
        peripheral.clone(),
        adapter.clone(),
        self.requires_keepalive,
      ));
      if self
        .event_sender
        .send(HardwareCommunicationManagerEvent::DeviceFound {
          name: device_name,
          address: format!("{:?}", peripheral_id),
          creator: device_creator,
        })
        .await
        .is_err()
      {
        error!("Device manager receiver dropped, cannot send device found message.");
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
    self.adapter_connected.store(true, Ordering::SeqCst);

    let adapter;

    loop {
      let adapter_found = self.adapter_connected.load(Ordering::SeqCst);
      if !adapter_found {
        sleep(Duration::from_secs(1)).await;
      }
      adapter = match manager.adapters().await {
        Ok(adapters) => {
          if let Some(adapter) = adapters.into_iter().next() {
            info!("Bluetooth LE adapter found.");
            // Bluetooth dongle identification for Windows
            #[cfg(target_os = "windows")]
            {
              use windows::Devices::Bluetooth::BluetoothAdapter;
              let adapter_result = BluetoothAdapter::GetDefaultAsync()
                .expect("If we're here, we got an adapter")
                .await;
              let adapter = adapter_result.expect("Considering infallible at this point");
              let device_id = adapter
                .DeviceId()
                .expect("Considering infallible at this point")
                .to_string();
              info!("Windows Bluetooth Adapter ID: {:?}", device_id);
              let device_manufacturer = if device_id.contains("VID_0A12") {
                "Cambridge Silicon Radio (CSR)"
              } else if device_id.contains("VID_0A5C") {
                "Broadcom"
              } else if device_id.contains("VID_8087") {
                "Intel"
              } else if device_id.contains("VID_0BDA") {
                "RealTek"
              } else if device_id.contains("VID_0B05") {
                "Asus"
              } else if device_id.contains("VID_13D3") {
                "IMC"
              } else if device_id.contains("VID_10D7") {
                "Actions Semi"
              } else {
                "Unknown Manufacturer"
              };
              info!(
                "Windows Bluetooth Adapter Manufacturer: {}",
                device_manufacturer
              );
            }
            adapter
          } else {
            if adapter_found {
              self.adapter_connected.store(false, Ordering::SeqCst);
              warn!("Bluetooth LE adapter not found, will not be using bluetooth scanning until found. Buttplug will continue polling for the adapter, but no more warning messages will be posted.");
            }
            continue;
          }
        }
        Err(e) => {
          if adapter_found {
            self.adapter_connected.store(false, Ordering::SeqCst);
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
                  tried_addresses.retain(|info| info.peripheral_id != peripheral_id);
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
          } else {
            debug!("Command stream closed. Exiting btleplug adapter loop.");
            return;
          }
        }
      }
    }
  }
}
