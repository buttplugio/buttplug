// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::{
  Hardware, HardwareConnector, HardwareEvent, HardwareInternal, HardwareReadCmd, HardwareReading,
  HardwareSpecializer, HardwareSubscribeCmd, HardwareUnsubscribeCmd, HardwareWriteCmd,
};
use buttplug_server_device_config::{
  BluetoothLESpecifier, Endpoint, ProtocolCommunicationSpecifier,
};
use futures::future::{self, BoxFuture, FutureExt};
use js_sys::{DataView, Uint8Array};
use std::{
  collections::HashMap,
  fmt::{self, Debug},
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, error, info};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
  BluetoothDevice, BluetoothRemoteGattCharacteristic, BluetoothRemoteGattServer,
  BluetoothRemoteGattService, Event, MessageEvent,
};

struct BluetoothDeviceWrapper {
  pub device: BluetoothDevice,
}

// WASM is single-threaded; these impls are sound.
unsafe impl Send for BluetoothDeviceWrapper {}
unsafe impl Sync for BluetoothDeviceWrapper {}

pub struct WebBluetoothHardwareConnector {
  device: Option<BluetoothDeviceWrapper>,
  name: String,
}

// Holds a BluetoothDeviceWrapper; safe in WASM's single-threaded context.
unsafe impl Send for WebBluetoothHardwareConnector {}
unsafe impl Sync for WebBluetoothHardwareConnector {}

impl WebBluetoothHardwareConnector {
  pub fn new(device: BluetoothDevice) -> Self {
    let name = device.name().unwrap_or_default();
    Self {
      device: Some(BluetoothDeviceWrapper { device }),
      name,
    }
  }
}

impl Debug for WebBluetoothHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("WebBluetoothHardwareConnector")
      .field("name", &self.name)
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for WebBluetoothHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      &self.name,
      &HashMap::new(),
      &[],
    ))
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    Ok(Box::new(WebBluetoothHardwareSpecializer::new(
      self.device.take().unwrap(),
    )))
  }
}

pub struct WebBluetoothHardwareSpecializer {
  device: Option<BluetoothDeviceWrapper>,
}

// Holds a BluetoothDeviceWrapper; safe in WASM's single-threaded context.
unsafe impl Send for WebBluetoothHardwareSpecializer {}
unsafe impl Sync for WebBluetoothHardwareSpecializer {}

impl WebBluetoothHardwareSpecializer {
  fn new(device: BluetoothDeviceWrapper) -> Self {
    Self {
      device: Some(device),
    }
  }
}

#[async_trait]
impl HardwareSpecializer for WebBluetoothHardwareSpecializer {
  async fn specialize(
    &mut self,
    specifiers: &[ProtocolCommunicationSpecifier],
  ) -> Result<Hardware, ButtplugDeviceError> {
    let protocol = if let ProtocolCommunicationSpecifier::BluetoothLE(btle) = &specifiers[0] {
      btle.clone()
    } else {
      return Err(ButtplugDeviceError::DeviceCommunicationError(
        "WebBluetooth specializer received non-BLE specifier".to_owned(),
      ));
    };

    let (event_tx, mut event_rx) = mpsc::channel(256);
    let (command_tx, command_rx) = mpsc::channel(256);

    // All JS object access happens in this block. We move `device` into spawn_local
    // before any .await, keeping the outer future Send.
    let name;
    let address;
    let event_sender;
    {
      let device = self.device.take().unwrap().device;
      name = device.name().unwrap();
      address = device.id();
      let (es, _) = broadcast::channel(256);
      event_sender = es;
      let loop_fut =
        run_webbluetooth_loop(device, protocol, event_tx, event_sender.clone(), command_rx);
      spawn_local(async move {
        loop_fut.await;
      });
    }

    match event_rx.recv().await.unwrap() {
      WebBluetoothEvent::Connected(endpoints) => {
        info!("WebBluetooth device connected, returning device");
        let device_impl: Box<dyn HardwareInternal> =
          Box::new(WebBluetoothHardware::new(event_sender, command_tx));
        Ok(Hardware::new(
          &name,
          &address,
          &endpoints,
          &None,
          false,
          device_impl,
        ))
      }
      WebBluetoothEvent::Disconnected => Err(ButtplugDeviceError::DeviceCommunicationError(
        "Could not connect to WebBluetooth device".to_owned(),
      )),
    }
  }
}

#[derive(Debug, Clone)]
enum WebBluetoothEvent {
  Connected(Vec<Endpoint>),
  Disconnected,
}

