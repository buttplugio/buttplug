// use buttplug::client::{connector::ButtplugEmbeddedClientConnector, ButtplugClient};

// // TODO These comments are pulled from the C# examples, so they need updating.

// async fn embedded_connector_example() {
//     // To begin our exploration of the Buttplug library, we're going to set up a client
//     // with an embedded connector.

//     // To do this, we're going to need to bring in the main Buttplug nuget package. It's
//     // literally just called "Buttplug" (https://www.nuget.org/packages/Buttplug/). This
//     // contains all of the base classes for Buttplug core, client, and server. There are
//     // other packages on nuget for Buttplug, mostly dealing with connecting devices or
//     // clients/servers in ways that require more library dependencies. These will be covered
//     // in later tutorials.

//     // We'll need a connector first, as creating a client requires a connector. Connectors
//     // are how clients connect to servers. Since we're just starting out and don't want to
//     // deal with networks or IPC yet, we'll create an embedded client. This means that the
//     // Connector holds a Buttplug Server itself, so everything happens locally. This is
//     // usually the easiest case to develop with.
//     //
//     // For now, we'll just give the server a name. We'll go over other server constructor
//     // arguments in later examples.
//     let connector = ButtplugEmbeddedClientConnector::new("Example Server", 0);

//     // Now that we've got a connector, we can create a ButtplugClient. The client object will
//     // be how we communicate with Buttplug, and to the hardware that Buttplug deals with.
//     //
//     // We can give the client a name, which is sent to the server so we can identify what's
//     // connected on that end if the server has a GUI. We also hand it the connector we just
//     // made, so it knows how/what to connect to when we call Connect() on it.
//     //
//     // We don't really need to keep ownership of the connector outside of the client, so you
//     // could just pass a newly created object here instead of storing it to a variable, but
//     // since this is an example, verbosity wins the day.
//     let mut client = ButtplugClient::new("Example Client");

//     // Time to get a connection going! Due to sometimes having to make network connections
//     // that can take a while, this call is awaited. However, since our connector is embedded
//     // and contains the server, this should pretty much return immediately, so we'll skip
//     // sending it a Token.
//     client.connect(connector).await.unwrap();

//     // Of course, since this is just the connection example, now we're going to turn right
//     // around and disconnect. Once again, this is local, and since we haven't made any device
//     // connections yet it's basically a simple function call, so it should return immediately.
//     //await client.DisconnectAsync();

//     // That's it for the basics of setting up, connecting, and disconnecting a client.
// }

fn main() {
    // Setup a client, and wait until everything is done before exiting.
    // futures::executor::block_on(async { embedded_connector_example().await });
}
