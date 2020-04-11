// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// Time to see what devices are available! In this example, we'll see how
// servers can access certain types of devices, and how clients can ask
// servers which devices are available.
#![type_length_limit="5000000"]
#[allow(unused_imports)]
use async_std::task;
#[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
use buttplug::client::{
    connectors::websocket::ButtplugWebsocketClientConnector, ButtplugClient, ButtplugClientEvent,
};

#[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
async fn device_enumeration_example() {
    // Since as of this writing we don't actually have devices in Rust yet,
    // we'll have to use a websocket connector. You'll want to have Intiface
    // Desktop running with insecure sockets when you try this example for now.
    //
    // (If you're reading this and we have devices in Rust, please file a bug to
    // tell me to update this)
    //
    // Websocket connectors take a address, and whether they should ignore cert
    // verification or not. Since we're not using SSL for this example, that
    // doesn't really matter for now.
    let connector = ButtplugWebsocketClientConnector::new("ws://localhost:12345", true);

    // Since we don't have a server implementation yet, I'll be skipping the
    // explanation of how Device Subtype Managers work for this.

    // Let's talk about when and how you'll get events (in this case,
    // DeviceAdded events) from the server.
    //
    // The server can fire device connection events at 2 points.
    //
    // - When a client first connects, if the server has a device connection it
    // is already holding.
    //
    // - During device scanning.
    //
    // When the client connects as part of ButtplugClient::run(), it asks the
    // server for a list of already connected devices. The server will return
    // these as DeviceAdded events, including a ButtplugDevice instance we can
    // then use to control the device.
    //
    // A quick aside on why a server could hold devices. There are a few reasons
    // this could happen, some chosen, some forced.
    //
    // - On Windows 10, it is sometimes difficult to get bluetooth LE devices to
    // disconnect, so some software (including the Windows Buttplug Server)
    // leaves devices connected until either the device is powered off/taken out
    // of bluetooth range, or the program terminates.
    //
    // - Depending on how a server is being used, parts of it like a device
    // manager may stay alive between client connections. This would mean that
    // if a client disconnected from a server then reconnected quickly, setup
    // steps wouldn't have to happen again.
    //
    // With that out of the way, let's build our client.
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
            loop {
                match client.wait_for_event().await {
                    // Yay we got an event!
                    Ok(event) => match event {
                        ButtplugClientEvent::DeviceAdded(device) => {
                            // And we actually got a device!
                            //
                            // The device we're given is a real
                            // ButtplugClientDevice object. We could control the
                            // device with it if we wanted, but that's coming up
                            // in a later example. For now, we'll just print the
                            // device name then drop our instance of it.
                            println!("We got a device: {}", device.name);
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

                // Hypothetical situation: We've now exited our match block, and
                // realized that hey, we actually wanted that device object we
                // dropped in the DeviceAdded branch!
                //
                // Never fear, you can always ask for a vec of all devices from
                // the client. It requires an await as the devices require
                // creation by the event loop, but it should be pretty quick.
                //
                // As with everything else, since the event loop may have shut
                // down due to server disconnect, this returns a result that
                // will error if that has happened.
                if let Ok(devices) = client.devices().await {
                    println!("Devices currently connected:");
                    for dev in devices {
                        println!("- {}", dev.name);
                    }
                }
            }
            // And now we're done!
            println!("Exiting example");
        }
    };
    ButtplugClient::run("Example Client", connector, app_closure)
        .await
        .unwrap();
}

fn main() {
    #[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
    task::block_on(async {
        device_enumeration_example().await;
    })
}
