// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
use buttplug::{
  client::ButtplugClient,
  connector::{ButtplugRemoteClientConnector, ButtplugWebsocketClientTransport},
  core::messages::serializer::ButtplugClientJSONSerializer,
  util::async_manager
};

// We're gonna use async_std as our runtime for the examples, but you should be
// able to use futures, tokio, or whatever else.
#[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
use async_std::task;

#[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
async fn embedded_connector_example() {
  env_logger::init();
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
  // "ws://localhost:12345/buttplug", so we'll use that. If you are trying to
  // connect to another machine, you'll need to change this address to point
  // to that machine. The second argument specifies whether we should ignore
  // secure cert validity, but we're not connecting to a secure server so it
  // doesn't really matter here.
  let connector = ButtplugRemoteClientConnector::<
    ButtplugWebsocketClientTransport,
    ButtplugClientJSONSerializer,
  >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
    "ws://localhost:12345",
  ));

  // ButtplugClient creation is the same as the last example. From here on
  // out, things look basically the same, EXCEPT for the fact that, unlike the
  // mebedded connector, this can fail! If it does, the unwrap on run() will
  // panic and you'll get an error message about not being able to connect.
  let (client, _) = ButtplugClient::connect("Example Client", connector)
    .await
    .unwrap();
  println!("Is the client connected? {}", client.connected());

  // We don't actually have anything to do here yet, since we're just
  // showing off how to set up execution. We'll just fall out of our
  // closure here.
  println!("Exiting example");

  // That's it for remote settings. Congrats, you've vaguely sorta teledildonicsed! At
  // least with two processes on the same machine, but hey, that's remote, right?
}

fn main() {
  // Setup a client, and wait until everything is done before exiting.
  #[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
  async_manager::block_on(async {
    embedded_connector_example().await;
  });
}
