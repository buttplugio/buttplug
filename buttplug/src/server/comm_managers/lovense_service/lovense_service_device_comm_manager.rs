use super::lovense_service_device_impl::LovenseServiceDeviceImplCreator;
use crate::{
  core::ButtplugResultFuture,
  server::comm_managers::{
    DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
  }
};
use futures::future;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

#[derive(Deserialize, Debug)]
struct LovenseServiceToyInfo {
  id: String,
  name: String,
  #[serde(rename = "nickName")]
  nickname: String,
  #[serde(rename = "status")]
  connected: String,
  version: String,
  battery: u8,
}

#[derive(Deserialize, Debug)]
struct LovenseServiceHostInfo {
  domain: String,
  #[serde(rename = "httpPort")]
  http_port: u16,
  #[serde(rename = "wsPort")]
  ws_port: u16,
  #[serde(rename = "httpsPort")]
  https_port: u16,
  #[serde(rename = "wssPort")]
  wss_port: u16,
  toys: HashMap<String, LovenseServiceToyInfo>,
}

type LovenseServiceInfo = HashMap<String, LovenseServiceHostInfo>;

pub struct LovenseServiceDeviceCommManager {
  sender: mpsc::Sender<DeviceCommunicationEvent>,
  scanning_notifier: Arc<Notify>,
}

impl DeviceCommunicationManagerCreator for LovenseServiceDeviceCommManager {
  fn new(sender: mpsc::Sender<DeviceCommunicationEvent>) -> Self {
    Self {
      sender,
      scanning_notifier: Arc::new(Notify::new()),
    }
  }
}

impl DeviceCommunicationManager for LovenseServiceDeviceCommManager {
  fn name(&self) -> &'static str {
    "LovenseServiceDeviceCommManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    let sender = self.sender.clone();
    Box::pin(async move {
      match reqwest::get("https://api.lovense.com/api/lan/getToys").await {
        Ok(res) => {
          let text = res.text().await.unwrap();
          let info: LovenseServiceInfo = serde_json::from_str(&text).unwrap();
          info!("{:?}", info);
          for (_, host_info) in info.iter() {
            for (_, toy) in host_info.toys.iter() {
              let device_creator = Box::new(LovenseServiceDeviceImplCreator::new(&host_info.domain, host_info.http_port, &toy.name, &toy.id));
              if sender
                .send(DeviceCommunicationEvent::DeviceFound { name: toy.name.clone(), address: toy.id.clone(), creator: device_creator })
                .await
                .is_err()
              {
                error!("Error sending device found message from HTTP Endpoint Manager.");
              }
            }
          }
        }
        Err(err) => error!("Got http error: {}", err),
      };
      Ok(())
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }
}
