// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Implementation of internal Buttplug Client event loop.

use super::{
  connectors::{
    ButtplugClientConnectionStateShared,
    ButtplugClientConnector,
    ButtplugClientConnectorError,
  },
  device::ButtplugClientDevice,
  ButtplugClientEvent,
  ButtplugClientResult,
  ButtplugClientMessageFuturePair,
};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{ButtplugClientOutMessage, DeviceList, DeviceMessageInfo}
  },
  util::future::{ButtplugFutureStateShared},
};
use async_std::{
  prelude::{FutureExt, StreamExt},
  sync::{channel, Receiver, Sender},
};
use std::collections::HashMap;

/// Enum used for communication from the client to the event loop.
pub enum ButtplugClientMessage {
  /// Client request to connect, via the included connector instance.
  ///
  /// Once connection is finished, use the bundled future to resolve.
  Connect(
    Box<dyn ButtplugClientConnector>,
    ButtplugClientConnectionStateShared,
  ),
  /// Client request to disconnect, via already sent connector instance.
  Disconnect(ButtplugClientConnectionStateShared),
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

pub enum ButtplugClientDeviceEvent {
  DeviceDisconnect,
  ClientDisconnect,
  Message(ButtplugClientOutMessage),
}

enum StreamReturn {
  ConnectorMessage(ButtplugClientOutMessage),
  ClientMessage(ButtplugClientMessage),
  DeviceMessage(ButtplugClientMessageFuturePair),
  Disconnect,
}

struct ButtplugClientEventLoop {
  devices: HashMap<u32, DeviceMessageInfo>,
  device_message_sender: Sender<ButtplugClientMessageFuturePair>,
  device_message_receiver: Receiver<ButtplugClientMessageFuturePair>,
  device_event_senders: HashMap<u32, Vec<Sender<ButtplugClientDeviceEvent>>>,
  event_sender: Sender<ButtplugClientEvent>,
  client_receiver: Receiver<ButtplugClientMessage>,
  connector: Box<dyn ButtplugClientConnector>,
  connector_receiver: Receiver<ButtplugClientOutMessage>,
}

impl ButtplugClientEventLoop {
  pub async fn wait_for_connector(
    event_sender: Sender<ButtplugClientEvent>,
    mut client_receiver: Receiver<ButtplugClientMessage>,
  ) -> Result<Self, ButtplugClientConnectorError> {
    match client_receiver.next().await {
      None => {
        debug!("Client disconnected.");
        Err(ButtplugClientConnectorError::new(
          "Client was dropped during connect.",
        ))
      }
      Some(msg) => match msg {
        ButtplugClientMessage::Connect(mut connector, state) => match connector.connect().await {
          Err(err) => {
            error!("Cannot connect to server: {}", err.message);
            let mut waker_state = state.try_lock().expect("Future locks should never be in contention");
            let reply = Err(ButtplugClientConnectorError::new(&format!(
              "Cannot connect to server: {}",
              err.message
            )));
            waker_state.set_reply(reply);
            Err(ButtplugClientConnectorError::new(
              "Client couldn't connect to server.",
            ))
          }
          Ok(_) => {
            info!("Connected!");
            let mut waker_state = state.try_lock().expect("Future locks should never be in contention");
            waker_state.set_reply(Ok(()));
            let (device_message_sender, device_message_receiver) =
              channel::<ButtplugClientMessageFuturePair>(256);
            Ok(ButtplugClientEventLoop {
              devices: HashMap::new(),
              device_event_senders: HashMap::new(),
              device_message_sender,
              device_message_receiver,
              event_sender,
              client_receiver,
              connector_receiver: connector.get_event_receiver(),
              connector,
            })
          }
        },
        _ => {
          error!("Received non-connector message before connector message.");
          Err(ButtplugClientConnectorError::new(
            "Event Loop did not receive Connect message first.",
          ))
        }
      },
    }
  }

  fn create_client_device(&mut self, info: &DeviceMessageInfo) -> ButtplugClientDevice {
    let (event_sender, event_receiver) = channel(256);
    self
      .device_event_senders
      .entry(info.device_index)
      .or_insert_with(|| vec![])
      .push(event_sender);
    ButtplugClientDevice::from((info, self.device_message_sender.clone(), event_receiver))
  }

  async fn parse_connector_message(&mut self, msg: ButtplugClientOutMessage) {
    info!("Sending message to clients.");
    match &msg {
      ButtplugClientOutMessage::DeviceAdded(dev) => {
        let info = DeviceMessageInfo::from(dev);
        let device = self.create_client_device(&info);
        self.devices.insert(dev.device_index, info);
        self
          .event_sender
          .send(ButtplugClientEvent::DeviceAdded(device))
          .await;
      }
      ButtplugClientOutMessage::DeviceList(dev) => {
        for d in &dev.devices {
          let device = self.create_client_device(&d);
          self.devices.insert(d.device_index, d.clone());
          self
            .event_sender
            .send(ButtplugClientEvent::DeviceAdded(device))
            .await;
        }
      }
      ButtplugClientOutMessage::DeviceRemoved(dev) => {
        let info = self.devices.remove(&dev.device_index);
        self.device_event_senders.remove(&dev.device_index);
        self
          .event_sender
          .send(ButtplugClientEvent::DeviceRemoved(info.unwrap()))
          .await;
      }
      _ => panic!("Got connector message type we don't know how to handle!"),
    }
  }

