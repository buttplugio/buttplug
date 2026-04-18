// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::message::serializer::{ButtplugMessageSerializer, ButtplugSerializedMessage};
use buttplug_server::{
  ButtplugServer, ButtplugServerBuilder,
  device::ServerDeviceManagerBuilder,
  message::{ButtplugServerMessageVariant, serializer::ButtplugServerJSONSerializer},
};
use buttplug_server_device_config::load_protocol_configs;
use buttplug_server_hwmgr_webbluetooth::WebBluetoothCommunicationManagerBuilder;
use console_error_panic_hook;
use js_sys::Uint8Array;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tracing_subscriber::{Registry, layer::SubscriberExt};
use tracing_wasm::{WASMLayer, WASMLayerConfig};
use wasm_bindgen::prelude::*;

type FFICallback = js_sys::Function;

/// Context holding a running embedded Buttplug server and its serializer.
///
/// The serializer carries version-negotiation state across calls, so it must be
/// shared rather than re-created per message.
pub struct ButtplugWASMServer {
  server: Arc<ButtplugServer>,
  serializer: Arc<ButtplugServerJSONSerializer>,
}

fn send_server_message(
  msg: &ButtplugServerMessageVariant,
  serializer: &ButtplugServerJSONSerializer,
  callback: &FFICallback,
) {
  if let ButtplugSerializedMessage::Text(text) = serializer.serialize(&[msg.clone()]) {
    let buf = text.as_bytes();
    let this = JsValue::null();
    let uint8buf = unsafe { Uint8Array::new(&Uint8Array::view(buf)) };
    let _ = callback.call1(&this, &JsValue::from(uint8buf));
  }
}

#[wasm_bindgen]
pub fn buttplug_create_embedded_wasm_server(callback: &FFICallback) -> *mut ButtplugWASMServer {
  console_error_panic_hook::set_once();

  let dcm = Arc::new(
    load_protocol_configs(&None, &None, false)
      .unwrap()
      .finish()
      .unwrap(),
  );
  let webbluetooth_builder = WebBluetoothCommunicationManagerBuilder::new(dcm.clone());
  let device_manager = ServerDeviceManagerBuilder::new_with_arc(dcm)
    .comm_manager(webbluetooth_builder)
    .finish()
    .unwrap();
  let server = Arc::new(ButtplugServerBuilder::new(device_manager).finish().unwrap());

  let serializer = Arc::new(ButtplugServerJSONSerializer::default());

  let event_stream = server.event_stream();
  let callback_clone = callback.clone();
  let serializer_clone = serializer.clone();

  wasm_bindgen_futures::spawn_local(async move {
    futures::pin_mut!(event_stream);
    while let Some(msg) = event_stream.next().await {
      send_server_message(&msg, &serializer_clone, &callback_clone);
    }
  });

  Box::into_raw(Box::new(ButtplugWASMServer { server, serializer }))
}

#[wasm_bindgen]
pub fn buttplug_free_embedded_wasm_server(ptr: *mut ButtplugWASMServer) {
  if !ptr.is_null() {
    unsafe {
      let _ = Box::from_raw(ptr);
    }
  }
}

#[wasm_bindgen]
pub fn buttplug_client_send_json_message(
  server_ptr: *mut ButtplugWASMServer,
  buf: &[u8],
  callback: &FFICallback,
) {
  let ctx = unsafe {
    assert!(!server_ptr.is_null());
    &*server_ptr
  };
  let server = ctx.server.clone();
  let serializer = ctx.serializer.clone();
  let callback = callback.clone();

  let text = std::str::from_utf8(buf).unwrap().to_owned();
  let inbound = match serializer.deserialize(&ButtplugSerializedMessage::Text(text)) {
    Ok(msgs) => msgs,
    Err(e) => {
      tracing::error!("Failed to deserialize client message: {:?}", e);
      return;
    }
  };

  wasm_bindgen_futures::spawn_local(async move {
    for msg in inbound {
      let response = match server.parse_message(msg).await {
        Ok(r) => r,
        Err(e) => e,
      };
      send_server_message(&response, &serializer, &callback);
    }
  });
}

#[wasm_bindgen]
pub fn buttplug_activate_env_logger(_max_level: &str) {
  let _ = tracing::subscriber::set_global_default(
    Registry::default().with(WASMLayer::new(WASMLayerConfig::default())),
  );
}
