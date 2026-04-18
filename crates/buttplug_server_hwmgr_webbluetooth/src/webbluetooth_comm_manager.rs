// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::webbluetooth_hardware::WebBluetoothHardwareConnector;
use buttplug_core::ButtplugResultFuture;
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager, HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
};
use buttplug_server_device_config::{DeviceConfigurationManager, ProtocolCommunicationSpecifier};
use futures::future;
use js_sys::JsString;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{error, info};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::BluetoothDevice;

pub struct WebBluetoothCommunicationManagerBuilder {
  dcm: Arc<DeviceConfigurationManager>,
}

impl WebBluetoothCommunicationManagerBuilder {
  pub fn new(dcm: Arc<DeviceConfigurationManager>) -> Self {
    Self { dcm }
  }
}

impl HardwareCommunicationManagerBuilder for WebBluetoothCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(WebBluetoothCommunicationManager {
      sender,
      dcm: self.dcm.clone(),
    })
  }
}

pub struct WebBluetoothCommunicationManager {
  sender: Sender<HardwareCommunicationManagerEvent>,
  dcm: Arc<DeviceConfigurationManager>,
}

#[wasm_bindgen]
extern "C" {
  #[wasm_bindgen(js_namespace = console)]
  fn log(s: &str);
}

impl HardwareCommunicationManager for WebBluetoothCommunicationManager {
  fn name(&self) -> &'static str {
    "WebBluetoothCommunicationManager"
  }

  fn can_scan(&self) -> bool {
    true
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    info!("WebBluetooth manager scanning");
    let sender_clone = self.sender.clone();
    let dcm = self.dcm.clone();
    spawn_local(async move {
      let nav = web_sys::window().unwrap().navigator();
      if nav.bluetooth().is_none() {
        error!("WebBluetooth is not supported on this browser");
        return;
      }
      info!("WebBluetooth supported by browser, continuing with scan.");

      let options = web_sys::RequestDeviceOptions::new();
      let mut filters: Vec<web_sys::BluetoothLeScanFilterInit> = Vec::new();
      let mut optional_services: Vec<JsString> = Vec::new();

      for (_protocol_name, specifiers) in dcm.base_communication_specifiers().iter() {
        for specifier in specifiers {
          if let ProtocolCommunicationSpecifier::BluetoothLE(btle) = specifier {
            for name in btle.names() {
              let filter = web_sys::BluetoothLeScanFilterInit::new();
              if name.contains('*') {
                let mut name_clone = name.clone();
                name_clone.pop();
                filter.set_name_prefix(&name_clone);
              } else {
                filter.set_name(name);
              }
              filters.push(filter);
            }
            for (service, _) in btle.services() {
              optional_services.push(JsString::from(service.to_string().as_str()));
            }
          }
        }
      }

      options.set_filters(&filters);
      options.set_optional_services(&optional_services);

      let nav = web_sys::window().unwrap().navigator();
      match JsFuture::from(nav.bluetooth().unwrap().request_device(&options)).await {
        Ok(device) => {
          let bt_device = BluetoothDevice::from(device);
          if bt_device.name().is_none() {
            return;
          }
          let name = bt_device.name().unwrap();
          let address = bt_device.id();
          let device_creator = Box::new(WebBluetoothHardwareConnector::new(bt_device));
          if sender_clone
            .send(HardwareCommunicationManagerEvent::DeviceFound {
              name,
              address,
              creator: device_creator,
            })
            .await
            .is_err()
          {
            error!("Device manager receiver dropped, cannot send device found message.");
          } else {
            info!("WebBluetooth device found.");
          }
        }
        Err(e) => {
          error!("Error while trying to start bluetooth scan: {:?}", e);
        }
      }

      let _ = sender_clone
        .send(HardwareCommunicationManagerEvent::ScanningFinished)
        .await;
    });
    Box::pin(future::ready(Ok(())))
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }
}