  async fn send_message(&mut self, msg_fut: ButtplugClientMessageFuturePair) {
    let mut waker = msg_fut.waker.try_lock().expect("Future locks should never be in contention");
    waker.set_reply(self.connector.send(msg_fut.msg).await);
  }

  // TODO Why does this return bool and not something more informative?
  async fn parse_client_message(&mut self, msg: ButtplugClientMessage) -> bool {
    debug!("Parsing a client message.");
    match msg {
      ButtplugClientMessage::Message(msg_fut) => {
        debug!("Sending message through connector.");
        self.send_message(msg_fut).await;
        true
      }
      ButtplugClientMessage::Disconnect(state) => {
        info!("Client requested disconnect");
        let mut waker_state = state.try_lock().expect("Future locks should never be in contention");
        waker_state.set_reply(self.connector.disconnect().await);
        false
      }
      ButtplugClientMessage::RequestDeviceList(fut) => {
        info!("Building device list!");
        let mut r = vec![];
        // TODO There's probably a better way to do this.
        let devices = self.devices.clone();
        for d in devices.values() {
          let dev = self.create_client_device(d);
          r.push(dev);
        }
        info!("Returning device list of {} items!", r.len());
        let mut waker_state = fut.try_lock().expect("Future locks should never be in contention");
        waker_state.set_reply(r);
        info!("Finised setting waker!");
        true
      }
      ButtplugClientMessage::HandleDeviceList(device_list) => {
        info!("Handling device list!");
        for d in &device_list.devices {
          let device = self.create_client_device(&d);
          self.devices.insert(d.device_index, d.clone());
          self
            .event_sender
            .send(ButtplugClientEvent::DeviceAdded(device))
            .await;
        }
        true
      }
      // TODO Do something other than panic if someone does
      // something like trying to connect twice..
      _ => panic!("Client message not handled!"),
    }
  }

  pub async fn run(&mut self) {
    // Once connected, wait for messages from either the client or the
    // connector, and send them the direction they're supposed to go.
    let mut client_receiver = self.client_receiver.clone();
    let mut connector_receiver = self.connector_receiver.clone();
    let mut device_receiver = self.device_message_receiver.clone();
    loop {
      let client_future = async {
        match client_receiver.next().await {
          None => {
            debug!("Client disconnected.");
            StreamReturn::Disconnect
          }
          Some(msg) => StreamReturn::ClientMessage(msg),
        }
      };
      let event_future = async {
        match connector_receiver.next().await {
          None => {
            debug!("Connector disconnected.");
            StreamReturn::Disconnect
          }
          Some(msg) => StreamReturn::ConnectorMessage(msg),
        }
      };
      let device_future = async {
        match device_receiver.next().await {
          None => {
            // Since we hold a reference to the sender so we can
            // redistribute it when creating devices, we'll never
            // actually do this.
            panic!("We should never get here.");
          }
          Some(msg) => StreamReturn::DeviceMessage(msg),
        }
      };

      let stream_fut = event_future.race(client_future).race(device_future);
      match stream_fut.await {
        StreamReturn::ConnectorMessage(msg) => self.parse_connector_message(msg).await,
        StreamReturn::ClientMessage(msg) => {
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
/// handles connection and communication with the server, and creation of events
/// received from the server.
///
/// The event_loop does a few different things during its lifetime.
///
/// - The first thing it will do is wait for a Connect message from a
/// client. This message contains a [ButtplugClientConnector] that will be
/// used to connect and communicate with a [crate::server::ButtplugServer].
///
/// - After a connection is established, it will listen for events from the
/// connector, or messages from the client, until either server/client
/// disconnects.
///
/// - Finally, on disconnect, it will tear down, and cannot be used again.
/// All clients and devices associated with the loop will be invalidated,
/// and a new [super::ButtplugClient] must be created.
///
/// # Parameters
///
/// - `event_sender`: Used when sending server updates to clients.
/// - `client_receiver`: Used when receiving commands from clients to
/// send to server.
pub async fn client_event_loop(
  event_sender: Sender<ButtplugClientEvent>,
  client_receiver: Receiver<ButtplugClientMessage>,
) -> ButtplugClientResult {
  info!("Starting client event loop.");
  ButtplugClientEventLoop::wait_for_connector(event_sender, client_receiver)
    .await?
    .run()
    .await;
  info!("Exiting client event loop");
  Ok(())
}
