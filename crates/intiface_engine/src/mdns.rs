// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use rand::distr::{Alphanumeric, SampleString};
use std::{future::Future, io};

pub struct IntifaceMdns {
  _responder: libmdns::Responder,
  _svc: libmdns::Service,
}

impl IntifaceMdns {
  pub fn new() -> Option<Self> {
    let random_suffix = Alphanumeric.sample_string(&mut rand::rng(), 6);
    let instance_name = format!("Intiface {}", random_suffix);
    info!(
      "Bringing up mDNS Advertisment using instance name {}",
      instance_name
    );

    Self::from_responder_result(&instance_name, libmdns::Responder::with_default_handle())
  }

  fn from_responder_result<T>(
    instance_name: &str,
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
    let _svc = _responder.register("_intiface_engine._tcp", &instance_name, 12345, &["path=/"]);
    tokio::spawn(async move {
      info!("Entering up mDNS task");
      task.await;
      info!("Exiting mDNS task");
    });
    Some(Self { _responder, _svc })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn mdns_startup_error_disables_advertisement() {
    let result = IntifaceMdns::from_responder_result::<std::future::Pending<()>>(
      "Intiface Test",
      Err(io::Error::new(
        io::ErrorKind::AddrInUse,
        "Address already in use",
      )),
    );

    assert!(result.is_none());
  }
}
