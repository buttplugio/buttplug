

// private static async Task WaitForKey()
// {
//     Console.WriteLine("Press any key to continue.");
//     while (!Console.KeyAvailable)
//     {
//         await Task.Delay(1);
//     }
//     Console.ReadKey(true);
// }

// private static async Task RunExample()
// {
    // Finally! It's time to make something move!
    //
    // (In this case, that "something" will just be a Test Device, so this is actually just a
    // simulation of something moving. Sorry to get you all excited.)

    // Let's go ahead, put our client/server together, and get connected.

    // var connector = new ButtplugEmbeddedConnector("Example Server");
    // var client = new ButtplugClient("Example Client", connector);
    // var server = connector.Server;
    // var testDevice = new TestDevice(new ButtplugLogManager(), "Test Device");
    // server.AddDeviceSubtypeManager(
    //     aLogManager => new TestDeviceSubtypeManager(testDevice));
    // try
    // {
    //     await client.ConnectAsync();
    // }
    // catch (Exception ex)
    // {
    //     Console.WriteLine($"Can't connect to Buttplug Server, exiting! Message: {ex.InnerException.Message}");
    //     await WaitForKey();
    //     return;
    // }
    // Console.WriteLine("Connected!");

    // You usually shouldn't run Start/Stop scanning back-to-back like this, but with
    // TestDevice we know our device will be found when we call StartScanning, so we can get
    // away with it.

    // await client.StartScanningAsync();
    // await client.StopScanningAsync();
    // Console.WriteLine("Client currently knows about these devices:");
    // foreach (var device in client.Devices)
    // {
    //     Console.WriteLine($"- {device.Name}");
    // }

    // await WaitForKey();

    // Ok, so we now have a connected client with a device set up. Let's start sending some
    // messages to make the device do things!
    //
    // It's worth noting that at the moment, a client knowing about a device is enough to
    // assume that device is connected to the server and ready to use. So if a client has a
    // device in its list, we can just start sending control messages.
    //
    // We'll need to see which messages our device handles. Luckily, devices hold this
    // information for you to query.
    //
    // When building applications, we can use AllowedMessages to see what types of messages
    // whatever device handed to us can take, and then react accordingly.

    // foreach (var device in client.Devices)
    // {
    //     Console.WriteLine($"{device.Name} supports the following messages:");
    //     foreach (var msgInfo in device.AllowedMessages)
    //     {

            // msgInfo will have two pieces of information
            // - Message Type, which will represent the classes of messages we can send
            // - Message constraints, which can vary depending on the type of message
            //
            // For instance the VibrateCmd message will have a Type of VibrateCmd (the C#
            // class in our library), and a "FeatureCount" of 1 < x < N, depending on the
            // number of vibration motors the device has. Messages that don't have a
            // FeatureCount will leave FeatureCount as null.
            //
            // Since we're working with a TestDevice, we know it will support 3 different
            // types of messages.
            //
            // - VibrateCmd with a FeatureCount of 2, meaning we can send 2 vibration
            // commands at a time.
            // - SingleMotorVibrateCmd, a legacy message that makes all vibrators on a device
            // vibrate at the same speed.
            // - StopDeviceCmd, which stops all output on a device. All devices should
            // support this message.

    //         Console.WriteLine($"- {msgInfo.Key.Name}");
    //         if (msgInfo.Value.FeatureCount != null)
    //         {
    //             Console.WriteLine($"  - Feature Count: {msgInfo.Value.FeatureCount}");
    //         }
    //     }
    // }

    // Console.WriteLine("Sending commands");

    // Now that we know the message types for our connected device, we can send a message
    // over! Seeing as we want to stick with the modern generic messages, we'll go with VibrateCmd.
    //
    // There's a couple of ways to send this message.

    // var testClientDevice = client.Devices[0];


    // We can use the convenience functions on ButtplugClientDevice to send the message. This
    // version sets all of the motors on a vibrating device to the same speed.

    // await testClientDevice.SendVibrateCmd(1.0);

    // If we wanted to just set one motor on and the other off, we could try this version
    // that uses an array. It'll throw an exception if the array isn't the same size as the
    // number of motors available as denoted by FeatureCount, though.
    //
    // You can get the vibrator count using the following code, though we know it's 2 so we
    // don't really have to use it.

    // var vibratorCount = testClientDevice.GetMessageAttributes<VibrateCmd>().FeatureCount;
    // await testClientDevice.SendVibrateCmd(new[] { 1.0, 0.0 } );

    // await WaitForKey();

    // And now we disconnect as usual.

    // await client.DisconnectAsync();

    // If we try to send a command to a device after the client has disconnected, we'll get
    // an exception thrown.

    // try
    // {
    //     await testClientDevice.SendVibrateCmd(1.0);
    // }
    // catch (ButtplugClientConnectorException e)
    // {
    //     Console.WriteLine("Tried to send a device message after the client disconnected! Exception: ");
    //     Console.WriteLine(e);
    // }
    // await WaitForKey();
// }


fn main() {
}
