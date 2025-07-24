use std::{io, net::SocketAddr, sync::Arc};

use tokio::net::TcpListener;
use axum::{extract::State, Router, routing::get};
use buttplug_client::ButtplugClient;
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_server::ButtplugServer;

pub struct IntifaceRestServer {
}

async fn start_scanning(State(client): State<Arc<ButtplugClient>>) {
  client.start_scanning().await;
}

async fn stop_scanning(State(client): State<Arc<ButtplugClient>>) {
  client.stop_scanning().await;
}

impl IntifaceRestServer {
  pub async fn run(server: ButtplugServer) -> Result<(), io::Error> {
    let connector = ButtplugInProcessClientConnectorBuilder::default()
      .server(server)
      .finish();
    let client = ButtplugClient::new("Intiface REST API");
    client.connect(connector).await.unwrap();


    // pass incoming GET requests on "/hello-world" to "hello_world" handler.
    let app = Router::new()
        .route("/start-scanning", get(start_scanning))
        .route("/stop-scanning", get(stop_scanning))
        .with_state(Arc::new(client));

    // write address like this to not make typos
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
  }
}