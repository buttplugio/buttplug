use super::lovense_dongle_messages::{
  LovenseDongleIncomingMessage,
  LovenseDongleMessageFunc,
  LovenseDongleMessageType,
  LovenseDongleOutgoingMessage,
  OutgoingLovenseData,
};
use crate::{core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  }, device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition},
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  }, util::{async_manager, stream::convert_broadcast_receiver_to_stream}};
use tokio::sync::{mpsc, broadcast};
use async_trait::async_trait;
use futures::{
  Stream,
  future::{self, BoxFuture},
};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use std::fmt::{self, Debug};

pub struct LovenseDongleDeviceImplCreator {
  specifier: DeviceSpecifier,
  id: String,
  device_outgoing: mpsc::Sender<OutgoingLovenseData>,
  device_incoming: Option<mpsc::Receiver<LovenseDongleIncomingMessage>>,
}

impl Debug for LovenseDongleDeviceImplCreator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LovenseDongleDeviceImplCreator")
      .field("id", &self.id)
      .field("specifier", &self.specifier)
      .finish()
  }
}

impl LovenseDongleDeviceImplCreator {
  pub fn new(
    id: &str,
    device_outgoing: mpsc::Sender<OutgoingLovenseData>,
    device_incoming: mpsc::Receiver<LovenseDongleIncomingMessage>,
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
        "LVS-DongleDevice",
      )),
      id: id.to_string(),
      device_outgoing,
      device_incoming: Some(device_incoming),
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
      self.device_incoming.take().unwrap(),
    )))
  }
}

#[derive(Clone)]
pub struct LovenseDongleDeviceImpl {
  name: String,
  address: String,
  device_outgoing: mpsc::Sender<OutgoingLovenseData>,
  connected: Arc<AtomicBool>,
  event_sender: broadcast::Sender<ButtplugDeviceEvent>,
}

impl LovenseDongleDeviceImpl {
  pub fn new(
    address: &str,
    device_outgoing: mpsc::Sender<OutgoingLovenseData>,
    mut device_incoming: mpsc::Receiver<LovenseDongleIncomingMessage>,
  ) -> Self {
    let (event_sender, _) = broadcast::channel(256);
    let event_sender_clone = event_sender.clone();
    async_manager::spawn(async move {
      while let Some(msg) = device_incoming.recv().await {
        if msg.func != LovenseDongleMessageFunc::ToyData {
          continue;
        }
        let data_str = msg.data.unwrap().data.unwrap();
        event_sender_clone
          .send(ButtplugDeviceEvent::Notification(
            Endpoint::Rx,
            data_str.into_bytes(),
          ))
          .unwrap();
      }
      info!("Lovense dongle device disconnected",);
      // This should always succeed, as it'll relay up to the device manager,
      // and that's what owns us.
      event_sender_clone
        .send(ButtplugDeviceEvent::Removed)
        .unwrap();
    })
    .unwrap();
    Self {
      name: "Lovense Dongle Device".to_owned(),
      address: address.to_string(),
      device_outgoing,
      connected: Arc::new(AtomicBool::new(true)),
      event_sender
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

  fn event_stream(&self) -> Box<dyn Stream<Item = ButtplugDeviceEvent> + Send + Unpin> {
    Box::new(Box::pin(convert_broadcast_receiver_to_stream(self.event_sender.subscribe())))
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
