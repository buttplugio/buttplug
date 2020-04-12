// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// Time to see what devices are available! In this example, we'll see how
// servers can access certain types of devices, and how clients can ask
// servers which devices are available.
#[allow(unused_imports)]
use async_std::task;
use buttplug::{
    client::{ connectors::ButtplugEmbeddedClientConnector, ButtplugClient, ButtplugClientEvent },
    test::{ TestDeviceCommunicationManager, TestDevice }
};

async fn device_enumeration_example() {

    // Time to see what devices are available! In this example, we'll see how
    // servers can access certain types of devices, and how clients can ask
    // servers which devices are available.

    // Since we're going to need to manage our server and client, this example
    // will use an embedded connector.
    let mut connector = ButtplugEmbeddedClientConnector::new("Example Server", 0);

    // This example will also work with a WebsocketConnector if you want to
    // connect to Intiface Desktop or an intiface-cli instance.

    // We're to the new stuff. When we create a ButtplugEmbeddedConnector, it in
    // turn creates a Buttplug Server to hold (unless we pass it one to use,
    // which we won't be doing until later examples). If you're just interested
    // in creating Buttplug Client applications that will access things like the
    // Windows Buttplug Server, you won't have to set up the server like this,
    // but this is good knowledge to have anyways, so it's recommended to at
    // least read through this.
    //
    // When a Buttplug Server is created, it in turn creates a Device Manager.
    // The Device Manager is basically the hub of all hardware communication for
    // Buttplug. A Device Manager will hold multiple Device Communication
    // Managers, which is where we get to specifics about hardware busses and
    // communications. For instance, as of this writing, Buttplug currently
    // ships with Device Communication Managers for
    //
    // - Bluetooth LE (Windows 10/Mac/Linux/iOS)
    // - XInput/XBox Gamepads (Win >= 7)
    // - Test/Simulator
    //
    // We can specify which device communication managers we want to use. For
    // this example, we'll just add a TestDeviceManager so we don't have to deal
    // with actual hardware. This requires a bit of manual setup.
    //
    // To do this, we'll add the device comm manager. For the test device comm
    // manager, this gets a little complicated. We'll just be emulating a
    // bluetooth device, the Aneros Vivi, by using its bluetooth name.
    
    let (_, test_device_impl_creator) = TestDevice::new_bluetoothle_test_device_impl_creator("Massage Demo");
    TestDeviceCommunicationManager::add_test_device(test_device_impl_creator);
    connector.add_comm_manager::<TestDeviceCommunicationManager>();

    // If we wanted to add a real device manager, like the btleplug manager,
    // we'd run something like this:
    //
    // connector.add_comm_manager::<BtlePlugCommunicationManager>()

    // Anyways, now that we have a manager sorted, Let's talk about when and how
    // you'll get events (in this case, DeviceAdded events) from the server.
    //
    // The server can fire device connection events at 2 points.
    //
    // - When a client first connects, if the server has a device connection it
    //   is already holding.
    //
    // - During device scanning.
    //
    // When the client connects as part of ButtplugClient::run(), it asks the
    // server for a list of already connected devices. The server will return
    // these as DeviceAdded events, including a ButtplugClientDevice instance we
    // can then use to control the device.
    //
    // A quick aside on why a server could hold devices. There are a few reasons
    // this could happen, some chosen, some forced.
    //
    // - On Windows 10, it is sometimes difficult to get bluetooth LE devices to
    //   disconnect, so some software (including the Windows Buttplug Server)
    //   leaves devices connected until either the device is powered off/taken
    //   out of bluetooth range, or the program terminates.
    //
    // - Depending on how a server is being used, parts of it like a device
    //   manager may stay alive between client connections. This would mean that
    //   if a client disconnected from a server then reconnected quickly, setup
    //   steps wouldn't have to happen again.
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
                    break;
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
    task::block_on(async {
        device_enumeration_example().await;
    })
}
