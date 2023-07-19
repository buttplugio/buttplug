// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Implementation of internal Buttplug Client event loop.

use super::{
  client_message_sorter::ClientMessageSorter,
  device::{ButtplugClientDevice, ButtplugClientDeviceEvent},
  ButtplugClientEvent,
  ButtplugClientMessageFuturePair,
  ButtplugClientMessageSender,
};
use crate::core::{
  connector::{ButtplugConnector, ButtplugConnectorStateShared},
  errors::{ButtplugDeviceError, ButtplugError},
  message::{
    ButtplugCurrentSpecClientMessage,
    ButtplugCurrentSpecServerMessage,
    ButtplugDeviceMessage,
    ButtplugMessageValidator,
    DeviceList,
    DeviceMessageInfo,
  },
};
use dashmap::DashMap;
use futures::FutureExt;
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::{broadcast, mpsc};

/// Enum used for communication from the client to the event loop.
#[derive(Clone)]
pub enum ButtplugClientRequest {
  /// Client request to disconnect, via already sent connector instance.
  Disconnect(ButtplugConnectorStateShared),
  /// Given a DeviceList message, update the inner loop values and create
  /// events for additions.
  HandleDeviceList(DeviceList),
  /// Client request to send a message via the connector.
  ///
  /// Bundled future should have reply set and waker called when this is
  /// finished.
  Message(ButtplugClientMessageFuturePair),
}

/// Event loop for running [ButtplugClient] connections.
///
/// Acts as a hub for communication between the connector and [ButtplugClient]
/// instances.
///
/// Created whenever a new [super::ButtplugClient] is created, the internal loop
/// handles connection and communication with the server through the connector,
/// and creation of events received from the server.
///
/// The event_loop does a few different things during its lifetime:
///
/// - It will listen for events from the connector, or messages from the client,
///   routing them to their proper receivers until either server/client
///   disconnects.
///
/// - On disconnect, it will tear down, and cannot be used again. All clients
///   and devices associated with the loop will be invalidated, and connect must
///   be called on the client again (or a new client should be created).
///
/// # Why an event loop?
///
/// Due to the async nature of Buttplug, we many channels routed to many
/// different tasks. However, all of those tasks will refer to the same event
/// loop. This allows us to coordinate and centralize our information while
/// keeping the API async.
///
/// Note that no async call here should block. Any .await should only be on
/// async channels, and those channels should never have backpressure. We hope.
pub(super) struct ButtplugClientEventLoop<ConnectorType>
where
  ConnectorType:
    ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage> + 'static,
{
  /// Connected status from client, managed by the event loop in case of disconnect.
  connected_status: Arc<AtomicBool>,
  /// Connector the event loop will use to communicate with the [ButtplugServer]
  connector: ConnectorType,
  /// Receiver for messages send from the [ButtplugServer] via the connector.
  from_connector_receiver: mpsc::Receiver<ButtplugCurrentSpecServerMessage>,
  /// Map of devices shared between the client and the event loop
  device_map: Arc<DashMap<u32, Arc<ButtplugClientDevice>>>,
  /// Sends events to the [ButtplugClient] instance.
  to_client_sender: broadcast::Sender<ButtplugClientEvent>,
  /// Sends events to the client receiver. Stored here so it can be handed to
  /// new ButtplugClientDevice instances.
  from_client_sender: Arc<ButtplugClientMessageSender>,
  /// Receives incoming messages from client instances.
  from_client_receiver: broadcast::Receiver<ButtplugClientRequest>,
  sorter: ClientMessageSorter,
}

