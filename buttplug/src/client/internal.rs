// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Implementation of internal Buttplug Client event loop.

use super::{
  client_message_sorter::ClientMessageSorter, device::{ButtplugClientDevice, ButtplugClientDeviceEvent}, 
  ButtplugClientError,
  ButtplugClientEvent, ButtplugClientMessageFuturePair,
};
use crate::{
  connector::{ButtplugConnector, ButtplugConnectorStateShared},
  core::messages::{
    ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage,
    DeviceList, DeviceMessageInfo,
  },
};
use async_channel::{bounded, Sender, Receiver};
use futures::{future::BoxFuture, StreamExt, FutureExt};
use std::{sync::Arc, hash::{Hash, Hasher}};
use broadcaster::BroadcastChannel;

/// Enum used for communication from the client to the event loop.
pub(super) enum ButtplugClientRequest {
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

#[derive(ShallowCopy)]
pub(super) struct ButtplugClientDeviceInternal {
  // We do not want to store a full ButtplugClientDevice here, as it will
  // contain event channels that are never handled. Instead, we should create
  // new client devices when they are requested. Ideally, client devices should
  // just be Arc<T>'s at some point so we don't have to worry about clones either.
  //
  // TODO Make devices Arc<T>, store here.
  pub device: Arc<DeviceMessageInfo>,
  pub channel: Arc<BroadcastChannel<ButtplugClientDeviceEvent>>
}

impl Eq for ButtplugClientDeviceInternal {}

impl ButtplugClientDeviceInternal {
  pub fn new(device: DeviceMessageInfo, channel: BroadcastChannel<ButtplugClientDeviceEvent>) -> Self {
    Self {
      device: Arc::new(device),
      channel: Arc::new(channel),
    }
  }
}

impl PartialEq for ButtplugClientDeviceInternal {
  fn eq(&self, other: &Self) -> bool {
    self.device.device_index == other.device.device_index
  }
}

impl Hash for ButtplugClientDeviceInternal {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.device.device_index.hash(state);
    self.device.device_name.hash(state);
  }
}

