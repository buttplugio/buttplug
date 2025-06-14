// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#![allow(dead_code)]

use crate::util::ButtplugTestServer;
use buttplug::{
  client::{
    connector::ButtplugRemoteClientConnector,
    serializer::ButtplugClientJSONSerializer,
    ButtplugClient,
    ButtplugClientError,
  },
  core::{
    connector::{
      transport::{ButtplugConnectorTransport, ButtplugTransportIncomingMessage},
      ButtplugConnectorError,
    },
    message::{
      serializer::{ButtplugMessageSerializer, ButtplugSerializedMessage},
      ButtplugClientMessageV4,
      ButtplugMessage,
      DeviceListV4,
      RequestServerInfoV4,
      ServerInfoV4,
      BUTTPLUG_CURRENT_API_MAJOR_VERSION,
      BUTTPLUG_CURRENT_API_MINOR_VERSION,
    },
  },
  server::{
    connector::ButtplugRemoteServerConnector,
    message::{
      serializer::ButtplugServerJSONSerializer,
      ButtplugClientMessageVariant,
      ButtplugServerMessageVariant,
    },
  },
  util::async_manager,
};
use futures::{
  future::{self, BoxFuture},
  select,
  FutureExt,
};
use log::*;
use std::sync::Arc;
use tokio::sync::{
  mpsc::{channel, Receiver, Sender},
  Mutex,
  Notify,
};

struct ChannelTransport {
  outside_receiver: Arc<Mutex<Option<Receiver<ButtplugTransportIncomingMessage>>>>,
  outside_sender: Sender<ButtplugSerializedMessage>,
  disconnect_notifier: Arc<Notify>,
}

impl ChannelTransport {
  pub fn new(
    outside_receiver: Receiver<ButtplugTransportIncomingMessage>,
    outside_sender: Sender<ButtplugSerializedMessage>,
  ) -> Self {
    Self {
      outside_receiver: Arc::new(Mutex::new(Some(outside_receiver))),
      outside_sender,
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }
}

impl ButtplugConnectorTransport for ChannelTransport {
  fn connect(
    &self,
    mut outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let disconnect_notifier = self.disconnect_notifier.clone();
    let outside_sender = self.outside_sender.clone();
    let outside_receiver_mutex = self.outside_receiver.clone();
    async_manager::spawn(async move {
      let mut outside_receiver = outside_receiver_mutex
        .lock()
        .await
        .take()
        .expect("Test, assuming infallible");
      loop {
        select! {
          _ = disconnect_notifier.notified().fuse() => {
            info!("Test requested disconnect.");
            return;
          }
          outgoing = outgoing_receiver.recv().fuse() => {
            if let Some(o) = outgoing {
              outside_sender.send(o).await.expect("Test, assuming infallible");
            } else {
              info!("Test dropped stream, returning");
              return;
            }
          }
          incoming = outside_receiver.recv().fuse() => {
            if let Some(i) = incoming {
              incoming_sender.send(i).await.expect("Test, assuming infallible");
            } else {
              info!("Test dropped stream, returning");
              return;
            }
          }
        };
      }
    });
    future::ready(Ok(())).boxed()
  }

  fn disconnect(self) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    self.disconnect_notifier.notify_waiters();
    future::ready(Ok(())).boxed()
  }
}

pub struct ChannelClientTestHelper {
  client: Arc<ButtplugClient>,
  sender: Sender<ButtplugTransportIncomingMessage>,
  receiver: Arc<Mutex<Receiver<ButtplugSerializedMessage>>>,
  connector: Arc<Mutex<Option<ButtplugRemoteClientConnector<ChannelTransport>>>>,
  server_serializer: ButtplugServerJSONSerializer,
  client_serializer: ButtplugClientJSONSerializer,
}

impl Default for ChannelClientTestHelper {
  fn default() -> Self {
    Self::new()
  }
}

impl ChannelClientTestHelper {
  pub fn new() -> Self {
    let client = Arc::new(ButtplugClient::new("test client"));
    let (incoming_sender, incoming_receiver) = channel(256);
    let (outgoing_sender, outgoing_receiver) = channel(256);
    let connector = Arc::new(Mutex::new(Some(ButtplugRemoteClientConnector::<
      ChannelTransport,
    >::new(ChannelTransport::new(
      incoming_receiver,
      outgoing_sender,
    )))));
    let client_serializer = ButtplugClientJSONSerializer::default();
    let rsi_setup_msg = client_serializer.serialize(&[RequestServerInfoV4::new(
      "Test client",
      BUTTPLUG_CURRENT_API_MAJOR_VERSION,
      BUTTPLUG_CURRENT_API_MINOR_VERSION,
    )
    .into()]);
    let server_serializer = ButtplugServerJSONSerializer::default();
    server_serializer
      .deserialize(&rsi_setup_msg)
      .expect("Test, assuming infallible");
    Self {
      client,
      connector,
      sender: incoming_sender,
      receiver: Arc::new(Mutex::new(outgoing_receiver)),
      client_serializer,
      server_serializer,
    }
  }

  pub fn client(&self) -> &ButtplugClient {
    &self.client
  }

  pub async fn connect_without_reply(&self) -> Result<(), ButtplugClientError> {
    let connector = self
      .connector
      .lock()
      .await
      .take()
      .expect("Test, assuming infallible");
    self.client.connect(connector).await
  }

