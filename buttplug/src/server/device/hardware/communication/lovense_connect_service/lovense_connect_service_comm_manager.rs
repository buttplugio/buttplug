// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::lovense_connect_service_hardware::LovenseServiceHardwareCreator;
use crate::{
  core::ButtplugResultFuture,
  server::device::hardware::communication::{
    HardwareCommunicationManagerEvent,
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
  },
  util::async_manager,
};
use dashmap::DashMap;
use futures::future;
use futures_timer::Delay;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_aux::prelude::*;
use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing_futures::Instrument;

const LOVENSE_LOCAL_SERVICE_CHECK_INTERVAL: u64 = 1;
const LOVENSE_REMOTE_SERVICE_CHECK_INTERVAL: u64 = 1;

#[derive(Deserialize, Debug, Clone)]
pub(super) struct LovenseServiceToyInfo {
  pub id: String,
  pub name: String,
  #[serde(rename = "nickName", skip)]
  pub _nickname: String,
  #[serde(rename = "status", deserialize_with = "deserialize_bool_from_anything")]
  pub connected: bool,
  #[serde(
    rename = "version",
    skip,
    deserialize_with = "deserialize_number_from_string"
  )]
  pub _version: i32,
  #[serde(deserialize_with = "deserialize_number_from_string")]
  pub battery: i8,
}

#[derive(Deserialize, Debug)]
struct LovenseServiceHostInfo {
  #[serde(rename = "domain")]
  pub _domain: String,
  #[serde(
    rename = "httpPort",
    deserialize_with = "deserialize_number_from_string"
  )]
  pub http_port: u16,
  #[serde(
    rename = "wsPort",
    skip,
    deserialize_with = "deserialize_number_from_string"
  )]
  pub _ws_port: u16,
  #[serde(
    rename = "httpsPort",
    skip,
    deserialize_with = "deserialize_number_from_string"
  )]
  pub _https_port: u16,
  #[serde(
    rename = "wssPort",
    skip,
    deserialize_with = "deserialize_number_from_string"
  )]
  pub _wss_port: u16,
  #[serde(rename = "toys", skip)]
  pub _toys: HashMap<String, LovenseServiceToyInfo>,
}

#[derive(Deserialize, Debug)]
struct LovenseServiceLocalInfo {
  #[serde(
    rename = "type",
    skip,
    deserialize_with = "deserialize_string_from_number"
  )]
  pub _reply_type: String,
  #[serde(
    rename = "code",
    skip,
    deserialize_with = "deserialize_number_from_string"
  )]
  pub _code: u32,
  pub data: HashMap<String, LovenseServiceToyInfo>,
}

type LovenseServiceInfo = HashMap<String, LovenseServiceHostInfo>;

