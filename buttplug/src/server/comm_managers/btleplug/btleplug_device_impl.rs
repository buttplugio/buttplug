use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition},
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceImplInternal,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
  server::comm_managers::ButtplugDeviceSpecificError,
  util::async_manager,
};
use async_trait::async_trait;
use btleplug::{
  api::{Central, CentralEvent, Characteristic, Peripheral, ValueNotification, WriteType},
  platform::{Adapter, PeripheralId},
};
use futures::{
  future::{self, BoxFuture, FutureExt},
  Stream,
  StreamExt,
};
use std::{
  collections::HashMap,
  fmt::{self, Debug},
  pin::Pin,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::broadcast;
use uuid::Uuid;

pub struct BtlePlugDeviceImplCreator<T: Peripheral + 'static> {
  name: String,
  address: PeripheralId,
  services: Vec<Uuid>,
  device: T,
  adapter: Adapter,
}

impl<T: Peripheral> BtlePlugDeviceImplCreator<T> {
  pub fn new(
    name: &str,
    address: &PeripheralId,
    services: &[Uuid],
    device: T,
    adapter: Adapter,
  ) -> Self {
    Self {
      name: name.to_owned(),
      address: address.to_owned(),
      services: services.to_vec(),
      device,
      adapter,
    }
  }
}

impl<T: Peripheral> Debug for BtlePlugDeviceImplCreator<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("BtlePlugDeviceImplCreator").finish()
  }
}

#[async_trait]
impl<T: Peripheral> ButtplugDeviceImplCreator for BtlePlugDeviceImplCreator<T> {
  fn get_specifier(&self) -> DeviceSpecifier {
    DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      &self.name,
      &self.services,
    ))
  }

  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDefinition,
  ) -> Result<DeviceImpl, ButtplugError> {
    if !self
      .device
      .is_connected()
      .await
      .expect("If we crash here it's Bluez's fault. Use something else please.")
    {
      if let Err(err) = self.device.connect().await {
        let return_err = ButtplugDeviceError::DeviceSpecificError(
          ButtplugDeviceSpecificError::BtleplugError(format!("{:?}", err)),
        );
        return Err(return_err.into());
      }
    }

    // Lets get characteristics every time
    if let Err(err) = self.device.discover_services().await {
      error!("BTLEPlug error discovering characteristics: {:?}", err);
      return Err(
        ButtplugDeviceError::DeviceConnectionError(format!(
          "BTLEPlug error discovering characteristics: {:?}",
          err
        ))
        .into(),
      );
    }
    debug!(
      "Services for {:?}: {:?}",
      self.device.address(),
      self.device.services()
    );

    // Map UUIDs to endpoints
    let mut uuid_map = HashMap::<Uuid, Endpoint>::new();
    let mut endpoints = HashMap::<Endpoint, Characteristic>::new();

    let btle = protocol
      .btle()
      .as_ref()
      .expect("To get this far we are guaranteed to have a btle block in the config");

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
    for required_endpoint in btle.required_endpoints() {
      if !endpoints.contains_key(required_endpoint) {
        debug!(
          "Device {:?} missing endpoint {:?} required for protocol",
          self.device.address(),
          *required_endpoint
        );
        return Err(ButtplugDeviceError::InvalidEndpoint(*required_endpoint).into());
      }
    }

    let notification_stream = self
      .device
      .notifications()
      .await
      .expect("Should always be able to get notifications");
    let device_internal_impl = BtlePlugDeviceImpl::new(
      self.device.clone(),
      &self.name,
      self.address.clone(),
      self
        .adapter
        .events()
        .await
        .expect("Should always be able to get events"),
      notification_stream,
      endpoints.clone(),
      uuid_map,
    );
    let device_impl = DeviceImpl::new(
      &self.name,
      &format!("{:?}", self.address),
      &endpoints.keys().cloned().collect::<Vec<Endpoint>>(),
      Box::new(device_internal_impl),
    );
    Ok(device_impl)
  }
}

pub struct BtlePlugDeviceImpl<T: Peripheral + 'static> {
  device: T,
  event_stream: broadcast::Sender<ButtplugDeviceEvent>,
  connected: Arc<AtomicBool>,
  endpoints: HashMap<Endpoint, Characteristic>,
}

impl<T: Peripheral + 'static> BtlePlugDeviceImpl<T> {
  pub fn new(
    device: T,
    name: &str,
    address: PeripheralId,
    mut adapter_event_stream: Pin<Box<dyn Stream<Item = CentralEvent> + Send>>,
    mut notification_stream: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    endpoints: HashMap<Endpoint, Characteristic>,
    uuid_map: HashMap<Uuid, Endpoint>,
  ) -> Self {
    let (event_stream, _) = broadcast::channel(256);
    let event_stream_clone = event_stream.clone();
    let address_clone = address.clone();
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
              if let Err(err) = event_stream_clone.send(ButtplugDeviceEvent::Notification(
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
              if address_clone == addr {
                info!(
                  "Device {:?} disconnected",
                  name_clone
                );
                if event_stream_clone.receiver_count() != 0 {
                  if let Err(err) = event_stream_clone
                  .send(ButtplugDeviceEvent::Removed(
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
        address_clone
      )
    });
    Self {
      device,
      endpoints,
      connected: Arc::new(AtomicBool::new(true)),
      event_stream,
    }
  }
}

impl<T: Peripheral + 'static> DeviceImplInternal for BtlePlugDeviceImpl<T> {
  fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.event_stream.subscribe()
  }

  fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    let device = self.device.clone();
    Box::pin(async move {
      let _ = device.disconnect().await;
      Ok(())
    })
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return Box::pin(future::ready(Err(
          ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into(),
        )));
      }
    };
    let device = self.device.clone();
    let write_type = if msg.write_with_response {
      WriteType::WithResponse
    } else {
      WriteType::WithoutResponse
    };
    Box::pin(async move {
      match device.write(&characteristic, &msg.data, write_type).await {
        Ok(()) => Ok(()),
        Err(err) => {
          error!("BTLEPlug device write error: {:?}", err);
          Err(
            ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(
              format!("{:?}", err),
            ))
            .into(),
          )
        }
      }
    })
  }

  fn read_value(
    &self,
    msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    // Right now we only need read for doing a whitelist check on devices. We
    // don't care about the data we get back.
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return Box::pin(future::ready(Err(
          ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into(),
        )));
      }
    };
    let device = self.device.clone();
    Box::pin(async move {
      match device.read(&characteristic).await {
        Ok(data) => {
          trace!("Got reading: {:?}", data);
          Ok(RawReading::new(0, msg.endpoint, data))
        }
        Err(err) => {
          error!("BTLEPlug device read error: {:?}", err);
          Err(
            ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(
              format!("{:?}", err),
            ))
            .into(),
          )
        }
      }
    })
  }

  fn subscribe(&self, msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return Box::pin(future::ready(Err(
          ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into(),
        )));
      }
    };
    let device = self.device.clone();
    Box::pin(async move {
      device.subscribe(&characteristic).await.map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(
          format!("{:?}", e),
        ))
        .into()
      })
    })
  }

  fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return Box::pin(future::ready(Err(
          ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into(),
        )));
      }
    };
    let device = self.device.clone();
    Box::pin(async move {
      device.unsubscribe(&characteristic).await.map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(
          format!("{:?}", e),
        ))
        .into()
      })
    })
  }
}
