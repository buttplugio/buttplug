use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages,
  },
  device::{
    configuration_manager::BluetoothLESpecifier,
    BoundedDeviceEventBroadcaster,
    ButtplugDeviceCommand,
    ButtplugDeviceEvent,
    ButtplugDeviceImplInfo,
    ButtplugDeviceReturn,
    DeviceImplCommand,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
  util::future::{ButtplugFuture, ButtplugFutureStateShared},
};
use async_std::{
  prelude::{FutureExt, StreamExt},
  sync::{channel, Receiver},
  task,
};
use btleplug::api::{Central, CentralEvent, Characteristic, Peripheral, ValueNotification, UUID};
use std::collections::HashMap;

pub type DeviceReturnStateShared = ButtplugFutureStateShared<ButtplugDeviceReturn>;
pub type DeviceReturnFuture = ButtplugFuture<ButtplugDeviceReturn>;

enum BtlePlugCommLoopChannelValue {
  DeviceCommand(ButtplugDeviceCommand, DeviceReturnStateShared),
  DeviceEvent(CentralEvent),
  ChannelClosed,
}

pub struct BtlePlugInternalEventLoop<T: Peripheral> {
  device: T,
  protocol: BluetoothLESpecifier,
  write_receiver: Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
  event_receiver: Receiver<CentralEvent>,
  output_sender: BoundedDeviceEventBroadcaster,
  endpoints: HashMap<Endpoint, Characteristic>,
}

fn uuid_to_rumble(uuid: &uuid::Uuid) -> UUID {
  let mut rumble_uuid = *uuid.as_bytes();
  rumble_uuid.reverse();
  UUID::B128(rumble_uuid)
}

impl<T: Peripheral> BtlePlugInternalEventLoop<T> {
  pub fn new<C>(
    central: C,
    device: T,
    protocol: BluetoothLESpecifier,
    write_receiver: Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    output_sender: BoundedDeviceEventBroadcaster,
  ) -> Self
  where
    C: Central<T>,
  {
    let (event_sender, event_receiver) = channel(256);
    // Add ourselves to the central event handler output now, so we don't
    // have to carry around the Central object. We'll be using this in
    // connect anyways.
    let on_event = move |event: CentralEvent| match event {
      CentralEvent::DeviceConnected(_) => {
        let s = event_sender.clone();
        let e = event;
        task::spawn(async move {
          s.send(e).await;
        });
      }
      CentralEvent::DeviceDisconnected(_) => {
        let s = event_sender.clone();
        let e = event;
        task::spawn(async move {
          s.send(e).await;
        });
      }
      _ => {}
    };
    // TODO There's no way to unsubscribe central event handlers. That
    // needs to be fixed in rumble somehow, but for now we'll have to
    // make our handlers exit early after dying or something?
    central.on_event(Box::new(on_event));
    BtlePlugInternalEventLoop {
      device,
      protocol,
      write_receiver,
      event_receiver,
      output_sender,
      endpoints: HashMap::new(),
    }
  }

