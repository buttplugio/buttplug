// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[cfg(feature = "websockets")]
use buttplug::{
  client::{ButtplugClient, ButtplugClientEvent},
  core::{
    connector::{ButtplugRemoteClientConnector, ButtplugWebsocketClientTransport},
    message::serializer::ButtplugClientJSONSerializer,
  },
};
use futures::StreamExt;
use tracing_subscriber;

#[cfg(feature = "websockets")]
async fn websocket_connector_example() {
  tracing_subscriber::fmt::init();
  println!(
    "Setting up the client! Run this with RUST_LOG if you'd like to see library log messages."
  );

  // Welcome to the second example. Now, instead of embedding the server in
  // the client, we'll connect to an outside instance of Intiface Desktop.
  //
  // For this, you'll need to download Intiface Desktop, which you can get at
  // https://github.com/intiface/intiface-desktop. If you REALLY HATE
  // electron, you can just use the bare intiface-cli-rs utility (that
  // intiface desktop wraps) at https://github.com/intiface/intiface-cli-rs.
  //
  // If you device to go with Intiface Desktop, install it, start it, then
  // choose "Insecure Websockets" and hit "Start Server".
  //
  // For intiface-cli-rs, run "intiface-cli --wsinsecureport 12345"
  //
  // As with the last example, we'll need a connector first. This time,
  // instead of holding a server ourselves in the connector, the server will
  // be located elsewhere. In this case, it'll most likely be another process
  // on the same computer, though remote connections over networks are
  // certainly possible.
  //
  // This time, instead of specifying a Server Name, we now specify a server
  // network address. The default server address is
  // "ws://127.0.0.1:12345/buttplug", so we'll use that. If you are trying to
  // connect to another machine, you'll need to change this address to point
  // to that machine. The second argument specifies whether we should ignore
  // secure cert validity, but we're not connecting to a secure server so it
  // doesn't really matter here.
  let connector = ButtplugRemoteClientConnector::<
    ButtplugWebsocketClientTransport,
    ButtplugClientJSONSerializer,
  >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
    "ws://127.0.0.1:12345",
  ));

  let client = ButtplugClient::new("exmaple client");
  // ButtplugClient creation is the same as the last example. From here on
  // out, things look basically the same, EXCEPT for the fact that, unlike the
  // mebedded connector, this can fail! If it does, the unwrap on run() will
  // panic and you'll get an error message about not being able to connect.
  if let Err(e) = client.connect(connector).await {
    println!("Client connection failed! {}", e);
    return;
  }
  println!("Is the client connected? {}", client.connected());
  println!("Waiting for server disconnect...");
  let mut event_stream = client.event_stream();
  while let Some(event) = event_stream.next().await {
    match event {
      ButtplugClientEvent::ServerDisconnect => {
        println!("Received server disconnect event.");
        break;
      }
      _ => {}
    }
  }
  // We don't actually have anything to do here yet, since we're just
  // showing off how to set up execution. We'll just fall out of our
  // closure here.
  println!("Exiting example");

  // That's it for remote settings. Congrats, you've vaguely sorta teledildonicsed! At
  // least with two processes on the same machine, but hey, that's remote, right?
}

#[tokio::main]
async fn main() {
  // Setup a client, and wait until everything is done before exiting.
  #[cfg(feature = "websockets")]
  websocket_connector_example().await;
}
