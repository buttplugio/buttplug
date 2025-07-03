use rand::distr::{Alphanumeric, SampleString};

pub struct IntifaceMdns {
  _responder: libmdns::Responder,
  _svc: libmdns::Service,
}

impl IntifaceMdns {
  pub fn new() -> Self {
    let random_suffix = Alphanumeric.sample_string(&mut rand::rng(), 6);
    let instance_name = format!("Intiface {}", random_suffix);
    info!(
      "Bringing up mDNS Advertisment using instance name {}",
      instance_name
    );

    let (_responder, task) = libmdns::Responder::with_default_handle().unwrap();
    let _svc = _responder.register(
      "_intiface_engine._tcp".to_owned(),
      instance_name,
      12345,
      &["path=/"],
    );
    tokio::spawn(async move {
      info!("Entering up mDNS task");
      task.await;
      info!("Exiting mDNS task");
    });
    Self { _responder, _svc }
  }
}
