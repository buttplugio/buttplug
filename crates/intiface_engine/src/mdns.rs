// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use rand::distr::{Alphanumeric, SampleString};
use std::{collections::HashSet, future::Future, io, net::IpAddr};

pub const INTIFACE_MDNS_SERVICE: &str = "_intiface_engine._tcp";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntifaceMdnsServiceMetadata {
  pub service_type: String,
  pub instance_name: String,
  pub txt_records: Vec<String>,
}

impl IntifaceMdnsServiceMetadata {
  pub fn new(suffix: Option<&str>) -> Self {
    let random_suffix = Alphanumeric.sample_string(&mut rand::rng(), 6);
    Self::from_suffix_and_random_suffix(suffix, &random_suffix)
  }

  pub fn from_suffix_and_random_suffix(suffix: Option<&str>, random_suffix: &str) -> Self {
    let instance_name = match suffix.filter(|value| !value.trim().is_empty()) {
      Some(suffix) => format!("Intiface {} {}", random_suffix, suffix),
      None => format!("Intiface {}", random_suffix),
    };

    Self {
      service_type: INTIFACE_MDNS_SERVICE.to_owned(),
      instance_name,
      txt_records: vec!["path=/".to_owned()],
    }
  }
}

pub struct IntifaceMdns {
  _responder: libmdns::Responder,
  _svc: libmdns::Service,
}

impl IntifaceMdns {
  pub fn new(metadata: &IntifaceMdnsServiceMetadata, port: u16) -> Option<Self> {
    info!(
      "Bringing up mDNS Advertisment using instance name {} on port {}",
      metadata.instance_name, port
    );

    let responder_result = match mdns_advertisement_ips() {
      Some(allowed_ips) if allowed_ips.is_empty() => {
        warn!("Unable to bring up mDNS advertisement: no usable local network addresses found");
        return None;
      }
      Some(allowed_ips) => {
        info!("Restricting mDNS advertisement to IPs: {:?}", allowed_ips);
        libmdns::Responder::with_default_handle_and_ip_list(allowed_ips)
      }
      None => libmdns::Responder::with_default_handle(),
    };

    Self::from_responder_result(metadata, port, responder_result)
  }

  fn from_responder_result<T>(
    metadata: &IntifaceMdnsServiceMetadata,
    port: u16,
    responder_result: io::Result<(libmdns::Responder, T)>,
  ) -> Option<Self>
  where
    T: Future<Output = ()> + Send + 'static,
  {
    let (_responder, task) = match responder_result {
      Ok(result) => result,
      Err(err) => {
        warn!("Unable to bring up mDNS advertisement: {}", err);
        return None;
      }
    };
    let txt_records: Vec<&str> = metadata
      .txt_records
      .iter()
      .map(|record| record.as_str())
      .collect();
    let _svc = _responder.register(
      metadata.service_type.as_str(),
      metadata.instance_name.as_str(),
      port,
      &txt_records,
    );
    tokio::spawn(async move {
      info!("Entering up mDNS task");
      task.await;
      info!("Exiting mDNS task");
    });
    Some(Self { _responder, _svc })
  }
}

fn mdns_advertisement_ips() -> Option<Vec<IpAddr>> {
  let interfaces = match local_ip_address::list_afinet_netifas() {
    Ok(interfaces) => interfaces,
    Err(err) => {
      warn!("Unable to enumerate network interfaces for mDNS advertisement filtering: {err}");
      return None;
    }
  };

  Some(filter_mdns_advertisement_ips(interfaces))
}

fn filter_mdns_advertisement_ips(interfaces: Vec<(String, IpAddr)>) -> Vec<IpAddr> {
  let mut seen = HashSet::new();
  interfaces
    .into_iter()
    .filter_map(|(name, ip)| {
      if should_advertise_mdns_ip(&name, ip) && seen.insert(ip) {
        Some(ip)
      } else {
        None
      }
    })
    .collect()
}

fn should_advertise_mdns_ip(interface_name: &str, ip: IpAddr) -> bool {
  !is_ignored_mdns_interface(interface_name) && is_publishable_mdns_ip(ip)
}

fn is_ignored_mdns_interface(interface_name: &str) -> bool {
  let name = interface_name.to_ascii_lowercase();
  name.starts_with("lo")
    || name.starts_with("rmnet")
    || name.starts_with("ccmni")
    || name.starts_with("pdp_ip")
    || name.starts_with("wwan")
    || name.starts_with("clat")
    || name.starts_with("v4-")
}

fn is_publishable_mdns_ip(ip: IpAddr) -> bool {
  match ip {
    IpAddr::V4(addr) => {
      let octets = addr.octets();
      !addr.is_loopback()
        && !addr.is_unspecified()
        && !addr.is_multicast()
        && !addr.is_broadcast()
        && octets[..3] != [192, 0, 0]
    }
    IpAddr::V6(addr) => !addr.is_loopback() && !addr.is_unspecified() && !addr.is_multicast(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn mdns_metadata_uses_expected_service_contract() {
    let metadata =
      IntifaceMdnsServiceMetadata::from_suffix_and_random_suffix(Some("mobile"), "ABC123");

    assert_eq!(
      metadata,
      IntifaceMdnsServiceMetadata {
        service_type: "_intiface_engine._tcp".to_owned(),
        instance_name: "Intiface ABC123 mobile".to_owned(),
        txt_records: vec!["path=/".to_owned()],
      }
    );
  }

  #[test]
  fn mdns_startup_error_disables_advertisement() {
    let metadata = IntifaceMdnsServiceMetadata::from_suffix_and_random_suffix(None, "Test");
    let result = IntifaceMdns::from_responder_result::<std::future::Pending<()>>(
      &metadata,
      12345,
      Err(io::Error::new(
        io::ErrorKind::AddrInUse,
        "Address already in use",
      )),
    );

    assert!(result.is_none());
  }

  #[test]
  fn mdns_advertisement_ip_filter_drops_unreachable_mobile_addresses() {
    let result = filter_mdns_advertisement_ips(vec![
      ("lo".to_owned(), "127.0.0.1".parse().unwrap()),
      ("clat4".to_owned(), "192.0.0.4".parse().unwrap()),
      ("rmnet_data0".to_owned(), "10.0.0.4".parse().unwrap()),
      ("wlan0".to_owned(), "192.168.1.149".parse().unwrap()),
    ]);

    assert_eq!(result, vec!["192.168.1.149".parse::<IpAddr>().unwrap()]);
  }

  #[test]
  fn mdns_advertisement_ip_filter_dedupes_addresses() {
    let result = filter_mdns_advertisement_ips(vec![
      ("wlan0".to_owned(), "192.168.1.149".parse().unwrap()),
      ("wlan0:1".to_owned(), "192.168.1.149".parse().unwrap()),
    ]);

    assert_eq!(result, vec!["192.168.1.149".parse::<IpAddr>().unwrap()]);
  }
}
