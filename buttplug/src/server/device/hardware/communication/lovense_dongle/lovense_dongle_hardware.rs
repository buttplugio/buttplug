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
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{BluetoothLESpecifier, ProtocolCommunicationSpecifier},
    hardware::{
      GenericHardwareSpecializer,
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
use futures::future::{self, BoxFuture, FutureExt};
use std::{
  collections::HashMap,
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{broadcast, mpsc};

pub struct LovenseDongleHardwareConnector {
  specifier: ProtocolCommunicationSpecifier,
  id: String,
  device_outgoing: mpsc::Sender<OutgoingLovenseData>,
  device_incoming: Option<mpsc::Receiver<LovenseDongleIncomingMessage>>,
}

impl Debug for LovenseDongleHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LovenseDongleHardwareConnector")
      .field("id", &self.id)
      .field("specifier", &self.specifier)
      .finish()
  }
}

impl LovenseDongleHardwareConnector {
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
      specifier: ProtocolCommunicationSpecifier::BluetoothLE(
        BluetoothLESpecifier::new_from_device("LVS-DongleDevice", &HashMap::new(), &[]),
      ),
      id: id.to_string(),
      device_outgoing,
      device_incoming: Some(device_incoming),
    }
  }
}

#[async_trait]
impl HardwareConnector for LovenseDongleHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let hardware_internal = LovenseDongleHardware::new(
      &self.id,
      self.device_outgoing.clone(),
      self
        .device_incoming
        .take()
        .expect("We'll always have a device here"),
    );
    let device = Hardware::new(
      "Lovense Dongle Device",
      &self.id,
      &[Endpoint::Rx, Endpoint::Tx],
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(device)))
  }
}

#[derive(Clone)]
pub struct LovenseDongleHardware {
  address: String,
  device_outgoing: mpsc::Sender<OutgoingLovenseData>,
  connected: Arc<AtomicBool>,
  event_sender: broadcast::Sender<HardwareEvent>,
}

impl LovenseDongleHardware {
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
          .send(HardwareEvent::Notification(
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
        .send(HardwareEvent::Disconnected(address_clone.clone()))
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

impl HardwareInternal for LovenseDongleHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let connected = self.connected.clone();
    async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    }
    .boxed()
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Lovense Dongle does not support read".to_owned(),
    )))
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let port_sender = self.device_outgoing.clone();
    let address = self.address.clone();
    let data = msg.data.clone();
    async move {
      let outgoing_msg = LovenseDongleOutgoingMessage {
        func: LovenseDongleMessageFunc::Command,
        message_type: LovenseDongleMessageType::Toy,
        id: Some(address),
        command: Some(
          std::str::from_utf8(&data)
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
          ButtplugDeviceError::DeviceNotConnected("Port closed during writing".to_owned())
        })
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    // DO NOT CHANGE THIS.
    //
    // Lovense Dongle Subscribe/Unsubscribe basically needs to lie about subscriptions. The actual
    // devices need subscribe/unsubscribe to get information back from their rx characteristic, but
    // for the dongle we manage this in the state machine. Therefore we don't really have an
    // explicit attach/detach system like bluetooth. We just act like we do.
    future::ready(Ok(())).boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    // DO NOT CHANGE THIS.
    //
    // Lovense Dongle Subscribe/Unsubscribe basically needs to lie about subscriptions. The actual
    // devices need subscribe/unsubscribe to get information back from their rx characteristic, but
    // for the dongle we manage this in the state machine. Therefore we don't really have an
    // explicit attach/detach system like bluetooth. We just act like we do.
    future::ready(Ok(())).boxed()
  }
}
