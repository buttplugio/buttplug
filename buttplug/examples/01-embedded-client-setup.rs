// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#![type_length_limit = "24978768"]

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
use buttplug::client::{
    connectors::ButtplugEmbeddedClientConnector, ButtplugClient, ButtplugClientEvent,
};

// We're gonna use async_std as our runtime for the examples, but you should be
// able to use futures, tokio, or whatever else.
use async_std::task;
use buttplug::client::device::{LinearCommand, RotateCommand, VibrateCommand};
#[cfg(any(
    feature = "linux-ble",
    feature = "winrt-ble",
    feature = "corebluetooth-ble"
))]
use buttplug::server::comm_managers::btleplug::BtlePlugCommunicationManager;
use std::time::Duration;

#[cfg(any(
    feature = "linux-ble",
    feature = "winrt-ble",
    feature = "corebluetooth-ble"
))]
async fn embedded_connector_example() {
    env_logger::init();
    // We'll need a connector first, as creating a client requires a connector.
    // Connectors are how clients connect to servers. Since we're just starting
    // out and don't want to deal with networks or IPC yet, we'll create an
    // embedded client. This means that the Connector holds a Buttplug Server
    // itself, so everything happens locally and in-process. This is usually the
    // easiest case to develop with.
    //
    // For now, we'll just give the server a name. We'll go over other server
    // constructor arguments in later examples.
    let mut connector = ButtplugEmbeddedClientConnector::new("Example Server", 0);
    connector.add_comm_manager::<BtlePlugCommunicationManager>();
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
        async move {
            // First, we'll start the server looking for devices.
            if let Err(err) = client.start_scanning().await {
                // If the server disconnected between the time we spun up the
                // loop and now, the scanning will return an error. At that
                // point we should just bail out.
                println!("Client errored when starting scan! {}", err);
                return;
            }
            // Ok, we've started scanning. Now we need to wait to hear back from
            // the server on whether we got anything. To do that, we call
            // wait_for_event.
            //
            // wait_for_event is to Buttplug's Rust implementation what the
            // event handlers in C#/JS were to those implementations. However,
            // since we're not in a GC'd language anymore, event handlers are a
            // bit difficult to implement, so we just have a stream-like
            // function instead.
            //
            // wait_for_event will return a future that waits until it gets
            // something from the server. You can either await that and block
            // until you get something from the server (or race/select it
            // against other futures), or else save the future and use something
            // like a timeout join.
            //
            // For our purposes for the moment, all we care about is receiving
            // new devices, so we'll just loop and wait.
            let mut device = None;
            loop {
                match client.wait_for_event().await {
                    // Yay we got an event!
                    Ok(event) => match event {
                        ButtplugClientEvent::DeviceAdded(dev) => {
                            // And we actually got a device!
                            //
                            // The device we're given is a real
                            // ButtplugClientDevice object. We could control the
                            // device with it if we wanted, but that's coming up
                            // in a later example. For now, we'll just print the
                            // device name then drop our instance of it.
                            println!("We got a device: {}", dev.name);
                            device = Some(dev);
                            break;
                        }
                        ButtplugClientEvent::ServerDisconnect => {
                            // The server disconnected, which means we're done
                            // here, so just break up to the top level.
                            println!("Server disconnected!");
                            break;
                        }
                        _ => {
                            // Something else happened, like scanning finishing,
                            // devices getting removed, etc... Might as well say
                            // something about it.
                            println!("Got some other kind of event we don't care about");
                        }
                    },
                    // Once again, if we disconnected before calling
                    // wait_for_error, we'll get an error back.
                    Err(err) => {
                        println!("Error while waiting for client events: {}", err);
                        break;
                    }
                }
            }

            // Ok, so we now have a connected client with a device set up. Let's
            // start sending some messages to make the device do things!
            //
            // It's worth noting that at the moment, a client knowing about a
            // device is enough to assume that device is connected to the server
            // and ready to use. So if a client has a device in its list, we can
            // just start sending control messages.
            if let Some(mut dev) = device {
                // We'll need to see which messages our device handles. Luckily,
                // devices hold this information for you to query.
                //
                // When building applications, we can use allowed_messages to
                // see what types of messages whatever device handed to us can
                // take, and then react accordingly.
                //
                // Each entry of allowed_messages will have two pieces of
                // information
                //
                // - Message Type, which will represent the classes of messages
                // we can send
                //
                // - Message Attributes, which can vary depending on the type
                // of message
                //
                // For instance the VibrateCmd message will have a name of
                // "VibrateCmd", and a "FeatureCount" of 1 < x < N, depending on
                // the number of vibration motors the device has. Messages that
                // don't have a FeatureCount will leave Option<FeatureCount> as
                // None.
                //
                // Since we don't know what kind of device we'll be getting
                // here, we just assume it will be something that vibrates.
                //
                // Devices have "generic" commands for vibrate, rotate, and
                // linear (movement). Each of these takes a enum that is either:
                //
                // - A single value to send to all features. For instance if a
                // device has 6 vibrators, and we send one speed, all 6
                // vibrators will be set to that speed.
                //
                // - A map of index/value pairs, which allows setting certain
                // device feature indexes to certain values.
                //
                // - A vector of values, which can address most or all feature
                // indexes.
                //
                // For this example, we'll use the simple single value.

                println!("Device: {:?}", dev);

                if dev.name.contains("F1s") {
                    // Give me time to press the button
                    println!("Press the power button once!");
                    task::sleep(Duration::from_secs(5)).await;
                }

                if dev.allowed_messages.contains_key("VibrateCmd") {
                    let count = dev
                        .allowed_messages
                        .get("VibrateCmd")
                        .unwrap()
                        .feature_count
                        .unwrap();

                    if count > 1 {
                        for i in 0..count {
                            let mut speeds: Vec<f64> = vec![];
                            for j in 0..count {
                                speeds.push(if i == j { 1.0 } else { 0.0 });
                            }
                            dev.vibrate(VibrateCommand::SpeedVec(speeds)).await.unwrap();
                            println!("{} should start vibrating on motor {}!", dev.name, i + 1);
                            task::sleep(Duration::from_secs(1)).await;
                        }
                        dev.stop().await.unwrap();
                        println!("{} should stop vibrating!", dev.name);
                        task::sleep(Duration::from_secs(1)).await;
                    }

                    dev.vibrate(VibrateCommand::Speed(1.0)).await.unwrap();
                    println!("{} should start vibrating!", dev.name);
                    task::sleep(Duration::from_secs(1)).await;
                    // All devices also have a "stop" command that will make
                    // them stop whatever they're doing.
                    dev.stop().await.unwrap();
                    println!("{} should stop vibrating!", dev.name);
                    task::sleep(Duration::from_secs(1)).await;
                }

                if dev.allowed_messages.contains_key("RotateCmd") {
                    dev.rotate(RotateCommand::Rotate(1.0, true)).await.unwrap();
                    println!("{} should start rotating!", dev.name);
                    task::sleep(Duration::from_secs(1)).await;
                    dev.rotate(RotateCommand::Rotate(1.0, false)).await.unwrap();
                    println!("{} should start rotating the other way!", dev.name);
                    task::sleep(Duration::from_secs(1)).await;
                    // All devices also have a "stop" command that will make
                    // them stop whatever they're doing.
                    dev.stop().await.unwrap();
                    println!("{} should stop rotating!", dev.name);
                    task::sleep(Duration::from_secs(1)).await;
                }

                if dev.allowed_messages.contains_key("LinearCmd") {
                    dev.linear(LinearCommand::Linear(250, 0.95)).await.unwrap();
                    println!("{} should start moving!", dev.name);
                    task::sleep(Duration::from_secs(1)).await;
                    dev.linear(LinearCommand::Linear(250, 0.05)).await.unwrap();
                    println!("{} should start moving in reverse!", dev.name);
                    task::sleep(Duration::from_secs(1)).await;
                    dev.linear(LinearCommand::Linear(250, 0.95)).await.unwrap();
                    println!("{} should start moving in reverse!", dev.name);
                    task::sleep(Duration::from_secs(1)).await;
                }
            }
            // And now we're done!
            println!("Exiting example");
        }
    };
    ButtplugClient::run("Example Client", connector, app_closure)
        .await
        .unwrap();

    // That's it for the basics of setting up, connecting, and disconnecting a client.
}

fn main() {
    // Setup a client, and wait until everything is done before exiting.
    #[cfg(any(
        feature = "linux-ble",
        feature = "winrt-ble",
        feature = "corebluetooth-ble"
    ))]
    task::block_on(async {
        embedded_connector_example().await;
    });
    ()
}