/// Event loop for running [ButtplugClient] connections.
///
/// Acts as a hub for communication between the connector and [ButtplugClient]
/// instances.
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
struct ButtplugClientEventLoop<ConnectorType>
where
  ConnectorType:
    ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage> + 'static,
{
  device_map_writer: evmap::WriteHandle<u32, ButtplugClientDeviceInternal>,
  device_map_reader: evmap::ReadHandle<u32, ButtplugClientDeviceInternal>,
  /// Sends events to the [ButtplugClient] instance. This needs to be a
  /// broadcast channel, as the client will have at least 2 copies of it, so we
  /// want one sender, many receivers, all receiving messages.
  event_sender: BroadcastChannel<ButtplugClientEvent>,
  /// Sends events to the client receiver. Stored here so it can be handed to
  /// new ButtplugClientDevice instances.
  client_sender: Sender<ButtplugClientRequest>,
  /// Receives incoming messages from client instances.
  client_receiver: Receiver<ButtplugClientRequest>,
  /// Connector the event loop will use to communicate with the [ButtplugServer]
  connector: ConnectorType,
  /// Receiver for messages send from the [ButtplugServer] via the connector.
  connector_receiver: Receiver<ButtplugCurrentSpecServerMessage>,
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
    connector: ConnectorType,
    connector_receiver: Receiver<ButtplugCurrentSpecServerMessage>,
    event_sender: BroadcastChannel<ButtplugClientEvent>,
    client_sender: Sender<ButtplugClientRequest>,
    client_receiver: Receiver<ButtplugClientRequest>,
    device_map_writer: evmap::WriteHandle<u32, ButtplugClientDeviceInternal>,
    device_map_reader: evmap::ReadHandle<u32, ButtplugClientDeviceInternal>,
  ) -> Self {
    Self {
      device_map_reader,
      device_map_writer,
      client_sender,
      client_receiver,
      event_sender,
      connector_receiver,
      connector,
      sorter: ClientMessageSorter::default(),
    }
  }

  /// Creates a [ButtplugClientDevice] from [DeviceMessageInfo].
  ///
  /// Given a [DeviceMessageInfo] from a [DeviceAdded] or [DeviceList] message,
  /// creates a ButtplugClientDevice and adds it the internal device map, then
  /// returns the instance.
  fn create_client_device(&mut self, info: &DeviceMessageInfo) -> ButtplugClientDevice {
    match self.device_map_reader.get_one(&info.device_index) {
      // If the device already exists in our map, clone it.
      Some(dev) => {
        ButtplugClientDevice::from((&*dev.device, self.client_sender.clone(), (*dev.channel).clone()))
      }, 
      // If it doesn't, insert it.
      None => {
        let channel = BroadcastChannel::new();
        let device = ButtplugClientDevice::from((info, self.client_sender.clone(), channel.clone()));
        self.device_map_writer.insert(info.device_index, ButtplugClientDeviceInternal::new(info.clone(), channel));
        self.device_map_writer.flush();
        device
      }
    }
  }

  async fn send_client_event(&mut self, event: &ButtplugClientEvent) {
    self.event_sender.send(event).await.unwrap();
    // Due to how broadcaster works, it will always send messages to ALL copies
    // of itself, including this one. This means that many time we send a value,
    // we also have to read it out here. Sucks, but not gonna kill us.
    self.event_sender.recv().await.unwrap();
  }

  /// Parse device messages from the connector.
  ///
  /// Since the event loop maintains the state of all devices reported from the
  /// server, it will catch [DeviceAdded]/[DeviceList]/[DeviceRemoved] messages
  /// and update its map accordingly. After that, it will pass the information
  /// on as a [ButtplugClientEvent] to the [ButtplugClient].
  async fn parse_connector_message(&mut self, msg: ButtplugCurrentSpecServerMessage) {
    info!("Sending message to clients.");
    if self.sorter.maybe_resolve_message(&msg).await {
      return;
    }
    match &msg {
      ButtplugCurrentSpecServerMessage::DeviceAdded(dev) => {
        let info = DeviceMessageInfo::from(dev);
        let device = self.create_client_device(&info);
        self
          .send_client_event(&ButtplugClientEvent::DeviceAdded(device))
          .await;
      }
      ButtplugCurrentSpecServerMessage::DeviceRemoved(dev) => {
        if self.device_map_reader.contains_key(&dev.device_index) {
            let info = (*self.device_map_reader.get_one(&dev.device_index).unwrap().device).clone();
            self.device_map_writer.empty(dev.device_index);
            self.device_map_writer.flush();
            self
              .send_client_event(&ButtplugClientEvent::DeviceRemoved(info))
              .await;
        } else {
            error!("Received DeviceRemoved for non-existent device index");
        }
      }
      ButtplugCurrentSpecServerMessage::Log(log) => {
        self
          .send_client_event(&ButtplugClientEvent::Log(
            log.log_level.clone(),
            log.log_message.clone(),
          ))
          .await;
      }
      ButtplugCurrentSpecServerMessage::ScanningFinished(_) => {
        self
          .send_client_event(&ButtplugClientEvent::ScanningFinished)
          .await;
      }
      _ => panic!("Cannot process message: {:?}", msg),
    }
  }

  /// Send a message from the [ButtplugClient] to the [ButtplugClientConnector].
  async fn send_message(&mut self, mut msg_fut: ButtplugClientMessageFuturePair) {
    self.sorter.register_future(&mut msg_fut);
    // TODO What happens if the connector isn't connected?
    self.connector.send(msg_fut.msg).await.unwrap();
  }

  /// Parses message types from the client, returning false when disconnect
  /// happens.
  ///
  /// Takes different messages from the client and handles them:
  ///
  /// - For outbound messages to the server, sends them to the connector/server.
  /// - For disconnections, requests connector disconnect
  /// - For RequestDeviceList, builds a reply out of its own
  async fn parse_client_message(&mut self, msg: ButtplugClientRequest) -> bool {
    trace!("Parsing a client message.");
    match msg {
      ButtplugClientRequest::Message(msg_fut) => {
        debug!("Sending message through connector.");
        self.send_message(msg_fut).await;
        true
      }
      ButtplugClientRequest::Disconnect(state) => {
        debug!("Client requested disconnect");
        state.set_reply(self.connector.disconnect().await);
        false
      }
      ButtplugClientRequest::HandleDeviceList(device_list) => {
        debug!("Handling device list!");
        for d in &device_list.devices {
          if self.device_map_reader.contains_key(&d.device_index) {
            continue;
          }
          let device = self.create_client_device(&d);
          self
            .send_client_event(&ButtplugClientEvent::DeviceAdded(device))
            .await;
        }
        true
      }
    }
  }

  /// Runs the event loop, returning once either the client or connector drops.
  pub async fn run(&mut self) {
    // Once connected, wait for messages from either the client, the generated
    // client devices, or the connector, and send them the direction they're
    // supposed to go.
    let mut client_receiver = self.client_receiver.clone();
    let mut connector_receiver = self.connector_receiver.clone();
    loop {
      select! {
        event = connector_receiver.next().fuse() => match event {
          None => {
            debug!("Connector disconnected.");
            return;
          }
          Some(msg) => {
            self.parse_connector_message(msg).await;
          }
        },
        client = client_receiver.next().fuse() => match client {
          None => {
            debug!("Client disconnected.");
            return;
          }
          Some(msg) => { 
            if !self.parse_client_message(msg).await {
              break;
            }
          }
        },
      };
    }
  }
}

