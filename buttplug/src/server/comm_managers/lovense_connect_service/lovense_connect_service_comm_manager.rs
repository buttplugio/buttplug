use super::lovense_connect_service_device_impl::LovenseServiceDeviceImplCreator;
use crate::{
  core::ButtplugResultFuture,
  server::comm_managers::{
    DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerBuilder,
  },
  util::async_manager
};
use dashmap::DashMap;
use futures::future;
use futures_timer::Delay;
use serde::{Deserialize};
use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tracing_futures::Instrument;
use tokio::sync::{mpsc, Mutex, RwLock};
use serde_aux::prelude::*;

const LOVENSE_LOCAL_SERVICE_CHECK_INTERVAL: u64 = 1;
const LOVENSE_REMOTE_SERVICE_CHECK_INTERVAL: u64 = 1;

#[derive(Deserialize, Debug, Clone)]
pub(super) struct LovenseServiceToyInfo {
  pub id: String,
  pub name: String,
  #[serde(rename = "nickName")]
  pub nickname: String,
  #[serde(rename = "status", deserialize_with = "deserialize_bool_from_anything")]
  pub connected: bool,
  pub version: String,
  #[serde(deserialize_with = "deserialize_number_from_string")]
  pub battery: i8,
}

#[derive(Deserialize, Debug)]
struct LovenseServiceHostInfo {
  pub domain: String,
  #[serde(rename = "httpPort")]
  pub http_port: u16,
  #[serde(rename = "wsPort")]
  pub ws_port: u16,
  #[serde(rename = "httpsPort")]
  pub https_port: u16,
  #[serde(rename = "wssPort")]
  pub wss_port: u16,
  pub toys: HashMap<String, LovenseServiceToyInfo>,
}

#[derive(Deserialize, Debug)]
struct LovenseServiceLocalInfo {
  #[serde(rename = "type", deserialize_with = "deserialize_string_from_number")]
  pub reply_type: String,
  #[serde(deserialize_with = "deserialize_number_from_string")]
  pub code: u32,
  pub data: HashMap<String, LovenseServiceToyInfo>,
}

type LovenseServiceInfo = HashMap<String, LovenseServiceHostInfo>;

async fn lovense_local_service_check(
  event_sender: mpsc::Sender<DeviceCommunicationEvent>,
  has_known_hosts: Arc<AtomicBool>,
  is_scanning: Arc<AtomicBool>,
  known_hosts: Arc<Mutex<Vec<String>>>,
) {
  let connected_device_info: Arc<DashMap<String, Arc<RwLock<LovenseServiceToyInfo>>>> = Arc::new(DashMap::new());
  loop {
    let hosts = known_hosts.lock().await.clone();
    if hosts.len() == 0 {
      has_known_hosts.store(false, Ordering::SeqCst);
      break;
    }
    for host in hosts {
      match reqwest::get(format!("{}/GetToys", host)).await {
        Ok(res) => {
          let text = res.text().await.unwrap();
          let info: LovenseServiceLocalInfo = serde_json::from_str(&text).unwrap();

          // First off, remove all devices that are no longer in the list
          // (devices turned off or removed from the Lovense Connect app)

          for disconnected_device in connected_device_info.iter().filter(|p| !info.data.contains_key(p.key())) {
            disconnected_device.value().write().await.connected = false;
          }
          connected_device_info.retain(|k, _| info.data.contains_key(k));

          for (_, toy) in info.data.iter() {
            if connected_device_info.contains_key(&toy.id) {
              // For some reason, this requires its own scoping block, otherwise
              // the write lock will hold forever, which blocks the server? I'm
              // guessing this has to do with loop hoisting but it still seems
              // odd.
              {
                let info_ref = connected_device_info.get(&toy.id).unwrap();
                let mut info_lock = info_ref.write().await;
                *info_lock = toy.clone();
              }
              // If the toy is no longer connected, remove it from our tracking.
              if !toy.connected {
                info!("Removing toy from main info map");
                connected_device_info.retain(|k, _| *k != toy.id);
              }
              continue;
            }
            if !is_scanning.load(Ordering::SeqCst) {
              continue;
            }
            if !toy.connected {
              continue;
            }
            connected_device_info.insert(toy.id.clone(), Arc::new(RwLock::new((*toy).clone())));
            let device_creator = Box::new(LovenseServiceDeviceImplCreator::new(
              &host,
              connected_device_info.get(&toy.id).unwrap().clone()
            ));
            if event_sender
              .send(DeviceCommunicationEvent::DeviceFound {
                name: toy.name.clone(),
                address: toy.id.clone(),
                creator: device_creator,
              })
              .await
              .is_err()
            {
              error!("Error sending device found message from HTTP Endpoint Manager.");
            }
          }
          
          //connected_devices = new_connected_devices;
        }
        Err(err) => {
          error!(
            "Got http error from lovense service, assuming Lovense connect app shutdown: {}",
            err
          );
          (*known_hosts.lock().await).retain(|x| *x != host);
        }
      }
    }
    Delay::new(Duration::from_secs(LOVENSE_LOCAL_SERVICE_CHECK_INTERVAL)).await;
  }
}


