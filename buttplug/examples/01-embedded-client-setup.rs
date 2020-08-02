// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// To begin our exploration of the Buttplug library, we're going to set up a client
// with an embedded connector.
//
// To do this, we're going to need to bring in the main Buttplug crate. It's
// literally just called "Buttplug" (https://crates.io/crates/buttplug/). This
// contains all of the base classes for Buttplug core, client, and server.
//
// Unlike other implementation (like JS and C#), where Buttplug required
// multiple packages for different features, the Rust Buttplug crate contains
// everything you need to build Buttplug applications. Aren't Cargo Features
// great?
use buttplug::{
  client::ButtplugClient,
  connector::ButtplugInProcessClientConnector,
  util::async_manager,
};

async fn embedded_connector_example() {
  tracing_subscriber::fmt::init();
  println!(
    "Setting up the client! Run this with RUST_LOG if you'd like to see library log messages."
  );

  // We'll need a connector first, as creating a client requires a connector.
  // Connectors are how clients connect to servers. Since we're just starting
  // out and don't want to deal with networks or IPC yet, we'll create an
  // embedded client. This means that the Connector holds a Buttplug Server
  // itself, so everything happens locally and in-process. This is usually the
  // easiest case to develop with.
  //
  // For now, we'll just give the server a name. We'll go over other server
  // constructor arguments in later examples.
  let connector = ButtplugInProcessClientConnector::new("Example Server", 0);

  // Now that we've got a connector, we can use the ButtplugClient::connect()
  // function to spin up our client event loop. We pass this function three
  // things:
  //
  // - The client name, which is sent to the server so we can identify what's
  //   connected on that end if the server has a GUI.
  // - The connector we just made, used to connect to the Server
  //
  // The connect() function will take our connector, create a client, and try to
  // connect that client to the server (which, with an embedded connector,
  // should always succeed). It will return the client itself, as well as an
  // event receiver for listening for things like device connection and
  // disconnection, log messages, and other events.
  //
  // connect() can also return an error in certain situations, like not being
  // able to connect to the server.
  let (client, _) = ButtplugClient::connect("Example Client", connector)
    .await
    .unwrap();
  println!("Is the client connected? {}", client.connected());

  // We don't actually have anything to do here yet, since we're just
  // showing off how to set up execution. We'll just fall out of our
  // closure here.
  println!("Exiting example");

  // That's it for the basics of setting up, connecting, and disconnecting a client.
}

fn main() {
  // Setup a client, and wait until everything is done before exiting.
  async_manager::block_on(async {
    embedded_connector_example().await;
  });
}
