// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::hardware::communication::HardwareSpecificError,
  server::device::{
    configuration::{BluetoothLESpecifier, ProtocolCommunicationSpecifier},
    hardware::{
      Hardware,
      HardwareConnector,
      HardwareEvent,
      HardwareInternal,
      HardwareReadCmd,
      HardwareReading,
      HardwareSpecializer,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
  },
  util::async_manager,
};
use async_trait::async_trait;
use btleplug::api::CharPropFlags;
use btleplug::{
  api::{Central, CentralEvent, Characteristic, Peripheral, ValueNotification, WriteType},
  platform::Adapter,
};
use dashmap::DashSet;
use futures::{
  future::{self, BoxFuture, FutureExt},
  Stream,
  StreamExt,
};
use std::{
  collections::HashMap,
  fmt::{self, Debug},
  pin::Pin,
  sync::Arc,
};
use tokio::sync::broadcast;
use uuid::Uuid;

pub(super) struct BtleplugHardwareConnector<T: Peripheral + 'static> {
  // Passed in and stored as a member because otherwise it's annoying to get (properties require await)
  name: String,
  // Passed in and stored as a member because otherwise it's annoying to get (properties require await)
  manufacturer_data: HashMap<u16, Vec<u8>>,
  // Passed in and stored as a member because otherwise it's annoying to get (properties require await)
  services: Vec<Uuid>,
  device: T,
  adapter: Adapter,
  requires_keepalive: bool,
}

impl<T: Peripheral> BtleplugHardwareConnector<T> {
  pub fn new(
    name: &str,
    manufacturer_data: &HashMap<u16, Vec<u8>>,
    services: &[Uuid],
    device: T,
    adapter: Adapter,
    requires_keepalive: bool,
  ) -> Self {
    Self {
      name: name.to_owned(),
      manufacturer_data: manufacturer_data.clone(),
      services: services.to_vec(),
      device,
      adapter,
      requires_keepalive,
    }
  }
}

impl<T: Peripheral> Debug for BtleplugHardwareConnector<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("BtleplugHardwareCreator")
      .field("name", &self.name)
      .field("address", &self.device.id())
      .finish()
  }
}

#[async_trait]
impl<T: Peripheral> HardwareConnector for BtleplugHardwareConnector<T> {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      &self.name,
      &self.manufacturer_data,
      &self.services,
    ))
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    if !self
      .device
      .is_connected()
      .await
      .expect("If we crash here it's Bluez's fault. Use something else please.")
    {
      if let Err(err) = self.device.connect().await {
        let return_err = ButtplugDeviceError::DeviceSpecificError(
          HardwareSpecificError::BtleplugError(format!("{:?}", err)),
        );
        return Err(return_err);
      }
      if let Err(err) = self.device.discover_services().await {
        error!("BTLEPlug error discovering characteristics: {:?}", err);
        return Err(ButtplugDeviceError::DeviceConnectionError(format!(
          "BTLEPlug error discovering characteristics: {:?}",
          err
        )));
      }
    }
    Ok(Box::new(BtleplugHardwareSpecializer::new(
      &self.name,
      self.device.clone(),
      self.adapter.clone(),
      self.requires_keepalive,
    )))
  }
}

pub struct BtleplugHardwareSpecializer<T: Peripheral + 'static> {
  name: String,
  device: T,
  adapter: Adapter,
  requires_keepalive: bool,
}

impl<T: Peripheral> BtleplugHardwareSpecializer<T> {
  pub(super) fn new(name: &str, device: T, adapter: Adapter, requires_keepalive: bool) -> Self {
    Self {
      name: name.to_owned(),
      device,
      adapter,
      requires_keepalive,
    }
  }
}

