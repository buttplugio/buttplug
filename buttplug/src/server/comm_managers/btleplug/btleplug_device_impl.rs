use super::btleplug_internal::{
  BtlePlugInternalEventLoop,
  DeviceReturnFuture,
  DeviceReturnStateShared,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugUnknownError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition},
    ButtplugDeviceCommand,
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    ButtplugDeviceReturn,
    DeviceImpl,
    DeviceImplInternal,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
  },
  util::async_manager,
};
use async_trait::async_trait;
use btleplug::api::{CentralEvent, Peripheral};
use futures::future::BoxFuture;
use std::{
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{broadcast, mpsc};

pub struct BtlePlugDeviceImplCreator<T: Peripheral + 'static> {
  device: Option<T>,
  broadcaster: broadcast::Sender<CentralEvent>,
}

impl<T: Peripheral> BtlePlugDeviceImplCreator<T> {
  pub fn new(device: T, broadcaster: broadcast::Sender<CentralEvent>) -> Self {
    Self {
      device: Some(device),
      broadcaster,
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
  ) -> Result<DeviceImpl, ButtplugError> {
    if self.device.is_none() {
      return Err(
        ButtplugDeviceError::DeviceConnectionError(
          "Cannot call try_create_device_impl twice!".to_owned(),
        )
        .into(),
      );
    }
    let device = self.device.take().unwrap();
    if let Some(ref proto) = protocol.btle {
      let (device_sender, device_receiver) = mpsc::channel(256);
      let name = device.properties().local_name.unwrap();
      let address = device.properties().address.to_string();
      let (device_event_sender, _) = broadcast::channel(256);
      // rumble calls, so this will block whatever thread it's spawned to.
      let mut event_loop = BtlePlugInternalEventLoop::new(
        self.broadcaster.subscribe(),
        device,
        proto.clone(),
        device_receiver,
        device_event_sender.clone(),
      );
      async_manager::spawn(async move { event_loop.run().await }).unwrap();
      let fut = DeviceReturnFuture::default();
      let waker = fut.get_state_clone();
      if device_sender
        .send((ButtplugDeviceCommand::Connect, waker))
        .await
        .is_err()
      {
        return Err(
          ButtplugDeviceError::DeviceConnectionError(
            "Event loop exited before we could connect.".to_owned(),
          )
          .into(),
        );
      };
      match fut.await {
        ButtplugDeviceReturn::Connected(info) => {
          let device_internal_impl = BtlePlugDeviceImpl::new(device_sender, device_event_sender);
          let device_impl = DeviceImpl::new(
            &name,
            &address,
            &info.endpoints,
            Box::new(device_internal_impl),
          );
          Ok(device_impl)
        }
        // TODO It'd be nice to carry this error through as a source.
        ButtplugDeviceReturn::Error(err) => Err(
          ButtplugDeviceError::DeviceConnectionError(format!(
            "Device connection failed: {:?}",
            err
          ))
          .into(),
        ),
        other => Err(ButtplugUnknownError::UnexpectedType(format!("{:?}", other)).into()),
      }
    } else {
      Err(
        ButtplugDeviceError::DeviceConnectionError(
          "Got a protocol with no Bluetooth Definition!".to_owned(),
        )
        .into(),
      )
    }
  }
}

//#[derive(Clone)]
pub struct BtlePlugDeviceImpl {
  event_stream: broadcast::Sender<ButtplugDeviceEvent>,
  thread_sender: mpsc::Sender<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
  connected: Arc<AtomicBool>,
}

unsafe impl Send for BtlePlugDeviceImpl {
}
unsafe impl Sync for BtlePlugDeviceImpl {
}

impl BtlePlugDeviceImpl {
  pub fn new(
    thread_sender: mpsc::Sender<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    event_stream: broadcast::Sender<ButtplugDeviceEvent>,
  ) -> Self {
    Self {
      thread_sender,
      connected: Arc::new(AtomicBool::new(true)),
      event_stream,
    }
  }

  fn send_to_device_task(
    &self,
    cmd: ButtplugDeviceCommand,
  ) -> ButtplugResultFuture<ButtplugDeviceReturn> {
    let sender = self.thread_sender.clone();
    Box::pin(async move {
      let fut = DeviceReturnFuture::default();
      let waker = fut.get_state_clone();
      if sender.send((cmd, waker)).await.is_err() {
        error!("Device event loop shut down, cannot send command.");
        return Err(
          ButtplugDeviceError::DeviceNotConnected(
            "Device event loop shut down, cannot send command.".to_owned(),
          )
          .into(),
        );
      }
      Ok(fut.await)
    })
  }

  fn send_to_device_expect_ok(
    &self,
    cmd: ButtplugDeviceCommand,
    err_str: &str,
  ) -> ButtplugResultFuture {
    let fut = self.send_to_device_task(cmd);
    let err_fut_str = err_str.to_owned();
    Box::pin(async move {
      match fut.await? {
        ButtplugDeviceReturn::Ok(_) => Ok(()),
        ButtplugDeviceReturn::Error(e) => {
          let err_out = format!("{}: {:?}", err_fut_str, e);
          error!("{}", err_out);
          // TODO Need to whittle down what this error actually means.
          Err(ButtplugDeviceError::DeviceCommunicationError(err_out).into())
        }
        other => {
          Err(ButtplugUnknownError::UnexpectedType(format!("{}: {:?}", err_fut_str, other)).into())
        }
      }
    })
  }
}

impl DeviceImplInternal for BtlePlugDeviceImpl {
  fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.event_stream.subscribe()
  }

  fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    self.send_to_device_expect_ok(
      ButtplugDeviceCommand::Disconnect,
      "Cannot disconnect device",
    )
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    self.send_to_device_expect_ok(
      ButtplugDeviceCommand::Message(msg.into()),
      "Cannot write to endpoint",
    )
  }

  fn read_value(
    &self,
    msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    // Right now we only need read for doing a whitelist check on devices. We
    // don't care about the data we get back.
    let task = self.send_to_device_task(ButtplugDeviceCommand::Message(msg.into()));
    Box::pin(async move {
      let val = task.await?;
      if let ButtplugDeviceReturn::RawReading(reading) = val {
        Ok(reading)
      } else {
        Err(
          ButtplugUnknownError::UnexpectedType(format!(
            "Read Error, unexpected return type: {:?}",
            val
          ))
          .into(),
        )
      }
    })
  }

  fn subscribe(&self, msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    self.send_to_device_expect_ok(
      ButtplugDeviceCommand::Message(msg.into()),
      "Cannot subscribe",
    )
  }

  fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    self.send_to_device_expect_ok(
      ButtplugDeviceCommand::Message(msg.into()),
      "Cannot unsubscribe",
    )
  }
}