enum WebBluetoothDeviceCommand {
  Write(
    HardwareWriteCmd,
    oneshot::Sender<Result<(), ButtplugDeviceError>>,
  ),
  Read(
    HardwareReadCmd,
    oneshot::Sender<Result<HardwareReading, ButtplugDeviceError>>,
  ),
  Subscribe(
    HardwareSubscribeCmd,
    oneshot::Sender<Result<(), ButtplugDeviceError>>,
  ),
  Unsubscribe(
    HardwareUnsubscribeCmd,
    oneshot::Sender<Result<(), ButtplugDeviceError>>,
  ),
}

async fn run_webbluetooth_loop(
  device: BluetoothDevice,
  btle_protocol: BluetoothLESpecifier,
  event_tx: mpsc::Sender<WebBluetoothEvent>,
  external_event_tx: broadcast::Sender<HardwareEvent>,
  mut command_rx: mpsc::Receiver<WebBluetoothDeviceCommand>,
) {
  let mut char_map: HashMap<Endpoint, BluetoothRemoteGattCharacteristic> = HashMap::new();

  let server: BluetoothRemoteGattServer =
    match JsFuture::from(device.gatt().unwrap().connect()).await {
      Ok(val) => val.unchecked_into(),
      Err(_) => {
        let _ = event_tx.send(WebBluetoothEvent::Disconnected).await;
        return;
      }
    };

  for (service_uuid, service_endpoints) in btle_protocol.services() {
    let service =
      if let Ok(serv) =
        JsFuture::from(server.get_primary_service_with_str(&service_uuid.to_string())).await
      {
        info!(
          "Service {} found on device {}",
          service_uuid,
          device.name().unwrap()
        );
        serv.unchecked_into::<BluetoothRemoteGattService>()
      } else {
        info!(
          "Service {} not found on device {}",
          service_uuid,
          device.name().unwrap()
        );
        continue;
      };

    for (chr_name, chr_uuid) in service_endpoints.iter() {
      info!("Connecting chr {} {}", chr_name, chr_uuid.to_string());
      let char: BluetoothRemoteGattCharacteristic =
        JsFuture::from(service.get_characteristic_with_str(&chr_uuid.to_string()))
          .await
          .unwrap()
          .unchecked_into();
      char_map.insert(*chr_name, char);
    }
  }

  {
    let event_sender = external_event_tx.clone();
    let id = device.id();
    let ondisconnected = Closure::wrap(Box::new(move |_: Event| {
      info!("device disconnected!");
      let _ = event_sender.send(HardwareEvent::Disconnected(id.clone()));
    }) as Box<dyn FnMut(Event)>);
    device.set_ongattserverdisconnected(Some(ondisconnected.as_ref().unchecked_ref()));
    ondisconnected.forget();
  }

  info!("WebBluetooth device created!");
  let endpoints: Vec<Endpoint> = char_map.keys().copied().collect();
  let _ = event_tx.send(WebBluetoothEvent::Connected(endpoints)).await;

  while let Some(msg) = command_rx.recv().await {
    match msg {
      WebBluetoothDeviceCommand::Write(write_cmd, reply) => {
        debug!("Writing to endpoint {:?}", write_cmd.endpoint());
        let chr = char_map.get(&write_cmd.endpoint()).unwrap().clone();
        spawn_local(async move {
          let data: Uint8Array = Uint8Array::from(write_cmd.data().as_slice());
          let result = match chr.write_value_with_u8_array(&data) {
            Ok(promise) => JsFuture::from(promise)
              .await
              .map(|_| ())
              .map_err(|e| {
                ButtplugDeviceError::DeviceCommunicationError(format!(
                  "WebBluetooth write failed: {:?}",
                  e
                ))
              }),
            Err(e) => Err(ButtplugDeviceError::DeviceCommunicationError(format!(
              "WebBluetooth write setup failed: {:?}",
              e
            ))),
          };
          let _ = reply.send(result);
        });
      }
      WebBluetoothDeviceCommand::Read(read_cmd, reply) => {
        debug!("Reading from endpoint {:?}", read_cmd.endpoint());
        let chr = char_map.get(&read_cmd.endpoint()).unwrap().clone();
        spawn_local(async move {
          let result = JsFuture::from(chr.read_value())
            .await
            .map_err(|e| {
              ButtplugDeviceError::DeviceCommunicationError(format!(
                "WebBluetooth read failed: {:?}",
                e
              ))
            })
            .map(|val| {
              let data_view = DataView::try_from(val).unwrap();
              let mut body = vec![0u8; data_view.byte_length() as usize];
              Uint8Array::new(&data_view).copy_to(&mut body[..]);
              HardwareReading::new(read_cmd.endpoint(), &body)
            });
          let _ = reply.send(result);
        });
      }
      WebBluetoothDeviceCommand::Subscribe(subscribe_cmd, reply) => {
        debug!("Subscribing to endpoint {:?}", subscribe_cmd.endpoint());
        let chr = char_map.get(&subscribe_cmd.endpoint()).unwrap().clone();
        let ep = subscribe_cmd.endpoint();
        let event_sender = external_event_tx.clone();
        let id = device.id();
        let onchange = Closure::wrap(Box::new(move |e: MessageEvent| {
          let event_chr: BluetoothRemoteGattCharacteristic =
            BluetoothRemoteGattCharacteristic::from(JsValue::from(e.target().unwrap()));
          let value = Uint8Array::new_with_byte_offset(
            &JsValue::from(event_chr.value().unwrap().buffer()),
            0,
          );
          let value_vec = value.to_vec();
          debug!("Subscription notification from {:?}: {:?}", ep, value_vec);
          let _ = event_sender.send(HardwareEvent::Notification(id.clone(), ep, value_vec));
        }) as Box<dyn FnMut(MessageEvent)>);
        chr.set_oncharacteristicvaluechanged(Some(onchange.as_ref().unchecked_ref()));
        onchange.forget();
        spawn_local(async move {
          let result = JsFuture::from(chr.start_notifications())
            .await
            .map(|_| ())
            .map_err(|e| {
              ButtplugDeviceError::DeviceCommunicationError(format!(
                "WebBluetooth subscribe failed: {:?}",
                e
              ))
            });
          debug!("Endpoint subscribed");
          let _ = reply.send(result);
        });
      }
      WebBluetoothDeviceCommand::Unsubscribe(_unsubscribe_cmd, reply) => {
        error!("WebBluetooth unsubscribe not yet implemented");
        let _ = reply.send(Ok(()));
      }
    }
  }

  debug!("run_webbluetooth_loop exited");
}

