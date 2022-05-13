// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

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
    messages::{Endpoint, RawReading},
    ButtplugResultFuture,
  },
  server::device::{
    configuration::{BluetoothLESpecifier, ProtocolCommunicationSpecifier, ProtocolDeviceConfiguration},
    device::device_impl::{
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceImplInternal,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    },
  },
  util::async_manager,
};
use async_trait::async_trait;
use futures::future::{self, BoxFuture};
use std::fmt::{self, Debug};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::{broadcast, mpsc};

pub struct LovenseDongleDeviceImplCreator {
  specifier: ProtocolCommunicationSpecifier,
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
      specifier: ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
        "LVS-DongleDevice",
        &[],
      )),
      id: id.to_string(),
      device_outgoing,
      device_incoming: Some(device_incoming),
    }
  }
}

#[async_trait]
impl ButtplugDeviceImplCreator for LovenseDongleDeviceImplCreator {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn try_create_device_impl(
    &mut self,
    _protocol: ProtocolDeviceConfiguration,
  ) -> Result<DeviceImpl, ButtplugError> {
    let device_impl_internal = LovenseDongleDeviceImpl::new(
      &self.id,
      self.device_outgoing.clone(),
      self
        .device_incoming
        .take()
        .expect("We'll always have a device here"),
    );
    let device = DeviceImpl::new(
      "Lovense Dongle Device",
      &self.id,
      &[Endpoint::Rx, Endpoint::Tx],
      Box::new(device_impl_internal),
    );
    Ok(device)
  }
}

#[derive(Clone)]
pub struct LovenseDongleDeviceImpl {
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
    let address_clone = address.to_owned();
    let (device_event_sender, _) = broadcast::channel(256);
    let device_event_sender_clone = device_event_sender.clone();
    async_manager::spawn(async move {
      while let Some(msg) = device_incoming.recv().await {
        if msg.func != LovenseDongleMessageFunc::ToyData {
          continue;
        }
        let data_str = msg
          .data
          .expect("USB format shouldn't change")
          .data
          .expect("USB format shouldn't change");
        if device_event_sender_clone
          .send(ButtplugDeviceEvent::Notification(
            address_clone.clone(),
            Endpoint::Rx,
            data_str.into_bytes(),
          ))
          .is_err()
        {
          // This sometimes happens with the serial dongle, not sure why. I
          // think it may have to do some sort of connection timing. It seems
          // like we can continue through it and be fine? Who knows. God I
          // hate the lovense dongle.
          error!("Can't send to device event sender, continuing Lovense dongle loop.");
        }
      }
      info!("Lovense dongle device disconnected",);
      if device_event_sender_clone
        .send(ButtplugDeviceEvent::Disconnected(address_clone.clone()))
        .is_err()
      {
        error!("Device Manager no longer alive, cannot send removed event.");
      }
    });
    Self {
      address: address.to_owned(),
      device_outgoing,
      connected: Arc::new(AtomicBool::new(true)),
      event_sender: device_event_sender,
    }
  }
}

impl DeviceImplInternal for LovenseDongleDeviceImpl {
  fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.event_sender.subscribe()
  }

  fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    let connected = self.connected.clone();
    Box::pin(async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    })
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
        command: Some(
          std::str::from_utf8(&msg.data)
            .expect("Got this from our own protocol code, we know it'll be a formattable string.")
            .to_string(),
        ),
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