#[derive(Default)]
pub struct LovenseConnectServiceCommunicationManagerBuilder {
  sender: Option<tokio::sync::mpsc::Sender<DeviceCommunicationEvent>>
}

impl DeviceCommunicationManagerBuilder for LovenseConnectServiceCommunicationManagerBuilder {
  fn set_event_sender(&mut self, sender: mpsc::Sender<DeviceCommunicationEvent>) {
    self.sender = Some(sender)
  }

  fn finish(mut self) -> Box<dyn DeviceCommunicationManager> {
    Box::new(LovenseConnectServiceCommunicationManager::new(self.sender.take().unwrap()))
  }
}

pub struct LovenseConnectServiceCommunicationManager {
  sender: mpsc::Sender<DeviceCommunicationEvent>,
  known_hosts: Arc<Mutex<Vec<String>>>,
  is_scanning: Arc<AtomicBool>,
  has_known_hosts: Arc<AtomicBool>,
}

impl LovenseConnectServiceCommunicationManager {
  fn new(sender: mpsc::Sender<DeviceCommunicationEvent>) -> Self {
    Self {
      sender,
      known_hosts: Arc::new(Mutex::new(vec![])),
      is_scanning: Arc::new(AtomicBool::new(false)),
      has_known_hosts: Arc::new(AtomicBool::new(false))
    }
  }
}

impl DeviceCommunicationManager for LovenseConnectServiceCommunicationManager {
  fn name(&self) -> &'static str {
    "LovenseServiceDeviceCommManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    self.is_scanning.store(true, Ordering::SeqCst);
    let sender = self.sender.clone();
    let is_scanning = self.is_scanning.clone();
    let known_hosts = self.known_hosts.clone();
    let has_known_hosts = self.has_known_hosts.clone();
    async_manager::spawn(async move {
      debug!("Starting scanning");
      while is_scanning.load(Ordering::SeqCst) {
        match reqwest::get("https://api.lovense.com/api/lan/getToys").await {
          Ok(res) => {
            let text = res.text().await.unwrap();
            let info: LovenseServiceInfo = serde_json::from_str(&text).unwrap();
            let mut current_known_hosts = known_hosts.lock().await;
            // We set the protocol type here so it'll just filter down, in case we want to move to secure.
            let new_known_hosts: Vec<String> = info.iter().map(|x| format!("http://{}:{}", x.0, x.1.http_port)).collect();
            // check for both different numbers of elements as well as elements not being the same
            if current_known_hosts.len() != new_known_hosts.len() || !current_known_hosts.iter().all(|item| new_known_hosts.contains(&item)) {
              *current_known_hosts = new_known_hosts.iter().map(|x| (*x).clone()).collect();
            }
            if current_known_hosts.len() > 0 && !has_known_hosts.load(Ordering::SeqCst) {
              has_known_hosts.store(true, Ordering::SeqCst);
              let service_fut = lovense_local_service_check(sender.clone(), has_known_hosts.clone(), is_scanning.clone(), known_hosts.clone());
              async_manager::spawn(async move {
                service_fut.await;
              }).unwrap();
            }
          }
          Err(err) => error!("Got http error: {}", err),
        };
        Delay::new(Duration::from_secs(LOVENSE_REMOTE_SERVICE_CHECK_INTERVAL)).await;
      }
      debug!("Stopping scanning");
    }.instrument(info_span!("Lovense Connect Service Scanner"))).unwrap();
    Box::pin(async move { Ok(()) })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    self.is_scanning.store(false, Ordering::SeqCst);
    Box::pin(future::ready(Ok(())))
  }
}

impl Drop for LovenseConnectServiceCommunicationManager {
  fn drop(&mut self) {
    self.is_scanning.store(false, Ordering::SeqCst);
    self.has_known_hosts.store(false, Ordering::SeqCst);
  }
}