use std::{
  collections::BTreeMap, convert::Infallible, io, net::SocketAddr, sync::Arc,
};

use axum::{
  Json, Router,
  extract::{Path, State, rejection::JsonRejection},
  http::StatusCode,
  response::{
    IntoResponse, Response, Sse,
    sse::{Event, KeepAlive},
  },
  routing::{get, put},
};
use buttplug_client::{
  ButtplugClient, ButtplugClientDevice, ButtplugClientError,
  device::{ClientDeviceFeature, ClientDeviceOutputCommand},
};
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_core::message::{DeviceFeature, OutputType};
use buttplug_server::ButtplugServer;
use futures::{Stream, StreamExt};
use serde::Serialize;
use thiserror::Error;
use tokio::net::TcpListener;

#[derive(Error, Debug)]
enum IntifaceRestError {
  #[error("JsonRejection: {0}")]
  JsonRejection(JsonRejection),
  #[error("Library Error: {0}")]
  ButtplugClientError(ButtplugClientError),
  #[error("Device index {0} does not refer to a currently connected device.")]
  InvalidDevice(u32),
  #[error("Device index {0} feature index {1} does not refer to a valid device feature.")]
  InvalidFeature(u32, u32),
  /*
  #[error("{0} is not a valid output type. Valid output types are: {1:?}")]
  InvalidOutputType(String, Vec<OutputType>),
  #[error("{0} is not a valid input type. Valid input types are: {1:?}")]
  InvalidInputType(String, Vec<String>),
  #[error("{0} is not a valid input commands. Valid input commands are: {1:?})")]
  InvalidInputCommand(u32, Vec<String>),
  #[error("Value {0} is not valid for the current command.)")]
  InvalidValue(u32),
  */
}

// Tell axum how `AppError` should be converted into a response.
//
// This is also a convenient place to log errors.
impl IntoResponse for IntifaceRestError {
  fn into_response(self) -> Response {
    let (status, message) = match self {
      IntifaceRestError::JsonRejection(rejection) => {
        // This error is caused by bad user input so don't log it
        (rejection.status(), rejection.body_text())
      }
      _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
    };
    (status, message).into_response()
  }
}

impl From<JsonRejection> for IntifaceRestError {
  fn from(rejection: JsonRejection) -> Self {
    Self::JsonRejection(rejection)
  }
}

impl From<ButtplugClientError> for IntifaceRestError {
  fn from(error: ButtplugClientError) -> Self {
    Self::ButtplugClientError(error)
  }
}

#[derive(Serialize)]
struct IntifaceRestDevice {
  index: u32,
  name: String,
  display_name: Option<String>,
  features: BTreeMap<u32, DeviceFeature>,
}

impl From<&ButtplugClientDevice> for IntifaceRestDevice {
  fn from(device: &ButtplugClientDevice) -> Self {
    Self {
      index: device.index(),
      name: device.name().clone(),
      display_name: device.display_name().clone(),
      features: device
        .device_features()
        .iter()
        .map(|(i, d)| (*i, d.feature().clone()))
        .collect(),
    }
  }
}

pub struct IntifaceRestServer {}

fn get_device(
  client: &ButtplugClient,
  index: u32,
) -> Result<ButtplugClientDevice, IntifaceRestError> {
  client
    .devices()
    .get(&index)
    .ok_or(IntifaceRestError::InvalidDevice(index))
    .cloned()
}

fn get_feature(
  client: &ButtplugClient,
  index: u32,
  feature_index: u32,
) -> Result<ClientDeviceFeature, IntifaceRestError> {
  get_device(client, index)?
    .device_features()
    .get(&feature_index)
    .ok_or(IntifaceRestError::InvalidFeature(index, feature_index))
    .cloned()
}

async fn start_scanning(
  State(client): State<Arc<ButtplugClient>>,
) -> Result<(), IntifaceRestError> {
  client
    .start_scanning()
    .await
    .map_err(IntifaceRestError::ButtplugClientError)
}

async fn stop_scanning(State(client): State<Arc<ButtplugClient>>) -> Result<(), IntifaceRestError> {
  client
    .stop_scanning()
    .await
    .map_err(IntifaceRestError::ButtplugClientError)
}

async fn stop_all_devices(
  State(client): State<Arc<ButtplugClient>>,
) -> Result<(), IntifaceRestError> {
  client
    .stop_all_devices()
    .await
    .map_err(IntifaceRestError::ButtplugClientError)
}

async fn stop_device(
  State(client): State<Arc<ButtplugClient>>,
  Path(index): Path<u32>,
) -> Result<(), IntifaceRestError> {
  get_device(&client, index)?
    .stop()
    .await
    .map_err(IntifaceRestError::ButtplugClientError)
}

