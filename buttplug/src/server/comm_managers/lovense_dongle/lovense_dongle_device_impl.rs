use super::lovense_dongle_messages::{
  LovenseDongleIncomingMessage,
  LovenseDongleMessageFunc,
  LovenseDongleMessageType,
  LovenseDongleOutgoingMessage,
  OutgoingLovenseData,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition},
    BoundedDeviceEventBroadcaster,
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
  util::async_manager,
};
use async_channel::{Receiver, Sender};
use async_trait::async_trait;
use broadcaster::BroadcastChannel;
use futures::{
  future::{self, BoxFuture},
  StreamExt,
};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};

pub struct LovenseDongleDeviceImplCreator {
  specifier: DeviceSpecifier,
  id: String,
  device_outgoing: Sender<OutgoingLovenseData>,
  device_incoming: Receiver<LovenseDongleIncomingMessage>,
}

impl LovenseDongleDeviceImplCreator {
  pub fn new(
    id: &str,
    device_outgoing: Sender<OutgoingLovenseData>,
    device_incoming: Receiver<LovenseDongleIncomingMessage>,
  ) -> Self {
    Self {
      // We know the only thing we'll ever get from a lovense dongle is a
      // lovense device. However, we don't have a way to specify that in our
      // device config file. Therefore, we just lie and act like it's a
      // bluetooth device with a name that will match the Lovense builder. Then
      // when we get the device, we can set up as we need.
      //
      // Hacky, but it works.
      specifier: DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
        "LVS-SerialPortDevice",
      )),
      id: id.to_string(),
      device_outgoing,
      device_incoming,
    }
  }
}

#[async_trait]
impl ButtplugDeviceImplCreator for LovenseDongleDeviceImplCreator {
  fn get_specifier(&self) -> DeviceSpecifier {
    self.specifier.clone()
  }

  async fn try_create_device_impl(
    &mut self,
    _protocol: ProtocolDefinition,
  ) -> Result<Box<dyn DeviceImpl>, ButtplugError> {
    Ok(Box::new(LovenseDongleDeviceImpl::new(
      &self.id,
      self.device_outgoing.clone(),
      self.device_incoming.clone(),
    )))
  }
}

#[derive(Clone)]
pub struct LovenseDongleDeviceImpl {
  name: String,
  address: String,
  device_outgoing: Sender<OutgoingLovenseData>,
  connected: Arc<AtomicBool>,
  event_receiver: BoundedDeviceEventBroadcaster,
}

impl LovenseDongleDeviceImpl {
  pub fn new(
    address: &str,
    device_outgoing: Sender<OutgoingLovenseData>,
    mut device_incoming: Receiver<LovenseDongleIncomingMessage>,
  ) -> Self {
    let event_broadcaster = BroadcastChannel::with_cap(256);
    let event_broadcaster_clone = event_broadcaster.clone();
    async_manager::spawn(async move {
      while let Some(msg) = device_incoming.next().await {
        if msg.func != LovenseDongleMessageFunc::ToyData {
          continue;
        }
        let data_str = msg.data.unwrap().data.unwrap();
        event_broadcaster_clone
          .send(&ButtplugDeviceEvent::Notification(
            Endpoint::Rx,
            data_str.into_bytes(),
          ))
          .await
          .unwrap();
      }
    })
    .unwrap();
    Self {
      name: "Lovense Dongle Device".to_owned(),
      address: address.to_string(),
      device_outgoing,
      connected: Arc::new(AtomicBool::new(true)),
      event_receiver: event_broadcaster,
    }
  }
}

impl DeviceImpl for LovenseDongleDeviceImpl {
  fn name(&self) -> &str {
    &self.name
  }

  fn address(&self) -> &str {
    &self.address
  }

  fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  fn endpoints(&self) -> Vec<Endpoint> {
    vec![Endpoint::Rx, Endpoint::Tx]
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    let connected = self.connected.clone();
    Box::pin(async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    })
  }

  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    self.event_receiver.clone()
  }

  fn read_value(
    &self,
    _msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    unimplemented!()
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let port_sender = self.device_outgoing.clone();
    let address = self.address.clone();
    Box::pin(async move {
      let outgoing_msg = LovenseDongleOutgoingMessage {
        func: LovenseDongleMessageFunc::Command,
        message_type: LovenseDongleMessageType::Toy,
        id: Some(address),
        command: Some(std::str::from_utf8(&msg.data).unwrap().to_string()),
        eager: None,
      };
      port_sender
        .send(OutgoingLovenseData::Message(outgoing_msg))
        .await
        .map_err(|_| {
          error!("Port closed during writing.");
          ButtplugError::ButtplugDeviceError(ButtplugDeviceError::DeviceNotConnected(
            "Port closed during writing".to_owned(),
          ))
        })
    })
  }

  fn subscribe(&self, _msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    // unimplemented!();
    Box::pin(future::ready(Ok(())))
  }
}
