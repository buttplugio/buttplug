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
use buttplug::client::{connectors::ButtplugEmbeddedClientConnector, ButtplugClient};

// We're gonna use async_std as our runtime for the examples, but you should be
// able to use futures, tokio, or whatever else.
use async_std::task;

async fn embedded_connector_example() {
    // We'll need a connector first, as creating a client requires a connector.
    // Connectors are how clients connect to servers. Since we're just starting
    // out and don't want to deal with networks or IPC yet, we'll create an
    // embedded client. This means that the Connector holds a Buttplug Server
    // itself, so everything happens locally and in-process. This is usually the
    // easiest case to develop with.
    //
    // For now, we'll just give the server a name. We'll go over other server
    // constructor arguments in later examples.
    let connector = ButtplugEmbeddedClientConnector::new("Example Server", 0);

    // Now that we've got a connector, we can use the ButtplugClient::run()
    // function to spin up our client event loop. We pass this function three
    // things:
    //
    // - The client name, which is sent to the server so we can identify what's
    // connected on that end if the server has a GUI.
    // - The connector we just made, used to connect to the Server
    // - A closure that will run whatever we want to do with Buttplug.
    //
    // The run() function will take our connector, create a client, try to connect
    // that client to the server (which, with an embedded connector, should
    // always succeed), and then will run the closure, passing it the connected
    // client instance.
    //
    // run() will block until we exit our closure. This could happen for
    // multiple reasons, like the client or server disconnecting, or our
    // application signalling that it's done doing whatever it wants to do with
    // buttplug.
    //
    // run() can also return an error in certain situations, like not being able
    // to connect to the server.
    //
    // For sake of clarity and indentation, we'll define our closure first, then
    // pass it into run().
    //
    // Note that the closure is (ButtplugClient) -> impl Future. We'll explain
    // why in the definition.
    let app_closure = |mut client: ButtplugClient| {
        // Things get a little weird in here, as we need to
        // return a Future from our closure for the event
        // loop to run. Inside this async block is where
        // we'll usually put our Buttplug application code.
        async move {
            // We'll just have the client disconnect itself.
            // Since we know we're connected if we've gotten
            // this far, we can just unwrap here.
            client.disconnect().await.unwrap();
        }
    };
    ButtplugClient::run("Example Client", connector, app_closure)
        .await
        .unwrap();

    // That's it for the basics of setting up, connecting, and disconnecting a client.
}

fn main() {
    // Setup a client, and wait until everything is done before exiting.
    task::block_on(async {
        embedded_connector_example().await;
    });
}