async fn set_device_output(
  State(client): State<Arc<ButtplugClient>>,
  Path((index, output_type, level)): Path<(u32, OutputType, f64)>,
) -> Result<(), IntifaceRestError> {
  let cmd = ClientDeviceOutputCommand::from_command_value_float(output_type, level)
    .map_err(IntifaceRestError::ButtplugClientError)?;

  get_device(&client, index)?
    .send_command(&cmd)
    .await
    .map_err(IntifaceRestError::ButtplugClientError)
}

async fn set_feature_output(
  State(client): State<Arc<ButtplugClient>>,
  Path((index, feature_index, output_type, level)): Path<(u32, u32, OutputType, f64)>,
) -> Result<(), IntifaceRestError> {
  let cmd = ClientDeviceOutputCommand::from_command_value_float(output_type, level)
    .map_err(IntifaceRestError::ButtplugClientError)?;

  get_feature(&client, index, feature_index)?
    .send_command(&cmd)
    .await
    .map_err(IntifaceRestError::ButtplugClientError)
}

async fn get_devices(
  State(client): State<Arc<ButtplugClient>>,
) -> Json<BTreeMap<u32, IntifaceRestDevice>> {
  client
    .devices()
    .iter()
    .map(|(i, x)| (*i, x.into()))
    .collect::<BTreeMap<u32, IntifaceRestDevice>>()
    .into()
}

async fn get_device_info(
  State(client): State<Arc<ButtplugClient>>,
  Path(index): Path<u32>,
) -> Result<Json<IntifaceRestDevice>, IntifaceRestError> {
  Ok(
    IntifaceRestDevice::from(
      client
        .devices()
        .get(&index)
        .ok_or(IntifaceRestError::InvalidDevice(index))?,
    )
    .into(),
  )
}

async fn get_features(
  State(client): State<Arc<ButtplugClient>>,
  Path(index): Path<u32>,
) -> Result<Json<BTreeMap<u32, DeviceFeature>>, IntifaceRestError> {
  Ok(
    get_device(&client, index)?
      .device_features()
      .iter()
      .map(|(i, f)| (*i, f.feature().clone()))
      .collect::<BTreeMap<u32, DeviceFeature>>()
      .into(),
  )
}

async fn get_feature_info(
  State(client): State<Arc<ButtplugClient>>,
  Path((index, feature_index)): Path<(u32, u32)>,
) -> Result<Json<DeviceFeature>, IntifaceRestError> {
  Ok(
    get_device(&client, index)?
      .device_features()
      .get(&feature_index)
      .ok_or(IntifaceRestError::InvalidFeature(index, feature_index))?
      .feature()
      .clone()
      .into(),
  )
}

/*
async fn feature_input_command(
  State(client): State<Arc<ButtplugClient>>,
  Path((index, feature_index, input_type, command)): Path<(u32, u32, String, String)>,
) -> Result<(), IntifaceRestError> {
  let cmd = convert_output_command(&command, level)?;

  Ok(
    get_feature(&client, index, feature_index)?
      .send_command(&cmd)
      .await
      .map_err(|e| IntifaceRestError::ButtplugClientError(e))?,
  )

  Ok(())
}
*/

async fn server_sse(
  State(client): State<Arc<ButtplugClient>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
  let stream = client
    .event_stream()
    .map(|e| Ok(Event::default().data(format!("{:?}", e))));

  Sse::new(stream).keep_alive(KeepAlive::default())
}

impl IntifaceRestServer {
  pub async fn run(port: u16, server: ButtplugServer) -> Result<(), io::Error> {
    let connector = ButtplugInProcessClientConnectorBuilder::default()
      .server(server)
      .finish();
    let client = ButtplugClient::new("Intiface REST API");
    client.connect(connector).await.unwrap();
    info!("Setting up app!");
    // pass incoming GET requests on "/hello-world" to "hello_world" handler.
    let app = Router::new().nest(
      "/api/v1",
      Router::new()
        .route("/start-scanning", get(start_scanning))
        .route("/stop-scanning", get(stop_scanning))
        .route("/devices", get(get_devices))
        .route("/devices/stop", put(stop_all_devices))
        .route("/devices/{index}", get(get_device_info))
        .route("/devices/{index}/stop", put(stop_device))
        .route("/devices/{index}/features", get(get_features))
        .route("/devices/{index}/features/{index}/", put(get_feature_info))
        .route(
          "/devices/{index}/outputs/{output_type}/{level}",
          put(set_device_output),
        )
        .route(
          "/devices/{index}/features/{index}/outputs/{output_type}/{level}",
          put(set_feature_output),
        )
        /*
        .route(
          "/devices/{index}/inputs/{input_type}/{input_command}",
          put(device_input_command),
        )
        .route(
          "/devices/{index}/features/{index}/inputs/{input_type}/{input_command}",
          put(feature_input_command),
        )
        .route("/devices/{index}/events", get(device_sse))
         */
        .route("/events", get(server_sse))
        //.route("/devices/{*index}/vibrate", post(set_feature_vibrate_speed))
        .with_state(Arc::new(client)),
    );

    // write address like this to not make typos
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Buttplug REST API Server now listening on {:?}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
  }
}