#[async_trait]
impl<T: Peripheral> HardwareSpecializer for BtleplugHardwareSpecializer<T> {
  async fn specialize(
    &mut self,
    specifiers: &[ProtocolCommunicationSpecifier],
  ) -> Result<Hardware, ButtplugDeviceError> {
    // Map UUIDs to endpoints
    let mut uuid_map = HashMap::<Uuid, Endpoint>::new();
    let mut endpoints = HashMap::<Endpoint, Characteristic>::new();
    let address = self.device.id();

    if let Some(ProtocolCommunicationSpecifier::BluetoothLE(btle)) = specifiers
      .iter()
      .find(|x| matches!(x, ProtocolCommunicationSpecifier::BluetoothLE(_)))
    {
      for (proto_uuid, proto_service) in btle.services() {
        for service in self.device.services() {
          if service.uuid != *proto_uuid {
            continue;
          }

          debug!("Found required service {} {:?}", service.uuid, service);
          for (chr_name, chr_uuid) in proto_service.iter() {
            if let Some(chr) = service.characteristics.iter().find(|c| c.uuid == *chr_uuid) {
              debug!(
                "Found characteristic {} for endpoint {}",
                chr.uuid, *chr_name
              );
              endpoints.insert(*chr_name, chr.clone());
              uuid_map.insert(*chr_uuid, *chr_name);
            } else {
              error!(
                "Characteristic {} ({}) not found, may cause issues in connection.",
                chr_name, chr_uuid
              );
            }
          }
        }
      }
    } else {
      error!(
        "Can't find btle protocol specifier mapping for device {} {:?}",
        self.name, address
      );
      return Err(ButtplugDeviceError::DeviceConnectionError(format!(
        "Can't find btle protocol specifier mapping for device {} {:?}",
        self.name, address
      )));
    }
    let notification_stream = self
      .device
      .notifications()
      .await
      .expect("Should always be able to get notifications");

    let device_internal_impl = BtlePlugHardware::new(
      self.device.clone(),
      &self.name,
      self
        .adapter
        .events()
        .await
        .expect("Should always be able to get events"),
      notification_stream,
      endpoints.clone(),
      uuid_map,
    );
    let mut hardware = Hardware::new(
      &self.name,
      &format!("{:?}", address),
      &endpoints.keys().cloned().collect::<Vec<Endpoint>>(),
      Box::new(device_internal_impl),
    );

    // Let the hardware know if we need command resends or whatever. Fucking iOS.
    if self.requires_keepalive {
      hardware.set_requires_keepalive();
    }
    Ok(hardware)
  }
}

pub struct BtlePlugHardware<T: Peripheral + 'static> {
  device: T,
  event_stream: broadcast::Sender<HardwareEvent>,
  endpoints: HashMap<Endpoint, Characteristic>,
  subscribed_endpoints: Arc<DashSet<Endpoint>>,
}

impl<T: Peripheral + 'static> BtlePlugHardware<T> {
  pub fn new(
    device: T,
    name: &str,
    mut adapter_event_stream: Pin<Box<dyn Stream<Item = CentralEvent> + Send>>,
    mut notification_stream: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    endpoints: HashMap<Endpoint, Characteristic>,
    uuid_map: HashMap<Uuid, Endpoint>,
  ) -> Self {
    let (event_stream, _) = broadcast::channel(256);
    let event_stream_clone = event_stream.clone();
    let address = device.id();
    let name_clone = name.to_owned();
    async_manager::spawn(async move {
      let mut error_notification = false;
      loop {
        select! {
          notification = notification_stream.next().fuse() => {
            if let Some(notification) = notification {
              let endpoint = if let Some(endpoint) = uuid_map.get(&notification.uuid) {
                *endpoint
              } else {
                // Only print the error message once.
                if !error_notification {
                  error!(
                    "Endpoint for UUID {} not found in map, assuming device has disconnected.",
                    notification.uuid
                  );
                  error_notification = true;
                }
                continue;
              };
              if event_stream_clone.receiver_count() == 0 {
                continue;
              }
              if let Err(err) = event_stream_clone.send(HardwareEvent::Notification(
                format!("{:?}", address),
                endpoint,
                notification.value,
              )) {
                error!(
                  "Cannot send notification, device object disappeared: {:?}",
                  err
                );
                break;
              }
            }
          }
          adapter_event = adapter_event_stream.next().fuse() => {
            if let Some(CentralEvent::DeviceDisconnected(addr)) = adapter_event {
              if address == addr {
                info!(
                  "Device {:?} disconnected",
                  name_clone
                );
                if event_stream_clone.receiver_count() != 0 {
                  if let Err(err) = event_stream_clone
                  .send(HardwareEvent::Disconnected(
                    format!("{:?}", address)
                  )) {
                    error!(
                      "Cannot send notification, device object disappeared: {:?}",
                      err
                    );
                  }
                }
                // At this point, we have nothing left to do because we can't reconnect a device
                // that's been connected. Exit.
                break;
              }
            }
          }
        }
      }
      info!(
        "Exiting btleplug notification/event loop for device {:?}",
        address
      )
    });
    Self {
      device,
      endpoints,
      event_stream,
      subscribed_endpoints: Arc::new(DashSet::new()),
    }
  }
}

