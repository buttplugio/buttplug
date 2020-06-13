use super::btleplug_internal::{
  BtlePlugInternalEventLoop, DeviceReturnFuture, DeviceReturnStateShared,
};
use crate::{
  core::{
    ButtplugResultFuture,
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
  },
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition},
    BoundedDeviceEventBroadcaster, ButtplugDeviceCommand, ButtplugDeviceImplCreator,
    ButtplugDeviceReturn, DeviceImpl, DeviceReadCmd, DeviceSubscribeCmd, DeviceUnsubscribeCmd,
    DeviceWriteCmd, Endpoint,
  },
  util::async_manager,
};
use async_channel::{bounded, Sender};
use async_trait::async_trait;
use broadcaster::BroadcastChannel;
use btleplug::api::{Central, Peripheral};
use futures::future::BoxFuture;

pub struct BtlePlugDeviceImplCreator<T: Peripheral + 'static, C: Central<T> + 'static> {
  device: Option<T>,
  central: C,
}

impl<T: Peripheral, C: Central<T>> BtlePlugDeviceImplCreator<T, C> {
  pub fn new(device: T, central: C) -> Self {
    Self {
      device: Some(device),
      central,
    }
  }
}

#[async_trait]
impl<T: Peripheral, C: Central<T>> ButtplugDeviceImplCreator for BtlePlugDeviceImplCreator<T, C> {
  fn get_specifier(&self) -> DeviceSpecifier {
    if self.device.is_none() {
      panic!("Cannot call get_specifier after device is taken!");
    }
    let name = self
      .device
      .as_ref()
      .unwrap()
      .properties()
      .local_name
      .unwrap();
    DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(&name))
  }

  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDefinition,
  ) -> Result<Box<dyn DeviceImpl>, ButtplugError> {
    // TODO ugggggggh there's gotta be a way to ensure this at compile time.
    if self.device.is_none() {
      panic!("Cannot call try_create_device_impl twice!");
    }
    let device = self.device.take().unwrap();
    if let Some(ref proto) = protocol.btle {
      let (device_sender, device_receiver) = bounded(256);
      let output_broadcaster = BroadcastChannel::with_cap(256);
      let p = proto.clone();
      let name = device.properties().local_name.unwrap();
      let address = device.properties().address.to_string();
      // TODO This is not actually async. We're currently using blocking
      let central = self.central.clone();
      // rumble calls, so this will block whatever thread it's spawned to.
      let broadcaster_clone = output_broadcaster.clone();
      let mut event_loop =
        BtlePlugInternalEventLoop::new(central, device, p, device_receiver, broadcaster_clone);
      async_manager::spawn(async move { event_loop.run().await }).unwrap();
      let fut = DeviceReturnFuture::default();
      let waker = fut.get_state_clone();
      if device_sender
        .send((ButtplugDeviceCommand::Connect, waker))
        .await.is_err() {
          return Err(ButtplugDeviceError::new("Event loop exited before we could connect.").into());
      };
      match fut.await {
        ButtplugDeviceReturn::Connected(info) => Ok(Box::new(BtlePlugDeviceImpl::new(
          &name,
          &address,
          info.endpoints,
          device_sender,
          output_broadcaster.clone(),
        ))),
        _ => Err(ButtplugError::ButtplugDeviceError(
          ButtplugDeviceError::new("Cannot connect"),
        )),
      }
    } else {
      panic!("Got a protocol with no Bluetooth Definition!");
    }
  }
}

#[derive(Clone)]
pub struct BtlePlugDeviceImpl {
  name: String,
  address: String,
  endpoints: Vec<Endpoint>,
  thread_sender: Sender<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
  event_receiver: BoundedDeviceEventBroadcaster,
}

unsafe impl Send for BtlePlugDeviceImpl {}
unsafe impl Sync for BtlePlugDeviceImpl {}

impl BtlePlugDeviceImpl {
  pub fn new(
    name: &str,
    address: &str,
    endpoints: Vec<Endpoint>,
    thread_sender: Sender<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    event_receiver: BoundedDeviceEventBroadcaster,
  ) -> Self {
    Self {
      name: name.to_string(),
      address: address.to_string(),
      endpoints,
      thread_sender,
      event_receiver,
    }
  }

  fn send_to_device_task(
    &self,
    cmd: ButtplugDeviceCommand,
    err_msg: &str,
  ) -> ButtplugResultFuture {
    let sender = self.thread_sender.clone();
    let msg = err_msg.to_owned();
    Box::pin(async move {
      let fut = DeviceReturnFuture::default();
      let waker = fut.get_state_clone();
      if sender.send((cmd, waker)).await.is_err() {
        error!("Device event loop shut down, cannot send command.");
        return Err(ButtplugError::ButtplugDeviceError(
          ButtplugDeviceError::new("Device event loop shut down, cannot send command.")
        ));
      }
      match fut.await {
        ButtplugDeviceReturn::Ok(_) => Ok(()),
        ButtplugDeviceReturn::Error(e) => {
          error!("{:?}", e);
          Err(ButtplugError::ButtplugDeviceError(
            ButtplugDeviceError::new(&e.to_string()),
          ))
        }
        _ => Err(ButtplugError::ButtplugDeviceError(
          ButtplugDeviceError::new(&msg),
        )),
      }
    })
  }
}

impl DeviceImpl for BtlePlugDeviceImpl {
  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    self.event_receiver.clone()
  }

  fn name(&self) -> &str {
    &self.name
  }

  fn address(&self) -> &str {
    &self.address
  }

  fn connected(&self) -> bool {
    // TODO Should figure out how we wanna deal with this across the
    // representation and inner loop.
    true
  }

  fn endpoints(&self) -> Vec<Endpoint> {
    self.endpoints.clone()
  }

  fn disconnect(&self) -> ButtplugResultFuture {
      self.send_to_device_task(
        ButtplugDeviceCommand::Disconnect,
        "Cannot disconnect device",
      )
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    self
      .send_to_device_task(
        ButtplugDeviceCommand::Message(msg.into()),
        "Cannot write to endpoint",
      )
  }

  fn read_value(&self, _msg: DeviceReadCmd) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    // TODO Actually implement value reading
    unimplemented!("Shouldn't get here!")
  }

  fn subscribe(&self, msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    self
      .send_to_device_task(
        ButtplugDeviceCommand::Message(msg.into()),
        "Cannot subscribe",
      )
  }

  fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    self
      .send_to_device_task(
        ButtplugDeviceCommand::Message(msg.into()),
        "Cannot unsubscribe",
      )
  }
}
