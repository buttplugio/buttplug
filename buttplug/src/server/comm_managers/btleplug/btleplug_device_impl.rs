use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugUnknownError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition},
    ButtplugDeviceCommand, ButtplugDeviceEvent, ButtplugDeviceImplCreator, ButtplugDeviceReturn,
    DeviceImpl, DeviceImplInternal, DeviceReadCmd, DeviceSubscribeCmd, DeviceUnsubscribeCmd,
    DeviceWriteCmd, Endpoint
  },
  server::comm_managers::ButtplugDeviceSpecificError,
  util::async_manager,
};
use async_trait::async_trait;
use btleplug::{api::{BDAddr, CentralEvent, Characteristic, Peripheral, WriteType, ValueNotification}, platform::Manager};
use uuid::Uuid;
use futures::{TryFutureExt, Stream, StreamExt, future::{self, BoxFuture}};
use std::{
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  collections::HashMap,
  pin::Pin
};
use tokio::sync::{broadcast, mpsc};
use tracing_futures::Instrument;

pub struct BtlePlugDeviceImplCreator<T: Peripheral + 'static> {
  name: String,
  address: BDAddr,
  manager: Manager,
  device: T,
}

impl<T: Peripheral> BtlePlugDeviceImplCreator<T> {
  pub fn new(name: &str, address: &BDAddr, manager: Manager, device: T) -> Self {
    Self {
      name: name.to_owned(),
      address: address.to_owned(),
      device,
      manager,
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
    DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(&self.name))
  }

  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDefinition,
  ) -> Result<DeviceImpl, ButtplugError> {
    if let Err(err) = self.device.connect().await {
      let return_err = ButtplugDeviceError::DeviceSpecificError(
        ButtplugDeviceSpecificError::BtleplugError(format!("{:?}", err)),
      );
      return Err(return_err.into());
    }
    // Map UUIDs to endpoints
    let mut uuid_map = HashMap::<Uuid, Endpoint>::new();
    let mut endpoints = HashMap::<Endpoint, Characteristic>::new();
    let chars = match self.device.discover_characteristics().await {
      Ok(chars) => chars,
      Err(err) => {
        error!("BTLEPlug error discovering characteristics: {:?}", err);
        return Err(
          ButtplugDeviceError::DeviceConnectionError(format!(
            "BTLEPlug error discovering characteristics: {:?}",
            err
          ))
          .into(),
        );
      }
    };
    for proto_service in protocol.btle.unwrap().services.values() {
      for (chr_name, chr_uuid) in proto_service.iter() {
        let maybe_chr = chars.iter().find(|c| c.uuid == *chr_uuid);
        if let Some(chr) = maybe_chr {
          endpoints.insert(*chr_name, chr.clone());
          uuid_map.insert(*chr_uuid, *chr_name);
        }
      }
    }
    let notification_stream = self.device.notifications().await.unwrap();
    let device_internal_impl = BtlePlugDeviceImpl::new(self.device.clone(), self.address, notification_stream, endpoints.clone(), uuid_map);
    let device_impl = DeviceImpl::new(
      &self.name,
      &self.address.to_string(),
      &endpoints.keys().cloned().collect::<Vec<Endpoint>>(),
    Box::new(device_internal_impl),
    );
    Ok(device_impl)
  }
}


//#[derive(Clone)]
pub struct BtlePlugDeviceImpl<T: Peripheral + 'static> {
  device: T,
  event_stream: broadcast::Sender<ButtplugDeviceEvent>,
  connected: Arc<AtomicBool>,
  endpoints: HashMap<Endpoint, Characteristic>
}

unsafe impl<T: Peripheral + 'static> Send for BtlePlugDeviceImpl<T> {}
unsafe impl<T: Peripheral + 'static> Sync for BtlePlugDeviceImpl<T> {}

impl<T: Peripheral + 'static> BtlePlugDeviceImpl<T> {
  pub fn new(
    device: T,
    address: BDAddr,
    mut notification_stream: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    endpoints: HashMap<Endpoint, Characteristic>,
    uuid_map: HashMap<Uuid, Endpoint>,  
  ) -> Self {
    let (event_stream, _) = broadcast::channel(256);
    let event_stream_clone = event_stream.clone();
    async_manager::spawn(async move {
      let mut error_notification = false;
      while let Some(notification) = notification_stream.next().await {
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
        if let Err(err) = event_stream_clone.send(ButtplugDeviceEvent::Notification(
          address.to_string(),
          endpoint,
          notification.value,
        )) {
          error!(
            "Cannot send notification, device object disappeared: {:?}",
            err
          );
          return;
        }
      }
    });
    Self {
      device,
      endpoints,
      connected: Arc::new(AtomicBool::new(true)),
      event_stream
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
      device.disconnect().await;
      Ok(())
    })
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return Box::pin(future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into())));
      }
    };
    let device = self.device.clone();
    let write_type = if msg.write_with_response {
      WriteType::WithResponse
    } else {
      WriteType::WithoutResponse
    };
    Box::pin(async move {
      device.write(&characteristic, &msg.data, write_type).await.unwrap();
      Ok(())
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
        return Box::pin(future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into())));
      }
    };
    let device = self.device.clone();
    Box::pin(async move {
      match device.read(&characteristic).await {
        Ok(data) => {
          trace!("Got reading: {:?}", data);
          Ok(RawReading::new(
            0,
            msg.endpoint,
            data,
          ))
        }
        Err(err) => {
          error!("BTLEPlug device read error: {:?}", err);
          Err(ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(
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
        return Box::pin(future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into())));
      }
    };
    let device = self.device.clone();
    Box::pin(async move {
      device.subscribe(&characteristic).await.map_err(|e| ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(format!("{:?}", e))).into())
    })
  }

  fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    let characteristic = match self.endpoints.get(&msg.endpoint) {
      Some(chr) => chr.clone(),
      None => {
        return Box::pin(future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into())));
      }
    };
    let device = self.device.clone();
    Box::pin(async move {
      device.unsubscribe(&characteristic).await.map_err(|e| ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(format!("{:?}", e))).into())
    })
  }
}
