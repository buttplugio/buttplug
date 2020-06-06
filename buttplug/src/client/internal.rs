// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Implementation of internal Buttplug Client event loop.

use super::{
  client_message_sorter::ClientMessageSorter, device::ButtplugClientDevice, ButtplugClientError,
  ButtplugClientEvent, ButtplugClientMessageFuturePair,
};
use crate::{
  connector::{ButtplugConnector, ButtplugConnectorStateShared},
  core::messages::{
    ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage,
    DeviceList, DeviceMessageInfo,
  },
  util::future::ButtplugFutureStateShared,
};
use async_channel::{bounded, Sender, Receiver};
use futures::{future::{BoxFuture, select}, StreamExt, FutureExt};
use std::collections::HashMap;
use broadcaster::BroadcastChannel;

/// Enum used for communication from the client to the event loop.
pub(super) enum ButtplugClientRequest {
  /// Client request to disconnect, via already sent connector instance.
  Disconnect(ButtplugConnectorStateShared),
  /// Given a DeviceList message, update the inner loop values and create
  /// events for additions.
  HandleDeviceList(DeviceList),
  /// Return new ButtplugClientDevice instances for all known and currently
  /// connected devices.
  RequestDeviceList(ButtplugFutureStateShared<Vec<ButtplugClientDevice>>),
  /// Client request to send a message via the connector.
  ///
  /// Bundled future should have reply set and waker called when this is
  /// finished.
  Message(ButtplugClientMessageFuturePair),
}

/// Enum for messages going to a [ButtplugClientDevice] instance.
#[derive(Clone)]
pub enum ButtplugClientDeviceEvent {
  /// Device has disconnected from server.
  DeviceDisconnect,
  /// Client has disconnected from server.
  ClientDisconnect,
  /// Message was received from server for that specific device.
  Message(ButtplugCurrentSpecServerMessage),
}

/// Set of possible responses from the different inputs to the client inner
/// loop.
enum StreamReturn {
  /// Response from the [ButtplugServer].
  ConnectorMessage(ButtplugCurrentSpecServerMessage),
  /// Incoming message from the [ButtplugClient].
  ClientRequest(ButtplugClientRequest),
  /// Incoming message from a [ButtplugClientDevice].
  DeviceMessage(ButtplugClientMessageFuturePair),
  /// Disconnection from the [ButtplugServer].
  Disconnect,
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
  /// List of currently connected devices.
  devices: HashMap<u32, DeviceMessageInfo>,
  /// Sender to pass to new [ButtplugClientDevice] instances.
  device_message_sender: Sender<ButtplugClientMessageFuturePair>,
  /// Receiver for incoming [ButtplugClientDevice] messages.
  device_message_receiver: Receiver<ButtplugClientMessageFuturePair>,
  /// Event sender for specific devices.
  ///
  /// We can have many instances of the same [ButtplugClientDevice]. This map
  /// allows us to send messages to all device instances that refer to the same
  /// device index on the server.
  device_event_senders: HashMap<u32, BroadcastChannel<ButtplugClientDeviceEvent>>,
  /// Sends events to the [ButtplugClient] instance.
  event_sender: BroadcastChannel<ButtplugClientEvent>,
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
    client_receiver: Receiver<ButtplugClientRequest>,
  ) -> Self {
    let (device_message_sender, device_message_receiver) =
      bounded::<ButtplugClientMessageFuturePair>(256);
    Self {
      devices: HashMap::new(),
      device_event_senders: HashMap::new(),
      device_message_sender,
      device_message_receiver,
      event_sender,
      client_receiver,
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
    // If we don't have an entry in the map for the channel, add it. Otherwise,
    // push it on the vector.
    //
    let event_receiver = self
      .device_event_senders
      .entry(info.device_index)
      .or_insert_with(BroadcastChannel::new)
      .clone();
    ButtplugClientDevice::from((info, self.device_message_sender.clone(), event_receiver))
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
        self.devices.insert(dev.device_index, info);
        self
          .send_client_event(&ButtplugClientEvent::DeviceAdded(device))
          .await;
      }
      ButtplugCurrentSpecServerMessage::DeviceList(dev) => {
        for d in &dev.devices {
          let device = self.create_client_device(&d);
          self.devices.insert(d.device_index, d.clone());
          self
            .send_client_event(&ButtplugClientEvent::DeviceAdded(device))
            .await;
        }
      }
      ButtplugCurrentSpecServerMessage::DeviceRemoved(dev) => {
        let info = self.devices.remove(&dev.device_index);
        self.device_event_senders.remove(&dev.device_index);
        self
          .send_client_event(&ButtplugClientEvent::DeviceRemoved(info.unwrap()))
          .await;
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
      ButtplugClientRequest::RequestDeviceList(fut) => {
        debug!("Building device list!");
        let mut device_return = vec![];
        // TODO There has to be a way to do this without the clone()
        for device in self.devices.clone().values() {
          let client_device = self.create_client_device(device);
          device_return.push(client_device);
        }
        debug!("Returning device list of {} items!", device_return.len());
        fut.set_reply(device_return);
        true
      }
      ButtplugClientRequest::HandleDeviceList(device_list) => {
        debug!("Handling device list!");
        for d in &device_list.devices {
          let device = self.create_client_device(&d);
          self.devices.insert(d.device_index, d.clone());
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
    let mut device_receiver = self.device_message_receiver.clone();
    loop {
      let stream_return = select! {
        event = connector_receiver.next().fuse() => match event {
          None => {
            debug!("Connector disconnected.");
            StreamReturn::Disconnect
          }
          Some(msg) => StreamReturn::ConnectorMessage(msg),
        },
        client = client_receiver.next().fuse() => match client {
          None => {
            debug!("Client disconnected.");
            StreamReturn::Disconnect
          }
          Some(msg) => StreamReturn::ClientRequest(msg),
        },
        device = device_receiver.next().fuse() => match device {
          None => {
            // Since we hold a reference to the sender so we can
            // redistribute it when creating devices, we'll never
            // actually do this.
            panic!("We should never get here.");
          }
          Some(msg) => StreamReturn::DeviceMessage(msg),
        }
      };
      match stream_return {
        StreamReturn::ConnectorMessage(msg) => self.parse_connector_message(msg).await,
        StreamReturn::ClientRequest(msg) => {
          if !self.parse_client_message(msg).await {
            break;
          }
        }
        StreamReturn::DeviceMessage(msg_fut) => {
          // TODO Check whether we actually are still connected to
          // this device.
          self.send_message(msg_fut).await;
        }
        StreamReturn::Disconnect => {
          info!("Disconnected!");
          break;
        }
      }
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
  client_receiver: Receiver<ButtplugClientRequest>,
) -> (
  BoxFuture<'static, Result<(), ButtplugClientError>>,
  impl StreamExt<Item = ButtplugClientEvent> + Clone,
) {
  let event_sender = broadcaster::BroadcastChannel::new();
  let event_loop_sender = event_sender.clone();
  (
    Box::pin(async {
      info!("Starting client event loop.");
      ButtplugClientEventLoop::new(
        connector,
        connector_receiver,
        event_loop_sender,
        client_receiver,
      )
      .run()
      .await;
      info!("Stopping client event loop.");
      Ok(())
    }),
    event_sender,
  )
}
