use buttplug::{
  core::{
    connector::{
      transport::{ButtplugConnectorTransport, ButtplugTransportIncomingMessage},
      ButtplugConnectorError,
      ButtplugConnectorResultFuture,
    },
    message::serializer::ButtplugSerializedMessage,
  },
  util::async_manager,
};
use futures::{future::BoxFuture, FutureExt};
use std::sync::Arc;
use tokio::{
  select,
  sync::{
    mpsc::{Receiver, Sender},
    Mutex,
    Notify,
  },
};

pub struct ChannelTransport {
  external_sender: Sender<ButtplugSerializedMessage>,
  external_receiver: Arc<Mutex<Option<Receiver<ButtplugSerializedMessage>>>>,
  disconnect_notifier: Arc<Notify>,
}

impl ChannelTransport {
  pub fn new(
    disconnect_notifier: &Arc<Notify>,
    external_sender: Sender<ButtplugSerializedMessage>,
    external_receiver: Receiver<ButtplugSerializedMessage>,
  ) -> Self {
    Self {
      disconnect_notifier: disconnect_notifier.clone(),
      external_sender,
      external_receiver: Arc::new(Mutex::new(Some(external_receiver))),
    }
  }
}

impl ButtplugConnectorTransport for ChannelTransport {
  fn connect(
    &self,
    mut outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let notifier = self.disconnect_notifier.clone();
    let external_sender = self.external_sender.clone();
    let receiver_clone = self.external_receiver.clone();
    async move {
      async_manager::spawn(async move {
        let mut receiver = receiver_clone.lock().await.take().expect("Should only run once");
        loop {
          select! {
            _ = notifier.notified() => {
              break;
            },
            outgoing_msg = outgoing_receiver.recv() => {
              if let Some(msg) = outgoing_msg {
                external_sender.send(msg).await.expect("Should exist");
              } else {
                break;
              }
            },
            incoming_msg = receiver.recv() => {
              if let Some(msg) = incoming_msg {
                incoming_sender.send(ButtplugTransportIncomingMessage::Message(msg)).await.expect("Should exist");
              } else {
                break;
              }
            }
          };
        }
      });
      Ok(())
    }
    .boxed()
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    let disconnect_notifier = self.disconnect_notifier;
    async move {
      disconnect_notifier.notify_waiters();
      Ok(())
    }
    .boxed()
  }
}
