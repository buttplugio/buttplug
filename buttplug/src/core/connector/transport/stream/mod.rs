// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Stream based transport, used in cases where we may need to hop FFI boundaries within the same
//! process space.

use crate::{
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
use futures::{
  future::{self, BoxFuture},
  FutureExt,
};

use std::sync::Arc;
use tokio::{
  select,
  sync::{
    mpsc::{Receiver, Sender},
    Mutex,
  },
};

#[derive(Debug)]
pub struct ButtplugStreamTransport {
  sender: Sender<ButtplugSerializedMessage>,
  receiver: Arc<Mutex<Option<Receiver<ButtplugSerializedMessage>>>>,
}

impl ButtplugStreamTransport {
  pub fn new(
    sender: Sender<ButtplugSerializedMessage>,
    receiver: Receiver<ButtplugSerializedMessage>,
  ) -> Self {
    Self {
      sender,
      receiver: Arc::new(Mutex::new(Some(receiver))),
    }
  }
}

impl ButtplugConnectorTransport for ButtplugStreamTransport {
  fn connect(
    &self,
    mut outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let incoming_recv = self.receiver.clone();
    let sender = self.sender.clone();
    async move {
      let mut incoming_recv = incoming_recv.lock().await.take().unwrap();
      async_manager::spawn(async move {
        loop {
          select! {
            msg = outgoing_receiver.recv() => {
              match msg {
                Some(m) => {
                  if sender.send(m).await.is_err() {
                    break;
                  }
                }
                None => break
              }
            },
            msg = incoming_recv.recv() => {
              match msg {
                Some(m) => {
                  if incoming_sender.send(ButtplugTransportIncomingMessage::Message(m)).await.is_err() {
                    break;
                  }
                }
                None => break
              }
            }
          }
        }
      });
      Ok(())
    }.boxed()
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    future::ready(Ok(())).boxed()
  }
}
