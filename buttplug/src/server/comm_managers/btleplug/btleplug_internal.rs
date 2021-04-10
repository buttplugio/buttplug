use crate::{
  core::{errors::ButtplugDeviceError, messages, ButtplugResult},
  device::{
    configuration_manager::BluetoothLESpecifier,
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
use btleplug::api::{CentralEvent, Characteristic, Peripheral, ValueNotification, WriteType};
use uuid::Uuid;
use futures::FutureExt;
use std::collections::HashMap;
use tokio::{runtime::Handle, sync::{broadcast, mpsc}};

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
  write_receiver: mpsc::Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
  event_receiver: mpsc::Receiver<CentralEvent>,
  output_sender: broadcast::Sender<ButtplugDeviceEvent>,
  endpoints: HashMap<Endpoint, Characteristic>,
}

impl<T: Peripheral> BtlePlugInternalEventLoop<T> {
  pub fn new(
    mut btleplug_event_broadcaster: broadcast::Receiver<CentralEvent>,
    device: T,
    protocol: BluetoothLESpecifier,
    write_receiver: mpsc::Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    output_sender: broadcast::Sender<ButtplugDeviceEvent>,
  ) -> Self {
    let (event_sender, event_receiver) = mpsc::channel(256);
    let device_address = device.address();
    async_manager::spawn(async move {
      while let Ok(event) = btleplug_event_broadcaster.recv().await {
        match event {
          CentralEvent::DeviceConnected(ev) => {
            if ev != device_address {
              debug!("Device {} connect event received, but instance device address is {}, ignoring", ev, device_address);
              continue;
            } else {
              debug!("Device {} connect event received, matches instance device address, notifying event loop", ev);
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
              debug!("Device {} disconnect event received, matches instance device address, exiting", ev);
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
      let return_err = ButtplugDeviceError::DeviceSpecificError(
        ButtplugDeviceSpecificError::BtleplugError(format!("{:?}", err)),
      );
      state.set_reply(ButtplugDeviceReturn::Error(return_err.clone().into()));
      return Err(return_err.into());
    }
    loop {
      let event = self.event_receiver.recv().await;
      if event.is_none() {
        error!("BTLEPlug connection event handler died, cannot receive connection event.");
        state.set_reply(ButtplugDeviceReturn::Error(
          ButtplugDeviceError::DeviceConnectionError(
            "BTLEPlug connection event handler died, cannot receive connection event.".to_owned(),
          )
          .into(),
        ));
        return Err(
          ButtplugDeviceError::DeviceConnectionError(
            "BTLEPlug connection event handler died, cannot receive connection event.".to_owned(),
          )
          .into(),
        );
      }
      // We just checked it, we can unwrap here.
      match event.unwrap() {
        CentralEvent::DeviceConnected(addr) => {
          if addr == self.device.address() {
            info!("Device connected.");
            break;
          }
        }
        wrong_event => warn!("Got unexpected message {:?}", wrong_event),
      }
    }
    // Map UUIDs to endpoints
    let mut uuid_map = HashMap::<Uuid, Endpoint>::new();
    let chars = match self.device.discover_characteristics() {
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
    for proto_service in self.protocol.services.values() {
      for (chr_name, chr_uuid) in proto_service.iter() {
        let maybe_chr = chars.iter().find(|c| c.uuid == *chr_uuid);
        if let Some(chr) = maybe_chr {
          self.endpoints.insert(*chr_name, chr.clone());
          uuid_map.insert(*chr_uuid, *chr_name);
        }
      }
    }
    let os = self.output_sender.clone();
    let mut error_notification = false;
    let address = self.device.properties().address.to_string();
    let handle = Handle::current();
    self
      .device
      .on_notification(Box::new(move |notification: ValueNotification| {
        let endpoint = if let Some(endpoint) = uuid_map.get(&notification.uuid) {
          *endpoint
        } else {
          if !error_notification {
            error!(
              "Endpoint for UUID {} not found in map, assuming device has disconnected.",
              notification.uuid
            );
            error_notification = true;
          }
          return;
        };
        let sender = os.clone();
        let address_clone = address.clone();
        let fut = async move {
          if let Err(err) = sender.send(ButtplugDeviceEvent::Notification(
            address_clone,
            endpoint,
            notification.value,
          )) {
            error!(
              "Cannot send notification, device object disappeared: {:?}",
              err
            );
          }
        };
        handle.spawn(fut);
      }));
    let device_info = ButtplugDeviceImplInfo {
      endpoints: self.endpoints.keys().cloned().collect(),
      manufacturer_name: None,
      product_name: None,
      serial_number: None,
    };
    state.set_reply(ButtplugDeviceReturn::Connected(device_info));
    Ok(())
  }

  fn handle_write(&mut self, write_msg: &DeviceWriteCmd, state: &mut DeviceReturnStateShared) {
    match self.endpoints.get(&write_msg.endpoint) {
      Some(chr) => {
        let write_type = if write_msg.write_with_response {
          WriteType::WithResponse
        } else {
          WriteType::WithoutResponse
        };
        if let Err(err) = self.device.write(&chr, &write_msg.data, write_type) {
          error!("BTLEPlug device write error: {:?}", err);
        } else {
          state.set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
        }
      }
      None => state.set_reply(ButtplugDeviceReturn::Error(
        ButtplugDeviceError::InvalidEndpoint(write_msg.endpoint).into(),
      )),
    }
  }

  fn handle_read(&mut self, read_msg: &DeviceReadCmd, state: &mut DeviceReturnStateShared) {
    match self.endpoints.get(&read_msg.endpoint) {
      Some(chr) => match self.device.read(&chr) {
        Ok(data) => {
          trace!("Got reading: {:?}", data);
          state.set_reply(ButtplugDeviceReturn::RawReading(messages::RawReading::new(
            0,
            read_msg.endpoint,
            data,
          )));
        }
        Err(err) => {
          error!("BTLEPlug device read error: {:?}", err);
          state.set_reply(ButtplugDeviceReturn::Error(
            ButtplugDeviceError::DeviceSpecificError(ButtplugDeviceSpecificError::BtleplugError(
              format!("{:?}", err),
            ))
            .into(),
          ));
        }
      },
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
        if let Err(err) = self.device.subscribe(&chr) {
          error!("BTLEPlug device subscribe error: {:?}", err);
        } else {
          state.set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
        }
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
        if let Err(err) = self.device.subscribe(&chr) {
          error!("BTLEPlug device unsubscribe error: {:?}", err);
        } else {
          state.set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
        }
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
      if self.device.address() == addr {
        info!(
          "Device {:?} disconnected",
          self.device.properties().local_name
        );

        // If output_sender isn't hooked up to anything (for instance, if we
        // disconnect while initializing), we have no one to relay this info to.
        // However, that doesn't really matter because we won't have emitted the
        // device as connected yet.
        if self.output_sender.receiver_count() == 0 {
          return true;
        }

        self
          .output_sender
          .send(ButtplugDeviceEvent::Removed(
            self.device.address().to_string(),
          ))
          .unwrap();
        return true;
      }
    }
    false
  }

  pub async fn run(&mut self) {
    loop {
      // Race our device input (from the client side) and any subscribed
      // notifications.
      let mut event = select! {
        ev = self.event_receiver.recv().fuse() => match ev {
          Some(valid_ev) => BtlePlugCommLoopChannelValue::DeviceEvent(valid_ev),
          None => BtlePlugCommLoopChannelValue::ChannelClosed,
        },
        recv = self.write_receiver.recv().fuse() => match recv {
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
          debug!("Channel closed, assuming device event loop should exit.");
          return;
        }
      }
    }
    info!("Exiting device loop");
  }
}