impl<T: Peripheral + 'static> HardwareInternal for BtlePlugHardware<T> {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_stream.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let device = self.device.clone();
    async move {
      let _ = device.disconnect().await;
      Ok(())
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint))).boxed();
      }
    };

    let device = self.device.clone();
    let mut write_type = if msg.write_with_response {
      WriteType::WithResponse
    } else {
      WriteType::WithoutResponse
    };

    if (write_type == WriteType::WithoutResponse
      && (characteristic.properties & CharPropFlags::WRITE_WITHOUT_RESPONSE)
        != CharPropFlags::WRITE_WITHOUT_RESPONSE)
      || (write_type == WriteType::WithResponse
        && (characteristic.properties & CharPropFlags::WRITE) != CharPropFlags::WRITE)
    {
      if write_type == WriteType::WithoutResponse
        && (characteristic.properties & CharPropFlags::WRITE) == CharPropFlags::WRITE
      {
        warn!("BTLEPlug device doesn't support write-without-response! Falling back to write-with-response!");
        write_type = WriteType::WithResponse
      } else if write_type == WriteType::WithResponse
        && (characteristic.properties & CharPropFlags::WRITE_WITHOUT_RESPONSE)
          == CharPropFlags::WRITE_WITHOUT_RESPONSE
      {
        warn!("BTLEPlug device doesn't support write-with-response! Falling back to write-without-response!");
        write_type = WriteType::WithoutResponse
      } else {
        error!(
          "BTLEPlug device doesn't support {}! No fallback available!",
          if write_type == WriteType::WithoutResponse {
            "write-without-response"
          } else {
            "write-with-response"
          }
        );
      }
    }

    let data = msg.data.clone();
    async move {
      match device.write(&characteristic, &data, write_type).await {
        Ok(()) => {
          trace!(
            "Sent write: {:?}, {:?} to {:?}",
            data,
            write_type,
            characteristic
          );
          Ok(())
        }
        Err(err) => {
          error!("BTLEPlug device write error: {:?}", err);
          Err(ButtplugDeviceError::DeviceSpecificError(
            HardwareSpecificError::BtleplugError(format!("{:?}", err)),
          ))
        }
      }
    }
    .boxed()
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    // Right now we only need read for doing a whitelist check on devices. We
    // don't care about the data we get back.
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint))).boxed();
      }
    };
    let device = self.device.clone();
    let endpoint = msg.endpoint;
    async move {
      match device.read(&characteristic).await {
        Ok(data) => {
          trace!("Got reading: {:?}", data);
          Ok(HardwareReading::new(endpoint, &data))
        }
        Err(err) => {
          error!("BTLEPlug device read error: {:?}", err);
          Err(ButtplugDeviceError::DeviceSpecificError(
            HardwareSpecificError::BtleplugError(format!("{:?}", err)),
          ))
        }
      }
    }
    .boxed()
  }

  fn subscribe(
    &self,
    msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let endpoint = msg.endpoint;
    if self.subscribed_endpoints.contains(&endpoint) {
      debug!(
        "Endpoint {} already subscribed, ignoring and returning Ok.",
        endpoint
      );
      return future::ready(Ok(())).boxed();
    }
    let characteristic = match self.endpoints.get(&endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint))).boxed();
      }
    };
    let endpoints = self.subscribed_endpoints.clone();
    let device = self.device.clone();
    async move {
      device.subscribe(&characteristic).await.map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(HardwareSpecificError::BtleplugError(format!(
          "{:?}",
          e
        )))
      })?;
      endpoints.insert(endpoint);
      Ok(())
    }
    .boxed()
  }

  fn unsubscribe(
    &self,
    msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let endpoint = msg.endpoint;
    if !self.subscribed_endpoints.contains(&endpoint) {
      debug!(
        "Endpoint {} already unsubscribed, ignoring and returning Ok.",
        endpoint
      );
      return future::ready(Ok(())).boxed();
    }
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint))).boxed();
      }
    };
    let endpoints = self.subscribed_endpoints.clone();
    let device = self.device.clone();
    async move {
      device.unsubscribe(&characteristic).await.map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(HardwareSpecificError::BtleplugError(format!(
          "{:?}",
          e
        )))
      })?;
      endpoints.remove(&endpoint);
      Ok(())
    }
    .boxed()
  }
}

impl<T: Peripheral> Drop for BtlePlugHardware<T> {
  fn drop(&mut self) {
    let disconnect_fut = self.disconnect();
    async_manager::spawn(async move {
      if let Err(e) = disconnect_fut.await {
        error!("Error disconnecting btleplug device: {:?}", e);
      }
    });
  }
}
