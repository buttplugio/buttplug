use crate::{
  core::{errors::ButtplugDeviceError, messages, ButtplugResult},
  device::{
    configuration_manager::BluetoothLESpecifier,
    BoundedDeviceEventBroadcaster,
    ButtplugDeviceCommand,
    ButtplugDeviceEvent,
    ButtplugDeviceImplInfo,
    ButtplugDeviceReturn,
    DeviceImplCommand,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
  server::comm_managers::ButtplugDeviceSpecificError,
  util::{
    async_manager,
    future::{ButtplugFuture, ButtplugFutureStateShared},
  },
};
use async_channel::{bounded, Receiver};
use broadcaster::BroadcastChannel;
use btleplug::api::{CentralEvent, Characteristic, Peripheral, ValueNotification, UUID};
use futures::{FutureExt, StreamExt};
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
  pub fn new(
    mut btleplug_event_broadcaster: BroadcastChannel<CentralEvent>,
    device: T,
    protocol: BluetoothLESpecifier,
    write_receiver: Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    output_sender: BoundedDeviceEventBroadcaster,
  ) -> Self {
    let (event_sender, event_receiver) = bounded(256);
    let device_address = device.address();
    async_manager::spawn(async move {
      while let Some(event) = btleplug_event_broadcaster.next().await {
        match event {
          CentralEvent::DeviceConnected(ev) => {
            if ev != device_address {
              debug!("Device {} connect event received, but instance device address is {}, ignoring", ev, device_address);
              continue;
            } else {
              info!("Device {} connect event received, matches instance device address, notifying event loop", ev);
            }
            let s = event_sender.clone();
            let e = event;
            if s.send(e).await.is_err() {
              error!("Trying to send connect event to unbound device!");
              break;
            }
          }
          CentralEvent::DeviceDisconnected(ev) => {
            if ev != device_address {
              debug!("Device {} disconnect event received, but instance device address is {}, ignoring", ev, device_address);
              continue;
            } else {
              info!("Device {} disconnect event received, matches instance device address, exiting", ev);
            }
            let s = event_sender.clone();
            let e = event;
            if s.send(e).await.is_err() {
              error!("Trying to send disconnect event to unbound device!");
            }
            // If we got a disconnect message, we're done being a device for now.
            break;
          }
          _ => {}
        }
      }
    })
    .unwrap();
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
  async fn handle_connection(&mut self, state: &mut DeviceReturnStateShared) -> ButtplugResult {
    info!("Connecting to BTLEPlug device");
    if let Err(err) = self.device.connect() {
      state.set_reply(ButtplugDeviceReturn::Error(
        ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(
          err.clone(),
        ))
        .into(),
      ));
      return Err(
        ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(err))
          .into(),
      );
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
        async_manager::spawn(async move {
          sender
            .send(&ButtplugDeviceEvent::Notification(
              endpoint,
              notification.value,
            ))
            .await
            .unwrap()
        })
        .unwrap();
      }));
    let device_info = ButtplugDeviceImplInfo {
      endpoints: self.endpoints.keys().cloned().collect(),
      manufacturer_name: None,
      product_name: None,
      serial_number: None,
    };
    info!("Device connected!");
    state.set_reply(ButtplugDeviceReturn::Connected(device_info));
    Ok(())
  }

  fn handle_write(&mut self, write_msg: &DeviceWriteCmd, state: &mut DeviceReturnStateShared) {
    match self.endpoints.get(&write_msg.endpoint) {
      Some(chr) => {
        self.device.command(&chr, &write_msg.data).unwrap();
        state.set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
      }
      None => state.set_reply(ButtplugDeviceReturn::Error(
        ButtplugDeviceError::InvalidEndpoint(write_msg.endpoint).into(),
      )),
    }
  }

  fn handle_read(&mut self, read_msg: &DeviceReadCmd, state: &mut DeviceReturnStateShared) {
    match self.endpoints.get(&read_msg.endpoint) {
      Some(chr) => {
        match self.device.read(&chr) {
          Ok (data) => {
            trace!("Got reading: {:?}", data);
            state.set_reply(ButtplugDeviceReturn::RawReading(messages::RawReading::new(0, read_msg.endpoint, data)));
          }
          Err(err) => {
            trace!("Read failed");
            state.set_reply(ButtplugDeviceReturn::Error(ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(err)).into()));
          }
        }
      }
      None => state.set_reply(ButtplugDeviceReturn::Error(
        ButtplugDeviceError::InvalidEndpoint(read_msg.endpoint).into(),
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
        state.set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
      }
      None => state.set_reply(ButtplugDeviceReturn::Error(
        ButtplugDeviceError::InvalidEndpoint(sub_msg.endpoint).into(),
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
        state.set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
      }
      None => state.set_reply(ButtplugDeviceReturn::Error(
        ButtplugDeviceError::InvalidEndpoint(sub_msg.endpoint).into(),
      )),
    }
  }

  pub async fn handle_device_command(
    &mut self,
    command: &ButtplugDeviceCommand,
    state: &mut DeviceReturnStateShared,
  ) -> ButtplugResult {
    match command {
      ButtplugDeviceCommand::Connect => {
        self.handle_connection(state).await?;
      }
      ButtplugDeviceCommand::Message(raw_msg) => match raw_msg {
        DeviceImplCommand::Write(write_msg) => {
          self.handle_write(write_msg, state);
        }
        DeviceImplCommand::Read(read_msg) => {
          self.handle_read(read_msg, state);
        }
        DeviceImplCommand::Subscribe(sub_msg) => {
          self.handle_subscribe(sub_msg, state);
        }
        DeviceImplCommand::Unsubscribe(sub_msg) => {
          self.handle_unsubscribe(sub_msg, state);
        }
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
    Ok(())
  }

  pub async fn handle_device_event(&mut self, event: CentralEvent) -> bool {
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
        return true;
      }
    }
    false
  }

  pub async fn run(&mut self) {
    loop {
      let mut wr = self.write_receiver.clone();
      let mut er = self.event_receiver.clone();

      // Race our device input (from the client side) and any subscribed
      // notifications.
      let mut event = select! {
        ev = er.next().fuse() => match ev {
          Some(valid_ev) => BtlePlugCommLoopChannelValue::DeviceEvent(valid_ev),
          None => BtlePlugCommLoopChannelValue::ChannelClosed,
        },
        recv = wr.next().fuse() => match recv {
          Some((command, state)) => BtlePlugCommLoopChannelValue::DeviceCommand(command, state),
          None => BtlePlugCommLoopChannelValue::ChannelClosed,
        }
      };
      match event {
        BtlePlugCommLoopChannelValue::DeviceCommand(ref command, ref mut state) => {
          if self.handle_device_command(command, state).await.is_err() {
            break;
          }
        }
        BtlePlugCommLoopChannelValue::DeviceEvent(event) => {
          if self.handle_device_event(event).await {
            break;
          }
        }
        BtlePlugCommLoopChannelValue::ChannelClosed => {
          info!("CHANNEL CLOSED");
          return;
        }
      }
    }
    info!("Exiting device loop");
  }
}
