// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug::{
  client::ButtplugClientError,
  core::{
    connector::transport::ButtplugTransportIncomingMessage,
    errors::{ButtplugError, ButtplugUnknownError},
    message::{
      self,
      serializer::ButtplugSerializedMessage,
      ButtplugClientMessage,
      ButtplugMessage,
      ButtplugServerMessage,
    },
  },
  util::async_manager,
};
use std::sync::Arc;
use tokio::sync::Notify;
use util::ChannelClientTestHelper;

#[tokio::test]
async fn test_garbled_client_rsi_response() {
  let helper = Arc::new(ChannelClientTestHelper::new());
  let helper_clone = helper.clone();
  let finish_notifier = Arc::new(Notify::new());
  let finish_notifier_clone = finish_notifier.clone();
  async_manager::spawn(async move {
    helper_clone
      .connect_without_reply()
      .await
      .expect("Test, assuming infallible.");
    finish_notifier_clone.notify_waiters();
  });
  // Just assume we get an RSI message
  let _ = helper.recv_outgoing().await;
  // Send back crap.
  helper
    .send_incoming(ButtplugTransportIncomingMessage::Message(
      ButtplugSerializedMessage::Text("Not the JSON we're expecting".to_owned()),
    ))
    .await;
  helper
    .send_client_incoming(
      message::ServerInfo::new(
        "test server",
        message::BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
        0,
      )
      .into(),
    )
    .await;
  let _ = helper.recv_outgoing().await;
  let mut dl = message::DeviceList::new(vec![]);
  dl.set_id(2);
  helper.send_client_incoming(dl.into()).await;
  finish_notifier.notified().await;
}

#[tokio::test]
async fn test_serialized_error_relay() {
  let helper = Arc::new(ChannelClientTestHelper::new());
  helper.simulate_successful_connect().await;
  let helper_clone = helper.clone();
  async_manager::spawn(async move {
    assert!(matches!(
      helper_clone.next_client_message().await,
      ButtplugClientMessage::StartScanning(..)
    ));
    let mut error_msg = ButtplugServerMessage::Error(message::Error::from(ButtplugError::from(
      ButtplugUnknownError::NoDeviceCommManagers,
    )));
    error_msg.set_id(3);
    helper_clone.send_client_incoming(error_msg).await;
  });
  assert!(matches!(
    helper.client().start_scanning().await.unwrap_err(),
    ButtplugClientError::ButtplugError(ButtplugError::ButtplugUnknownError(
      buttplug::core::errors::ButtplugUnknownError::NoDeviceCommManagers
    ))
  ));
}

// TODO Test bad incoming JSON
// TODO Test deserialization of concatenated messages
// TODO Test message with negative message id
// TODO Test device message with negative device id
