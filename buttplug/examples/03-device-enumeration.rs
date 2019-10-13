// private static async Task WaitForKey()
// {
//     Console.WriteLine("Press any key to continue.");
//     while (!Console.KeyAvailable)
//     {
//         await Task.Delay(1);
//     }
//     Console.ReadKey(true);
// }

private static async Task RunExample()
{
    // Time to see what devices are available! In this example, we'll see how servers can
    // access certain types of devices, and how clients can ask servers which devices are available.

    // Since we're going to need to manage our server and client, this example will use an embedded connector.

    // var connector = new ButtplugEmbeddedConnector("Example Server");

    // ButtplugClient creation is the same as always.

    // var client = new ButtplugClient("Example Client", connector);

    // We're to the new stuff. When we create a ButtplugEmbeddedConnector, it in turn creates
    // a Buttplug Server to hold (unless we pass it one to use, which we won't be doing until
    // later examples). If you're just interested in creating Buttplug Client applications
    // that will access things like the Windows Buttplug Server, you won't have to set up the
    // server like this, but this is good knowledge to have anyways, so it's recommended to
    // at least read through this.
    //
    // When a Buttplug Server is created, it in turn creates a Device Manager. The Device
    // Manager is basically the hub of all hardware communication for Buttplug. A Device
    // Manager will hold multiple Device Subtype Managers, which is where we get to specifics
    // about hardware busses and communications. For instance, as of this writing, Buttplug
    // currently ships with Device Subtype Managers for
    //
    // - Bluetooth LE (C# Win10/Typescript)
    // - USB Raw (C# Win7/Win10)
    // - USB HID (C# Win7/Win10)
    // - Serial (C# Win7/Win10)
    // - XInput/XBox Gamepads (C# Win7/Win10)
    // - Test/Simulator (C#/Typescript)
    //
    // When creating a Server, if we don't add subtype managers ourselves, the server will go
    // looking for them in DLLs around the executable on the first time we call
    // StartScanning(). This means you can simply add SubtypeManager nuget dependencies, and
    // they'll instantly be brought in when you start looking for devices.
    //
    // We can also specify which device subtype managers we want to use manually, if we want.
    // For this example, we'll just add a TestDeviceManager so we don't have to deal with
    // actual hardware. This requires manual setup.
    //
    // To do this, we'll get the server from the connector.

    // var server = connector.Server;

    // Then we add a TestDeviceManager to the server. Due to how our logging system works,
    // the server needs to be able to give the log manager it owns to the new device subtype
    // manager. That means we pass in a closure to create the manager.
    //
    // In this case, we also have to create a Test Device, since we aren't working with
    // actual hardware. This step won't normally be required if you're working with a
    // hardware subtype manager.

    // var testDevice = new TestDevice(new ButtplugLogManager(), "Test Device");
    // server.AddDeviceSubtypeManager(
    //     aLogManager => new TestDeviceSubtypeManager(testDevice));

    // If you'd like to see what manual setup looks like with an actual hardware manager,
    // here's how we'd add the XInput (Xbox Gamepad) manager to the server.
    //
    // server.AddDeviceSubtypeManager((IButtplugLogManager aLogManager) => new XInputGamepadManager(aLogManager));

    // Now that the server has at least one device subtype manager, whenever we ask it to
    // scan for devices, it will use the subtype manager to find new devices that it
    // supports. However, we need a way to know in the client when devices connect and
    // disconnect, so we'll need to set up event handlers.
    //
    // THIS NEXT PART IS IMPORTANT, HENCE CAPS.
    //
    // Client device connection event handlers should be set up BEFORE you connect a client
    // to a server. The server can fire device connection events at 2 points.
    //
    // - When a client first connects, if the server has a device connection it is already holding.
    // - During device scanning.
    //
    // If you do not have event handlers set up before connecting, you may miss connection events.
    //
    // A quick aside on why a server could hold devices. There are a few reasons this could
    // happen, some chosen, some forced.
    //
    // - On Windows 10, it is sometimes difficult to get bluetooth LE devices to disconnect,
    // so some software (including the Windows Buttplug Server) leaves devices connected
    // until either the device is powered off/taken out of bluetooth range, or the program terminates.
    //
    // - Depending on how a server is being used, parts of it like a device manager may stay
    // alive between client connections. This would mean that if a client disconnected from a
    // server then reconnected quickly, setup steps wouldn't have to happen again.
    //
    // Anyways, let's set up some simple event handlers.

    // client.DeviceAdded += (aObj, aDeviceEventArgs) =>
    //     Console.WriteLine($"Device {aDeviceEventArgs.Device.Name} Connected!");

    // client.DeviceRemoved += (aObj, aDeviceEventArgs) =>
    //     Console.WriteLine($"Device {aDeviceEventArgs.Device.Name} Removed!");

    // Now that everything is set up, we can connect.

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

    // We're connected, yay!

    // Console.WriteLine("Connected!");

    // It's time to ask the server what devices it can find. We'll do this using a pair of
    // calls, StartScanning() and StopScanning(), and an event handler, ScanningFinished.
    //
    // We start by calling StartScanning(), which tells all of the device subtype managers to
    // scan for whatever devices they manage. Some managers may scan and finish immediately,
    // while others like Bluetooth can take some time to find devices. Once the devices we
    // want to use are found, we can call StopScanning(), and once all scanning has ceased,
    // the ScanningFinish event will fire (which allows you to do things like updating UI).
    //
    // Sometimes, when all device managers are finished scanning, the ScanningFinished event
    // can be fired even without calling StopScanning, so that should be set up first.

    // client.ScanningFinished += (aObj, aScanningFinishedArgs) =>
    //     Console.WriteLine("Device scanning is finished!");

    // Now we can start scanning for devices, and any time a device is found, we should see
    // the device name printed out. Since we're just using the Test Device Manager here, we
    // expect that we'll see the Test Device name, then the scanning finished message.

    // await client.StartScanningAsync();
    // await WaitForKey();

    // The Test Subtype Manager will scan until we still it to stop, so let's stop it now.

    // await client.StopScanningAsync();
    // await WaitForKey();

    // Since we've scanned, the client holds information about devices it knows about for
    // us. These devices can be accessed with the Devices getter on the client.

    // Console.WriteLine("Client currently knows about these devices:");
    // foreach (var device in client.Devices)
    // {
    //     Console.WriteLine($"- {device.Name}");
    // }

    // await WaitForKey();

    // To show what happens when a device disconnects, we'll force the test device to
    // disconnect, which simulates the device powering off, going out of range, or doing
    // something else that makes it no longer connected to the server. This should fire off
    // the DeviceRemoved event.

    // testDevice.Disconnect();
    // await WaitForKey();

    // And now we disconnect as usual.

    // await client.DisconnectAsync();

    // Now we can connect and see what devices we have, so next we'll learn about sending
    // them commands!
}

fn main() {
}