  pub async fn simulate_successful_connect(&self) {
    let client_clone = self.client.clone();
    let connector = self
      .connector
      .lock()
      .await
      .take()
      .expect("Test, assuming infallible");
    let finish_notifier = Arc::new(Notify::new());
    let finish_notifier_clone = finish_notifier.clone();
    async_manager::spawn(async move {
      if let Err(e) = client_clone.connect(connector).await {
        assert!(false, "Error connecting to client: {:?}", e);
      }
      finish_notifier_clone.notify_waiters();
    });
    // Wait for RequestServerInfo message
    assert!(matches!(
      self.next_client_message().await,
      ButtplugClientMessageVariant::V4(ButtplugClientMessageV4::RequestServerInfo(..))
    ));
    // Just assume we get an RSI message
    self
      .send_client_incoming(ButtplugServerMessageVariant::V4(
        ServerInfoV4::new("test server", BUTTPLUG_CURRENT_API_MAJOR_VERSION, 0, 0).into(),
      ))
      .await;
    // Wait for RequestDeviceList message.
    assert!(matches!(
      self.next_client_message().await,
      ButtplugClientMessageVariant::V4(ButtplugClientMessageV4::RequestDeviceList(..))
    ));
    let mut dl = DeviceListV4::new(vec![]);
    dl.set_id(2);
    self
      .send_client_incoming(ButtplugServerMessageVariant::V4(dl.into()))
      .await;
    finish_notifier.notified().await;
  }

  pub async fn next_client_message(&self) -> ButtplugClientMessageVariant {
    self
      .server_serializer
      .deserialize(
        &self
          .recv_outgoing()
          .await
          .expect("Test, assuming infallible"),
      )
      .expect("Test, assuming infallible")[0]
      .clone()
  }

  pub async fn recv_outgoing(&self) -> Option<ButtplugSerializedMessage> {
    // If this ever conflicts, its the tests fault, so just panic.
    self
      .receiver
      .try_lock()
      .expect("Test, assuming infallible")
      .recv()
      .await
  }

  pub async fn send_incoming(&self, msg: ButtplugTransportIncomingMessage) {
    self
      .sender
      .send(msg)
      .await
      .expect("Test, assuming infallible");
  }

  pub async fn send_client_incoming(&self, msg: ButtplugServerMessageVariant) {
    self
      .send_incoming(ButtplugTransportIncomingMessage::Message(
        self.server_serializer.serialize(&vec![msg]),
      ))
      .await;
  }

  pub async fn send_server_incoming(&self, msg: ButtplugClientMessageV4) {
    self
      .send_incoming(ButtplugTransportIncomingMessage::Message(
        self.client_serializer.serialize(&[msg]),
      ))
      .await;
  }
}

pub struct ChannelServerTestHelper {
  server: Arc<ButtplugTestServer>,
  sender: Sender<ButtplugTransportIncomingMessage>,
  receiver: Arc<Mutex<Receiver<ButtplugSerializedMessage>>>,
  connector: Arc<
    Mutex<Option<ButtplugRemoteServerConnector<ChannelTransport, ButtplugServerJSONSerializer>>>,
  >,
  server_serializer: ButtplugServerJSONSerializer,
  client_serializer: ButtplugClientJSONSerializer,
}

impl Default for ChannelServerTestHelper {
  fn default() -> Self {
    Self::new()
  }
}

impl ChannelServerTestHelper {
  pub fn new() -> Self {
    let server = Arc::new(ButtplugTestServer::default());
    let (incoming_sender, incoming_receiver) = channel(256);
    let (outgoing_sender, outgoing_receiver) = channel(256);
    let connector = Arc::new(Mutex::new(Some(ButtplugRemoteServerConnector::<
      ChannelTransport,
      ButtplugServerJSONSerializer,
    >::new(ChannelTransport::new(
      incoming_receiver,
      outgoing_sender,
    )))));
    let client_serializer = ButtplugClientJSONSerializer::default();
    let server_serializer = ButtplugServerJSONSerializer::default();
    Self {
      server,
      connector,
      sender: incoming_sender,
      receiver: Arc::new(Mutex::new(outgoing_receiver)),
      client_serializer,
      server_serializer,
    }
  }

  pub fn server(&self) -> &ButtplugTestServer {
    &self.server
  }

  pub async fn recv_outgoing(&self) -> Option<ButtplugSerializedMessage> {
    // If this ever conflicts, its the tests fault, so just panic.
    self
      .receiver
      .try_lock()
      .expect("Test, assuming infallible")
      .recv()
      .await
  }

  pub async fn send_incoming(&self, msg: ButtplugTransportIncomingMessage) {
    self
      .sender
      .send(msg)
      .await
      .expect("Test, assuming infallible");
  }

  pub async fn send_client_incoming(&self, msg: ButtplugServerMessageVariant) {
    self
      .send_incoming(ButtplugTransportIncomingMessage::Message(
        self.server_serializer.serialize(&vec![msg]),
      ))
      .await;
  }

  pub async fn send_server_incoming(&self, msg: ButtplugClientMessageV4) {
    self
      .send_incoming(ButtplugTransportIncomingMessage::Message(
        self.client_serializer.serialize(&[msg]),
      ))
      .await;
  }
}