/// The internal event loop for [super::ButtplugClient] connection and
/// communication
///
/// Created whenever a new [super::ButtplugClient] is created, the internal loop
/// handles connection and communication with the server through the connector,
/// and creation of events received from the server.
///
/// The event_loop does a few different things during its lifetime.
///
/// - The first thing it will do is wait for a Connect message from a client.
///   This message contains a [ButtplugClientConnector] that will be used to
///   connect and communicate with a [crate::server::ButtplugServer].
///
/// - After a connection is established, it will listen for events from the
///   connector, or messages from the client, until either server/client
///   disconnects.
///
/// - Finally, on disconnect, it will tear down, and cannot be used again. All
///   clients and devices associated with the loop will be invalidated, and a
///   new [super::ButtplugClient] must be created.
pub(super) fn client_event_loop(
  connector: impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
    + 'static,
  connector_receiver: Receiver<ButtplugCurrentSpecServerMessage>,
) -> (
  BoxFuture<'static, Result<(), ButtplugClientError>>,
  evmap::ReadHandle<u32, ButtplugClientDeviceInternal>,
  Sender<ButtplugClientRequest>,
  // This needs clone internally, as the client will make multiple copies.
  impl StreamExt<Item = ButtplugClientEvent> + Clone,
) {
  let event_channel = BroadcastChannel::new();
  let (device_map_reader, device_map_writer) = evmap::new();
  let device_map_reader_clone = device_map_reader.clone();
  let (client_sender, client_receiver) = bounded(256);
  let client_sender_clone = client_sender.clone();
  let event_loop_sender = event_channel.clone();
  let mut event_loop = ButtplugClientEventLoop::new(
    connector,
    connector_receiver,
    event_loop_sender,
    client_sender,
    client_receiver,
    device_map_writer,
    device_map_reader
  );
  (
    Box::pin(async move {
      info!("Starting client event loop.");
      event_loop
      .run()
      .await;
      info!("Stopping client event loop.");
      Ok(())
    }),
    device_map_reader_clone,
    client_sender_clone,
    event_channel,
  )
}