#[derive(Debug)]
pub struct WebBluetoothHardware {
  event_sender: broadcast::Sender<HardwareEvent>,
  device_command_sender: mpsc::Sender<WebBluetoothDeviceCommand>,
}

impl WebBluetoothHardware {
  fn new(
    event_sender: broadcast::Sender<HardwareEvent>,
    device_command_sender: mpsc::Sender<WebBluetoothDeviceCommand>,
  ) -> Self {
    Self {
      event_sender,
      device_command_sender,
    }
  }
}

impl HardwareInternal for WebBluetoothHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    Box::pin(future::ready(Ok(())))
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    let sender = self.device_command_sender.clone();
    let msg = msg.clone();
    async move {
      let (tx, rx) = oneshot::channel();
      sender
        .send(WebBluetoothDeviceCommand::Read(msg, tx))
        .await
        .map_err(|_| {
          ButtplugDeviceError::DeviceCommunicationError("Command channel dropped".to_owned())
        })?;
      rx.await.map_err(|_| {
        ButtplugDeviceError::DeviceCommunicationError("Reply channel dropped".to_owned())
      })?
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.device_command_sender.clone();
    let msg = msg.clone();
    async move {
      let (tx, rx) = oneshot::channel();
      sender
        .send(WebBluetoothDeviceCommand::Write(msg, tx))
        .await
        .map_err(|_| {
          ButtplugDeviceError::DeviceCommunicationError("Command channel dropped".to_owned())
        })?;
      rx.await.map_err(|_| {
        ButtplugDeviceError::DeviceCommunicationError("Reply channel dropped".to_owned())
      })?
    }
    .boxed()
  }

  fn subscribe(
    &self,
    msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.device_command_sender.clone();
    let msg = msg.clone();
    async move {
      let (tx, rx) = oneshot::channel();
      sender
        .send(WebBluetoothDeviceCommand::Subscribe(msg, tx))
        .await
        .map_err(|_| {
          ButtplugDeviceError::DeviceCommunicationError("Command channel dropped".to_owned())
        })?;
      rx.await.map_err(|_| {
        ButtplugDeviceError::DeviceCommunicationError("Reply channel dropped".to_owned())
      })?
    }
    .boxed()
  }

  fn unsubscribe(
    &self,
    msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.device_command_sender.clone();
    let msg = msg.clone();
    async move {
      let (tx, rx) = oneshot::channel();
      sender
        .send(WebBluetoothDeviceCommand::Unsubscribe(msg, tx))
        .await
        .map_err(|_| {
          ButtplugDeviceError::DeviceCommunicationError("Command channel dropped".to_owned())
        })?;
      rx.await.map_err(|_| {
        ButtplugDeviceError::DeviceCommunicationError("Reply channel dropped".to_owned())
      })?
    }
    .boxed()
  }
}
