// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::lovense_connect_service_hardware::LovenseServiceHardwareConnector;
use crate::{
  core::errors::ButtplugDeviceError,
  server::device::hardware::communication::{
    HardwareCommunicationManager, HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent, TimedRetryCommunicationManager,
    TimedRetryCommunicationManagerImpl,
  },
};
use async_trait::async_trait;
use dashmap::DashSet;
use reqwest::StatusCode;
use serde::{Deserialize, Deserializer};
use serde_aux::prelude::*;
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

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
  /* ~ Sutekh
   * Implemented a deserializer for the battery field.
   * The battery field needs to be able to handle when the JSON field for it is null.
   */
  #[serde(deserialize_with = "parse_battery")]
  pub battery: i8,
}

/* ~ Sutekh
 * Parse the LovenseServiceToyInfo battery field to handle incoming JSON null values from the Lovense Connect app.
 * This deserializer will check if we received an i8 or null.
 * If the value is null it will set the battery level to 0.
 */
fn parse_battery<'de, D>(d: D) -> Result<i8, D::Error>
where
  D: Deserializer<'de>,
{
  Deserialize::deserialize(d).map(|b: Option<_>| b.unwrap_or(0))
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
pub(super) struct LovenseServiceLocalInfo {
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
  #[serde(default)]
  pub data: HashMap<String, LovenseServiceToyInfo>,
}

type LovenseServiceInfo = HashMap<String, LovenseServiceHostInfo>;

#[derive(Default, Clone)]
pub struct LovenseConnectServiceCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for LovenseConnectServiceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(TimedRetryCommunicationManager::new(
      LovenseConnectServiceCommunicationManager::new(sender),
    ))
  }
}

pub struct LovenseConnectServiceCommunicationManager {
  sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  known_hosts: DashSet<String>,
}

pub(super) async fn get_local_info(host: &str) -> Option<LovenseServiceLocalInfo> {
  match reqwest::get(format!("{}/GetToys", host)).await {
    Ok(res) => {
      if res.status() != StatusCode::OK {
        error!(
          "Error contacting Lovense Connect Local API endpoint. Status returned: {}",
          res.status()
        );
        return None;
      }

      match res.text().await {
        Ok(text) => match serde_json::from_str(&text) {
          Ok(info) => Some(info),
          Err(e) => {
            warn!("Should always get json back from service, if we got a response: ${e}");
            None
          }
        },
        Err(e) => {
          warn!("If we got a 200 back, we should at least have text: ${e}");
          None
        }
      }
    }
    Err(err) => {
      error!(
        "Got http error from lovense service, assuming Lovense connect app shutdown: {}",
        err
      );
      // 99% of the time, we'll only have one host. So just do the convenient thing and break.
      // This'll get called again in 1s anyways.
      None
    }
  }
}

impl LovenseConnectServiceCommunicationManager {
  fn new(sender: mpsc::Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self {
      sender,
      known_hosts: DashSet::new(),
    }
  }

  async fn lovense_local_service_check(&self) {
    if self.known_hosts.is_empty() {
      return;
    }
    for host in self.known_hosts.iter() {
      match get_local_info(&host).await {
        Some(info) => {
          for (_, toy) in info.data.iter() {
            if !toy.connected {
              continue;
            }
            let device_creator = Box::new(LovenseServiceHardwareConnector::new(&host, toy));
            // This will emit all of the toys as new devices every time we find them. Just let the
            // Device Manager reject them as either connecting or already connected.
            if self
              .sender
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
        }
        None => {
          self.known_hosts.remove(&*host);
        }
      }
    }
  }
}

#[async_trait]
impl TimedRetryCommunicationManagerImpl for LovenseConnectServiceCommunicationManager {
  fn name(&self) -> &'static str {
    "LovenseServiceDeviceCommManager"
  }

  fn rescan_wait_duration(&self) -> Duration {
    Duration::from_secs(10)
  }

  async fn scan(&self) -> Result<(), ButtplugDeviceError> {
    // If we already know about a local host, check it. Otherwise, query remotely to look for local
    // hosts.
    if !self.known_hosts.is_empty() {
      self.lovense_local_service_check().await;
    } else {
      match reqwest::get("https://api.lovense.com/api/lan/getToys").await {
        Ok(res) => {
          if res.status() != StatusCode::OK {
            error!(
              "Error contacting Lovense Connect Remote API endpoint. Status returned: {}",
              res.status()
            );
            return Ok(());
          }
          let text = res
            .text()
            .await
            .expect("Should always get json back from service, if we got a response.");
          let info: LovenseServiceInfo = serde_json::from_str(&text)
            .expect("Should always get json back from service, if we got a response.");
          info.iter().for_each(|x| {
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
            self.known_hosts.insert(host);
          });
          // If we've found new hosts, go ahead and search them.
          if !self.known_hosts.is_empty() {
            self.lovense_local_service_check().await
          }
        }
        Err(err) => {
          error!("Got http error: {}", err);
        }
      }
    }
    Ok(())
  }

  // Assume we've already got network access. A bad assumption, but we'll need to figure out how to
  // make this work better later.
  fn can_scan(&self) -> bool {
    true
  }
}