async fn lovense_local_service_check(
  event_sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  is_scanning: Arc<AtomicBool>,
  known_hosts: Arc<Mutex<Vec<String>>>,
) {
  let connected_device_info: Arc<DashMap<String, Arc<RwLock<LovenseServiceToyInfo>>>> =
    Arc::new(DashMap::new());
  loop {
    let hosts = known_hosts.lock().await.clone();
    if hosts.is_empty() {
      break;
    }
    for host in hosts {
      match reqwest::get(format!("{}/GetToys", host)).await {
        Ok(res) => {
          if res.status() != StatusCode::OK {
            error!(
              "Error contacting Lovense Connect Local API endpoint. Status returned: {}",
              res.status()
            );
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
          }

          let text = res
            .text()
            .await
            .expect("If we got a 200 back, we should at least have text.");
          let info: LovenseServiceLocalInfo = serde_json::from_str(&text)
            .expect("Should always get json back from service, if we got a response.");

          // First off, remove all devices that are no longer in the list
          // (devices turned off or removed from the Lovense Connect app)

          for disconnected_device in connected_device_info
            .iter()
            .filter(|p| !info.data.contains_key(p.key()))
          {
            disconnected_device.value().write().await.connected = false;
          }
          connected_device_info.retain(|k, _| info.data.contains_key(k));

          for (_, toy) in info.data.iter() {
            if let Some(info_ref) = connected_device_info.get(&toy.id) {
              // For some reason, this requires its own scoping block, otherwise
              // the write lock will hold forever, which blocks the server? I'm
              // guessing this has to do with loop hoisting but it still seems
              // odd.
              {
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
            let device_creator = Box::new(LovenseServiceHardwareCreator::new(
              &host,
              connected_device_info
                .get(&toy.id)
                .expect("Just inserted this.")
                .clone(),
            ));
            if event_sender
              .send(HardwareCommunicationManagerEvent::DeviceFound {
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

#[derive(Default, Clone)]
pub struct LovenseConnectServiceCommunicationManagerBuilder {
}

impl HardwareCommunicationManagerBuilder for LovenseConnectServiceCommunicationManagerBuilder {
  fn finish(&self, sender: Sender<HardwareCommunicationManagerEvent>) -> Box<dyn HardwareCommunicationManager> {
    Box::new(LovenseConnectServiceCommunicationManager::new(
        sender
    ))
  }
}

pub struct LovenseConnectServiceCommunicationManager {
  sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  known_hosts: Arc<Mutex<Vec<String>>>,
  is_scanning: Arc<AtomicBool>,
}

impl LovenseConnectServiceCommunicationManager {
  fn new(sender: mpsc::Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self {
      sender,
      known_hosts: Arc::new(Mutex::new(vec![])),
      is_scanning: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl HardwareCommunicationManager for LovenseConnectServiceCommunicationManager {
  fn name(&self) -> &'static str {
    "LovenseServiceDeviceCommManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    self.is_scanning.store(true, Ordering::SeqCst);
    let sender = self.sender.clone();
    let is_scanning = self.is_scanning.clone();
    let known_hosts = self.known_hosts.clone();
    async_manager::spawn(
      async move {
        debug!("Starting scanning");
        let mut has_warned = false;
        while is_scanning.load(Ordering::SeqCst) {
          match reqwest::get("https://api.lovense.com/api/lan/getToys").await {
            Ok(res) => {
              if res.status() != StatusCode::OK {
                error!("Error contacting Lovense Connect Remote API endpoint. Status returned: {}", res.status());
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
              }
              let text = res.text().await.expect("Should always get json back from service, if we got a response.");
              let info: LovenseServiceInfo = serde_json::from_str(&text).expect("Should always get json back from service, if we got a response.");
              let mut current_known_hosts = known_hosts.lock().await;
              let new_known_hosts: Vec<String> = info
                .iter()
                .map(|x| {
                  // Lovense Connect uses [ip].lovense.club, which is a loopback DNS resolver that
                  // should just point to [ip]. This is used for handling secure certificate
                  // resolution when trying to use lovense connect over secure contexts. However,
                  // this sometimes fails on DNS resolution. Since we aren't using secure contexts
                  // at the moment, we can just cut out the IP from the domain and use that
                  // directly, which has fixed issues for some users.
                  let host_parts: Vec<&str> = x.0.split('.').collect();
                  let new_http_host = host_parts[0].replace('-', ".");
                  // We set the protocol type here so it'll just filter down, in case we want to move to secure.
                  let host = format!("http://{}:{}", new_http_host, x.1.http_port);
                  debug!("Lovense Connect converting IP to {}", host);
                  host
                })
                .collect();
              // check for both different numbers of elements as well as elements not being the same
              if current_known_hosts.len() != new_known_hosts.len()
                || !current_known_hosts
                  .iter()
                  .all(|item| new_known_hosts.contains(item))
              {
                *current_known_hosts = new_known_hosts.iter().map(|x| (*x).clone()).collect();
              }
              if current_known_hosts.is_empty() {
                if !has_warned {
                  warn!("Lovense Connect Service could not find any usable hosts. Will continue scanning until hosts are found or scanning is requested to stop.");
                  has_warned = true;
                }
              } else {
                let service_fut = lovense_local_service_check(
                  sender.clone(),
                  is_scanning.clone(),
                  known_hosts.clone(),
                );
                info!("Lovense Connect Server API query returned: {}", text);
                async_manager::spawn(async move {
                  service_fut.await;
                });
                break;
              }
            }
            Err(err) => error!("Got http error: {}", err),
          };
          Delay::new(Duration::from_secs(LOVENSE_REMOTE_SERVICE_CHECK_INTERVAL)).await;
        }
        debug!("Stopping scanning");
      }
      .instrument(info_span!("Lovense Connect Service Scanner")),
    );
    Box::pin(async move { Ok(()) })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    self.is_scanning.store(false, Ordering::SeqCst);
    Box::pin(future::ready(Ok(())))
  }

  // Assume we've already got network access. A bad assumption, but we'll need to figure out how to
  // make this work better later.
  fn can_scan(&self) -> bool {
    true
  }
}

impl Drop for LovenseConnectServiceCommunicationManager {
  fn drop(&mut self) {
    self.is_scanning.store(false, Ordering::SeqCst);
  }
}
