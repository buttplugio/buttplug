use std::{collections::BTreeMap, io, net::SocketAddr, sync::Arc};

use axum::{extract::{Path, State}, http::StatusCode, routing::{get, post}, Json, Router};
use buttplug_client::{ButtplugClient, ButtplugClientDevice};
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_core::message::DeviceFeature;
use buttplug_server::ButtplugServer;
use tokio::net::TcpListener;
use serde::{Serialize, Deserialize};

#[derive(Serialize)]
struct IntifaceRestDevice {
  index: u32,
  name: String,
  display_name: Option<String>,
  features: BTreeMap<u32, DeviceFeature>
}

impl IntifaceRestDevice {
  fn from_client_device(device: &ButtplugClientDevice) -> Self {
    Self {
      index: device.index(),
      name: device.name().clone(),
      display_name: device.display_name().clone(),
      features: device.device_features()
        .iter()
        .map(|(i, d)| (*i, d.feature().clone()))
        .collect()
    }
  }
}

pub struct IntifaceRestServer {}

async fn start_scanning(State(client): State<Arc<ButtplugClient>>) {
  client.start_scanning().await;
}

async fn stop_scanning(State(client): State<Arc<ButtplugClient>>) {
  client.stop_scanning().await;
}

async fn set_device_vibrate_speed(State(client): State<Arc<ButtplugClient>>, Path((index, level)): Path<(u32, f64)>) -> Result<(), StatusCode> {
  client
  .devices()
  .get(&index)
  .ok_or(StatusCode::NOT_FOUND)?
  .vibrate(level)
  .await
  .map_err(|e| StatusCode::UNPROCESSABLE_ENTITY)
}

async fn get_devices(State(client): State<Arc<ButtplugClient>>) -> Json<BTreeMap<u32, IntifaceRestDevice>> {
  client
  .devices()
  .iter()
  .map(|(i, x)| (*i, IntifaceRestDevice::from_client_device(x)))
  .collect::<BTreeMap<u32, IntifaceRestDevice>>()
  .into()
}

impl IntifaceRestServer {
  pub async fn run(server: ButtplugServer) -> Result<(), io::Error> {
    let connector = ButtplugInProcessClientConnectorBuilder::default()
      .server(server)
      .finish();
    let client = ButtplugClient::new("Intiface REST API");
    client.connect(connector).await.unwrap();
    info!("Setting up app!");
    // pass incoming GET requests on "/hello-world" to "hello_world" handler.
    let app = Router::new()
      .route("/start-scanning", get(start_scanning))
      .route("/stop-scanning", get(stop_scanning))
      .route("/devices", get(get_devices))
      .route("/devices/{index}/vibrate/{level}", post(set_device_vibrate_speed))
      //.route("/devices/{*index}/vibrate", post(set_feature_vibrate_speed))
      .with_state(Arc::new(client));

    // write address like this to not make typos
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
  }
}