impl<ConnectorType> ButtplugClientEventLoop<ConnectorType>
where
  ConnectorType:
    ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage> + 'static,
{
  /// Creates a new [ButtplugClientEventLoop].
  ///
  /// Given the [ButtplugClientConnector] object, as well as the channels used
  /// for communicating with the client, creates an event loop structure and
  /// returns it.
  pub fn new(
    connected_status: Arc<AtomicBool>,
    connector: ConnectorType,
    from_connector_receiver: mpsc::Receiver<ButtplugCurrentSpecServerMessage>,
    to_client_sender: broadcast::Sender<ButtplugClientEvent>,
    from_client_sender: Arc<ButtplugClientMessageSender>,
    device_map: Arc<DashMap<u32, Arc<ButtplugClientDevice>>>,
  ) -> Self {
    trace!("Creating ButtplugClientEventLoop instance.");
    Self {
      connected_status,
      device_map,
      from_client_receiver: from_client_sender.subscribe(),
      from_client_sender,
      to_client_sender,
      from_connector_receiver,
      connector,
      sorter: ClientMessageSorter::default(),
    }
  }

  /// Creates a [ButtplugClientDevice] from [DeviceMessageInfo].
  ///
  /// Given a [DeviceMessageInfo] from a [DeviceAdded] or [DeviceList] message,
  /// creates a ButtplugClientDevice and adds it the internal device map, then
  /// returns the instance.
  fn create_client_device(&mut self, info: &DeviceMessageInfo) -> Arc<ButtplugClientDevice> {
    debug!(
      "Trying to create a client device from DeviceMessageInfo: {:?}",
      info
    );
    match self.device_map.get(&info.device_index()) {
      // If the device already exists in our map, clone it.
      Some(dev) => {
        debug!("Device already exists, creating clone.");
        dev.clone()
      }
      // If it doesn't, insert it.
      None => {
        debug!("Device does not exist, creating new entry.");
        let device = Arc::new(ButtplugClientDevice::new_from_device_info(
          info,
          &self.from_client_sender,
        ));
        self.device_map.insert(info.device_index(), device.clone());
        device
      }
    }
  }

  fn send_client_event(&mut self, event: ButtplugClientEvent) {
    trace!("Forwarding event {:?} to client", event);

    if self.to_client_sender.receiver_count() == 0 {
      error!(
        "Client event {:?} dropped, no client event listener available.",
        event
      );
      return;
    }

    self
      .to_client_sender
      .send(event)
      .expect("Already checked for receivers.");
  }

  fn disconnect_device(&mut self, device_index: u32) {
    if !self.device_map.contains_key(&device_index) {
      return;
    }

    let device = (*self
      .device_map
      .get(&device_index)
      .expect("Checked for device index already."))
    .clone();
    device.set_device_connected(false);
    device.queue_event(ButtplugClientDeviceEvent::DeviceRemoved);
    // Then remove it from our storage map
    self.device_map.remove(&device_index);
    self.send_client_event(ButtplugClientEvent::DeviceRemoved(device));
  }

  /// Parse device messages from the connector.
  ///
  /// Since the event loop maintains the state of all devices reported from the
  /// server, it will catch [DeviceAdded]/[DeviceList]/[DeviceRemoved] messages
  /// and update its map accordingly. After that, it will pass the information
  /// on as a [ButtplugClientEvent] to the [ButtplugClient].
  async fn parse_connector_message(&mut self, msg: ButtplugCurrentSpecServerMessage) {
    if self.sorter.maybe_resolve_result(&msg) {
      trace!("Message future found, returning");
      return;
    }
    if let Err(e) = msg.is_valid() {
      error!("Message not valid: {:?} - Error: {}", msg, e);
      self.send_client_event(ButtplugClientEvent::Error(ButtplugError::from(e)));
      return;
    }
    trace!("Message future not found, assuming server event.");
    info!("{:?}", msg);
    match msg {
      ButtplugCurrentSpecServerMessage::DeviceAdded(dev) => {
        trace!("Device added, updating map and sending to client");
        // We already have this device. Emit an error to let the client know the
        // server is being weird.
        if self.device_map.get(&dev.device_index()).is_some() {
          self.send_client_event(ButtplugClientEvent::Error(
            ButtplugDeviceError::DeviceConnectionError(
              "Device already exists in client. Server may be in a weird state.".to_owned(),
            )
            .into(),
          ));
          return;
        }
        let info = DeviceMessageInfo::from(dev);
        let device = self.create_client_device(&info);
        self.send_client_event(ButtplugClientEvent::DeviceAdded(device));
      }
      ButtplugCurrentSpecServerMessage::DeviceRemoved(dev) => {
        if self.device_map.contains_key(&dev.device_index()) {
          trace!("Device removed, updating map and sending to client");
          self.disconnect_device(dev.device_index());
        } else {
          error!("Received DeviceRemoved for non-existent device index");
          self.send_client_event(ButtplugClientEvent::Error(ButtplugDeviceError::DeviceConnectionError("Device removal requested for a device the client does not know about. Server may be in a weird state.".to_owned()).into()));
        }
      }
      ButtplugCurrentSpecServerMessage::ScanningFinished(_) => {
        trace!("Scanning finished event received, forwarding to client.");
        self.send_client_event(ButtplugClientEvent::ScanningFinished);
      }
      ButtplugCurrentSpecServerMessage::RawReading(msg) => {
        let device_idx = msg.device_index();
        if let Some(device) = self.device_map.get(&device_idx) {
          device
            .value()
            .queue_event(ButtplugClientDeviceEvent::Message(
              ButtplugCurrentSpecServerMessage::from(msg),
            ));
        }
      }
      ButtplugCurrentSpecServerMessage::SensorReading(msg) => {
        let device_idx = msg.device_index();
        if let Some(device) = self.device_map.get(&device_idx) {
          device
            .value()
            .queue_event(ButtplugClientDeviceEvent::Message(
              ButtplugCurrentSpecServerMessage::from(msg),
            ));
        }
      }
      ButtplugCurrentSpecServerMessage::Error(e) => {
        self.send_client_event(ButtplugClientEvent::Error(e.into()));
      }
      _ => error!("Cannot process message, dropping: {:?}", msg),
    }
  }

  /// Send a message from the [ButtplugClient] to the [ButtplugClientConnector].
  async fn send_message(&mut self, mut msg_fut: ButtplugClientMessageFuturePair) {
    if let Err(e) = &msg_fut.msg.is_valid() {
      error!("Message not valid: {:?} - Error: {}", msg_fut.msg, e);
      msg_fut
        .waker
        .set_reply(Err(ButtplugError::from(e.clone()).into()));
      return;
    }

    trace!("Sending message to connector: {:?}", msg_fut.msg);
    self.sorter.register_future(&mut msg_fut);
    if self.connector.send(msg_fut.msg).await.is_err() {
      error!("Sending message failed, connector most likely no longer connected.");
    }
  }

  /// Parses message types from the client, returning false when disconnect
  /// happens.
  ///
  /// Takes different messages from the client and handles them:
  ///
  /// - For outbound messages to the server, sends them to the connector/server.
  /// - For disconnections, requests connector disconnect
  /// - For RequestDeviceList, builds a reply out of its own
  async fn parse_client_request(&mut self, msg: ButtplugClientRequest) -> bool {
    match msg {
      ButtplugClientRequest::Message(msg_fut) => {
        trace!("Sending message through connector: {:?}", msg_fut.msg);
        self.send_message(msg_fut).await;
        true
      }
      ButtplugClientRequest::Disconnect(state) => {
        trace!("Client requested disconnect");
        state.set_reply(self.connector.disconnect().await);
        false
      }
      ButtplugClientRequest::HandleDeviceList(device_list) => {
        trace!("Device list received, updating map.");
        for d in device_list.devices() {
          if self.device_map.contains_key(&d.device_index()) {
            continue;
          }
          let device = self.create_client_device(d);
          self.send_client_event(ButtplugClientEvent::DeviceAdded(device));
        }
        true
      }
    }
  }

  /// Runs the event loop, returning once either the client or connector drops.
  pub async fn run(&mut self) {
    debug!("Running client event loop.");
    loop {
      select! {
        event = self.from_connector_receiver.recv().fuse() => match event {
          None => {
            info!("Connector disconnected, exiting loop.");
            break;
          }
          Some(msg) => {
            self.parse_connector_message(msg).await;
          }
        },
        client = self.from_client_receiver.recv().fuse() => match client {
          Err(_) => {
            info!("Client disconnected, exiting loop.");
            break;
          }
          Ok(msg) => {
            if !self.parse_client_request(msg).await {
              break;
            }
          }
        },
      };
    }
    self
      .device_map
      .iter()
      .for_each(|val| val.value().set_client_connected(false));

    let device_indexes: Vec<u32> = self.device_map.iter().map(|k| *k.key()).collect();
    device_indexes
      .iter()
      .for_each(|k| self.disconnect_device(*k));
    self.connected_status.store(false, Ordering::SeqCst);
    self.send_client_event(ButtplugClientEvent::ServerDisconnect);

    debug!("Exiting client event loop.");
  }
}
