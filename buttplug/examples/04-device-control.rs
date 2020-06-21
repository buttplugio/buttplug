// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// Let's make something move! In this example, we'll see how to tell what a
// device can do, then send it a command (assuming it vibrates)!

use buttplug::{
  client::{
    device::ButtplugClientDevice, device::VibrateCommand, ButtplugClient, ButtplugClientEvent,
  },
  core::messages::ButtplugDeviceMessageType,
  util::async_manager
};
use futures::StreamExt;
use futures_timer::Delay;
use std::time::Duration;

async fn device_control_example() {
  // Onto the final example! Controlling devices.

  // Instead of setting up our own connector for this example, we'll use the
  // connect_in_process convenience method. This creates an in process connector
  // for us, and also adds all of the device managers built into the library to
  // the server it uses. Handy!
  let (client, mut event_stream) = ButtplugClient::connect_in_process("Example Client", 0)
    .await
    .unwrap();

  // We'll mostly be doing the same thing we did in example #3, up until we get
  // a device.
  if let Err(err) = client.start_scanning().await {
    println!("Client errored when starting scan! {}", err);
    return;
  }

  let vibrate_device = |dev: ButtplugClientDevice| {
    async move {
      // Ok, so we now have a connected client with a device set up. Let's start
      // sending some messages to make the device do things!
      //
      // It's worth noting that at the moment, a client knowing about a device
      // is enough to assume that device is connected to the server and ready to
      // use. So if a client has a device in its list, we can just start sending
      // control messages.
      //
      // We'll need to see which messages our device handles. Luckily, devices
      // hold this information for you to query.
      //
      // When building applications, we can use allowed_messages to see what
      // types of messages whatever device handed to us can take, and then
      // react accordingly.
      //
      // Each entry of allowed_messages will have two pieces of information
      //
      // - Message Type, which will represent the classes of messages we can
      //   send
      //
      // - Message Attributes, which can vary depending on the type of message
      //
      // For instance the VibrateCmd message will have a name of "VibrateCmd",
      // and a "FeatureCount" of 1 < x < N, depending on the number of
      // vibration motors the device has. Messages that don't have a
      // FeatureCount will leave Option<FeatureCount> as None.
      //
      // Since we don't know what kind of device we'll be getting here, we
      // just assume it will be something that vibrates.
      //
      // Devices have "generic" commands for vibrate, rotate, and linear
      // (movement). Each of these takes a enum that is either:
      //
      // - A single value to send to all features. For instance if a device
      //   has 6 vibrators, and we send one speed, all 6 vibrators will be set
      //   to that speed.
      //
      // - A map of index/value pairs, which allows setting certain device
      //   feature indexes to certain values.
      //
      // - A vector of values, which can address most or all feature indexes.
      //
      // For this example, we'll use the simple single value.
      if dev
        .allowed_messages
        .contains_key(&ButtplugDeviceMessageType::VibrateCmd)
      {
        dev.vibrate(VibrateCommand::Speed(1.0)).await.unwrap();
        println!("{} should start vibrating!", dev.name);
        Delay::new(Duration::from_secs(1)).await;
        // All devices also have a "stop" command that will make
        // them stop whatever they're doing.
        dev.stop().await.unwrap();
        println!("{} should stop vibrating!", dev.name);
        Delay::new(Duration::from_secs(1)).await;
      } else {
        println!("{} doesn't vibrate! This example should be updated to handle rotation and linear movement!", dev.name);
      }
    }
  };

  loop {
    match event_stream.next().await.unwrap() {
      ButtplugClientEvent::DeviceAdded(dev) => {
        println!("We got a device: {}", dev.name);
        let fut = vibrate_device(dev);
        async_manager::spawn(async move {
          fut.await;
        }).unwrap();
        // break;
      }
      ButtplugClientEvent::ServerDisconnect => {
        // The server disconnected, which means we're done here, so just
        // break up to the top level.
        println!("Server disconnected!");
        break;
      }
      _ => {
        // Something else happened, like scanning finishing, devices
        // getting removed, etc... Might as well say something about it.
        println!("Got some other kind of event we don't care about");
      }
    }
  }

  // And now we're done!
  println!("Exiting example");
}

fn main() {
  async_manager::block_on(async {
    device_control_example().await;
  });
}