  // TODO this should probably return Result and we should handle state filling in parent.
  async fn handle_connection(&mut self, state: &mut DeviceReturnStateShared) {
    info!("Connecting to BTLEPlug device");
    if let Err(err) = self.device.connect() {
      state
        .lock_now_or_panic()
        .set_reply(ButtplugDeviceReturn::Error(
          ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(&format!(
            "Btleplug device cannot connect: {}",
            err
          ))),
        ));
      return;
    }
    loop {
      let event = self.event_receiver.next().await;
      match event.unwrap() {
        CentralEvent::DeviceConnected(addr) => {
          if addr == self.device.address() {
            info!(
              "Device {:?} connected!",
              self.device.properties().local_name
            );
            break;
          }
        }
        _ => warn!("Got unexpected message {:?}", event),
      }
    }
    // Map UUIDs to endpoints
    let mut uuid_map = HashMap::<UUID, Endpoint>::new();
    let chars = self.device.discover_characteristics().unwrap();
    for proto_service in self.protocol.services.values() {
      for (chr_name, chr_uuid) in proto_service.iter() {
        let maybe_chr = chars.iter().find(|c| c.uuid == uuid_to_rumble(chr_uuid));
        if let Some(chr) = maybe_chr {
          self.endpoints.insert(*chr_name, chr.clone());
          uuid_map.insert(uuid_to_rumble(chr_uuid), *chr_name);
        }
      }
    }
    let os = self.output_sender.clone();
    self
      .device
      .on_notification(Box::new(move |notification: ValueNotification| {
        let endpoint = *uuid_map.get(&notification.uuid).unwrap();
        let sender = os.clone();
        task::spawn(async move {
          sender
            .send(&ButtplugDeviceEvent::Notification(
              endpoint,
              notification.value,
            ))
            .await
        });
      }));
    let device_info = ButtplugDeviceImplInfo {
      endpoints: self.endpoints.keys().cloned().collect(),
      manufacturer_name: None,
      product_name: None,
      serial_number: None,
    };
    info!("Device connected!");
    state
      .lock_now_or_panic()
      .set_reply(ButtplugDeviceReturn::Connected(device_info));
  }

  fn handle_write(&mut self, write_msg: &DeviceWriteCmd, state: &mut DeviceReturnStateShared) {
    match self.endpoints.get(&write_msg.endpoint) {
      Some(chr) => {
        self.device.command(&chr, &write_msg.data).unwrap();
        state
          .lock_now_or_panic()
          .set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
      }
      None => state
        .lock_now_or_panic()
        .set_reply(ButtplugDeviceReturn::Error(
          ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(&format!(
            "Device does not contain an endpoint named {}",
            write_msg.endpoint
          ))),
        )),
    }
  }

  fn handle_subscribe(
    &mut self,
    sub_msg: &DeviceSubscribeCmd,
    state: &mut DeviceReturnStateShared,
  ) {
    match self.endpoints.get(&sub_msg.endpoint) {
      Some(chr) => {
        self.device.subscribe(&chr).unwrap();
        state
          .lock_now_or_panic()
          .set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
      }
      None => state
        .lock_now_or_panic()
        .set_reply(ButtplugDeviceReturn::Error(
          ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(&format!(
            "Device does not contain an endpoint named {}",
            sub_msg.endpoint
          ))),
        )),
    }
  }

  fn handle_unsubscribe(
    &mut self,
    sub_msg: &DeviceUnsubscribeCmd,
    state: &mut DeviceReturnStateShared,
  ) {
    match self.endpoints.get(&sub_msg.endpoint) {
      Some(chr) => {
        self.device.subscribe(&chr).unwrap();
        state
          .lock_now_or_panic()
          .set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
      }
      None => state
        .lock_now_or_panic()
        .set_reply(ButtplugDeviceReturn::Error(
          ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(&format!(
            "Device does not contain an endpoint named {}",
            sub_msg.endpoint
          ))),
        )),
    }
  }

  pub async fn handle_device_command(
    &mut self,
    command: &ButtplugDeviceCommand,
    state: &mut DeviceReturnStateShared,
  ) {
    match command {
      ButtplugDeviceCommand::Connect => {
        self.handle_connection(state).await;
      }
      ButtplugDeviceCommand::Message(raw_msg) => match raw_msg {
        DeviceImplCommand::Write(write_msg) => {
          self.handle_write(write_msg, state);
        }
        DeviceImplCommand::Subscribe(sub_msg) => {
          self.handle_subscribe(sub_msg, state);
        }
        DeviceImplCommand::Unsubscribe(sub_msg) => {
          self.handle_unsubscribe(sub_msg, state);
        }
        _ => state
          .lock_now_or_panic()
          .set_reply(ButtplugDeviceReturn::Error(
            ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(
              "Buttplug-rs does not yet handle reads",
            )),
          )),
      },
      ButtplugDeviceCommand::Disconnect => {
        if let Err(e) = self.device.disconnect() {
          error!(
            "Error disconnecting device {:?}: {:?}",
            self.device.properties().local_name,
            e
          );
        }
      }
    }
  }

  pub async fn handle_device_event(&mut self, event: CentralEvent) {
    if let CentralEvent::DeviceDisconnected(addr) = event {
      // TODO Ok. Great. We can disconnect, but output_sender doesn't
      // really *go* anywhere right now. We're just using it in the
      // Lovense protocol and that's it. We need to be watching for this
      // up in the device manager too, which is going to be...
      // interesting, as I have no idea how we'll deal with instances
      // where we disconnect while waiting in a protocol (for instance, if
      // the device disconnects while we're doing Lovense init). I may
      // need to rethink this.
      if self.device.address() == addr {
        info!(
          "Device {:?} disconnected",
          self.device.properties().local_name
        );
        // This should always succeed
        self
          .output_sender
          .send(&ButtplugDeviceEvent::Removed)
          .await
          .unwrap();
      }
    }
  }

  pub async fn run(&mut self) {
    loop {
      let mut wr = self.write_receiver.clone();
      let receiver = async {
        match wr.next().await {
          Some((command, state)) => BtlePlugCommLoopChannelValue::DeviceCommand(command, state),
          None => BtlePlugCommLoopChannelValue::ChannelClosed,
        }
      };
      let mut er = self.event_receiver.clone();
      let event = async {
        // We own both sides of this so it'll never actually die. Unwrap
        // with impunity.
        BtlePlugCommLoopChannelValue::DeviceEvent(er.next().await.unwrap())
      };
      // Race our device input (from the client side) and any subscribed
      // notifications.
      match receiver.race(event).await {
        BtlePlugCommLoopChannelValue::DeviceCommand(ref command, ref mut state) => {
          self.handle_device_command(command, state).await
        }
        BtlePlugCommLoopChannelValue::DeviceEvent(event) => self.handle_device_event(event).await,
        BtlePlugCommLoopChannelValue::ChannelClosed => {
          info!("CHANNEL CLOSED");
          return;
        }
      }
    }
  }
}
