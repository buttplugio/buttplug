#![allow(dead_code)]

use buttplug::{
  client::{ButtplugClient, ButtplugClientError},
  connector::{
    transport::{ButtplugConnectorTransport, ButtplugTransportIncomingMessage},
    ButtplugConnectorError, ButtplugRemoteClientConnector, ButtplugRemoteServerConnector
  },
  core::{
    messages::{
      self,
      serializer::{
        ButtplugClientJSONSerializer, ButtplugSerializedMessage, ButtplugServerJSONSerializer,
      },
      ButtplugMessage, ButtplugServerMessage, ButtplugClientMessage,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION, ButtplugCurrentSpecClientMessage, serializer::ButtplugMessageSerializer
    },
  },
  server::{ButtplugRemoteServer},
  util::async_manager,
};
use futures::{
  future::{self, BoxFuture},
  select, FutureExt,
};
use std::sync::Arc;
use tokio::sync::{
  mpsc::{channel, Receiver, Sender},
  Mutex, Notify,
};
use tracing::*;

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
      let mut outside_receiver = outside_receiver_mutex.lock().await.take().unwrap();
      loop {
        select! {
          _ = disconnect_notifier.notified().fuse() => {
            info!("Test requested disconnect.");
            return;
          }
          outgoing = outgoing_receiver.recv().fuse() => {
            if let Some(o) = outgoing {
              outside_sender.send(o).await.unwrap();
            } else {
              info!("Test dropped stream, returning");
              return;
            }
          }
          incoming = outside_receiver.recv().fuse() => {
            if let Some(i) = incoming {
              incoming_sender.send(i).await.unwrap();
            } else {
              info!("Test dropped stream, returning");
              return;
            }
          }
        };
      }
    })
    .unwrap();
    Box::pin(future::ready(Ok(())))
  }

  fn disconnect(self) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    self.disconnect_notifier.notify_waiters();
    Box::pin(future::ready(Ok(())))
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
    let rsi_setup_msg = client_serializer.serialize(vec![messages::RequestServerInfo::new(
      "Test client",
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    )
    .into()]);
    let server_serializer = ButtplugServerJSONSerializer::default();
    server_serializer.deserialize(rsi_setup_msg).unwrap();
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
    let connector = self.connector.lock().await.take().unwrap();
    self.client.connect(connector).await
  }

  pub async fn simulate_successful_connect(&self) {
    let client_clone = self.client.clone();
    let connector = self.connector.lock().await.take().unwrap();
    let finish_notifier = Arc::new(Notify::new());
    let finish_notifier_clone = finish_notifier.clone();
    async_manager::spawn(async move {
      if let Err(e) = client_clone.connect(connector).await {
        assert!(false, "Error connecting to client: {:?}", e);
      }
      finish_notifier_clone.notify_waiters();
    })
    .unwrap();
    // Wait for RequestServerInfo message
    assert!(matches!(self.get_next_client_message().await, ButtplugClientMessage::RequestServerInfo(..)));
    // Just assume we get an RSI message
    self
      .send_client_incoming(
        messages::ServerInfo::new(
          "test server",
          messages::BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
          0,
        )
        .into(),
      )
      .await;
    // Wait for RequestDeviceList message.
    assert!(matches!(self.get_next_client_message().await, ButtplugClientMessage::RequestDeviceList(..)));
    let mut dl = messages::DeviceList::new(vec![]);
    dl.set_id(2);
    self.send_client_incoming(dl.into()).await;
    finish_notifier.notified().await;
  }

  pub async fn get_next_client_message(&self) -> ButtplugClientMessage {
    self.server_serializer.deserialize(self.recv_outgoing().await.unwrap()).unwrap()[0].clone()
  }

  pub async fn recv_outgoing(&self) -> Option<ButtplugSerializedMessage> {
    // If this ever conflicts, its the tests fault, so just panic.
    self.receiver.try_lock().unwrap().recv().await
  }

  pub async fn send_incoming(&self, msg: ButtplugTransportIncomingMessage) {
    self.sender.send(msg).await.unwrap();
  }

  pub async fn send_client_incoming(&self, msg: ButtplugServerMessage) {
    self
      .send_incoming(ButtplugTransportIncomingMessage::Message(
        self.server_serializer.serialize(vec![msg]),
      ))
      .await;
  }

  pub async fn send_server_incoming(&self, msg: ButtplugCurrentSpecClientMessage) {
    self
      .send_incoming(ButtplugTransportIncomingMessage::Message(
        self.client_serializer.serialize(vec![msg]),
      ))
      .await;
  }
}

pub struct ChannelServerTestHelper {
  server: Arc<ButtplugRemoteServer>,
  sender: Sender<ButtplugTransportIncomingMessage>,
  receiver: Arc<Mutex<Receiver<ButtplugSerializedMessage>>>,
  connector: Arc<Mutex<Option<ButtplugRemoteServerConnector<ChannelTransport, ButtplugServerJSONSerializer>>>>,
  server_serializer: ButtplugServerJSONSerializer,
  client_serializer: ButtplugClientJSONSerializer,
}

impl ChannelServerTestHelper {
  pub fn new() -> Self {
    let server = Arc::new(ButtplugRemoteServer::default());
    let (incoming_sender, incoming_receiver) = channel(256);
    let (outgoing_sender, outgoing_receiver) = channel(256);
    let connector = Arc::new(Mutex::new(Some(ButtplugRemoteServerConnector::<
      ChannelTransport, ButtplugServerJSONSerializer
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

  pub fn server(&self) -> &ButtplugRemoteServer {
    &self.server
  }

  pub async fn recv_outgoing(&self) -> Option<ButtplugSerializedMessage> {
    // If this ever conflicts, its the tests fault, so just panic.
    self.receiver.try_lock().unwrap().recv().await
  }

  pub async fn send_incoming(&self, msg: ButtplugTransportIncomingMessage) {
    self.sender.send(msg).await.unwrap();
  }

  pub async fn send_client_incoming(&self, msg: ButtplugServerMessage) {
    self
      .send_incoming(ButtplugTransportIncomingMessage::Message(
        self.server_serializer.serialize(vec![msg]),
      ))
      .await;
  }

  pub async fn send_server_incoming(&self, msg: ButtplugCurrentSpecClientMessage) {
    self
      .send_incoming(ButtplugTransportIncomingMessage::Message(
        self.client_serializer.serialize(vec![msg]),
      ))
      .await;
  }
}