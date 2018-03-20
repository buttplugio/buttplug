# Architecture

## Protocol

The Buttplug Standard defines a message based protocol between a client and a server. Note that the use of client and server here does not explicitly denote network connection. These terms are used as a generic way to denote different communication endpoints.

Client are expected to request information from the server about devices that are connected, and to send information to those devices via the server. Servers will handle device enumeration, connection management to hardware, and failure recoveries \(for instance, stopping all connected devices on client disconnect\).

While serialization formats are not yet standardized, current references implementations of the Standard use JSON for serialization. More information on this is available in the Messages section.

## Stages

Buttplug sessions consist of 3 stages. While these stages need not be discrete, due to the way Buttplug will likely be used, they will usually end up being so. Applications may hide or combine some of the stages depending on requirements.

### Identification

During the identification stage, a client will establish connection with the server, and send over its identifying information. The server may trigger some sort of UI event at this point to ask if the user will allow the client to connect and interact.

### Enumeration

After the client/server connection is set up, device enumeration can begin. The client can ask the server to scan for devices on various busses or media \(serial, usb, bluetooth, network, etc\), and return a list of devices it can communicate with.

### Consummation

Once devices are found and selected, we can assume the user will begin interacting with connected devices. At this point, the client will mostly be sending and receiving device commands. It can usually \(but not always\) be assumed that continued enumeration may not be possible due to the context of situations that Buttplug software will be used in.

### Example lifecycle

The following lifecycle covers the general message flow expected between a Buttplug client and a Buttplug server.

```mermaid
sequenceDiagram
  Participant Client
  Participant Server
  
  Note over Client,Server: Once a connection is established, perform the protocol handshake
  Client->>+Server: RequestServerInfo Id=1
  Server->>-Client: ServerInfo Id=1
  
  
  Note over Client,Server: If the server has a non-zero PingTimeout, the client must ensure the server is sent the ping messages at before the timeout is reached. Sending a ping at an interval of half the timeout is a good generalisation.
  loop Every ~half ping timeout
    Client->>+Server: Ping ID=N++
    Server->>-Client: Ok ID=N++
  end
  
  Note over Client,Server: If we're connecting to a server that has already connected to devices (typically during reconnect) the client must call RequestDeviceList to get those devices.
  Client->>+Server: RequestDeviceList Id=2
  Server->>-Client: DeviceList Id=2
  
  Note over Client,Server: To discover new devices, the client must instrct the server to start scanning. 
  Client->>+Server: StartScaning Id=3
  Server->>-Client: Ok Id=3
  
  Note over Client,Server: Whilst the server is scanning, the server will notifiy the client of new devices.
  Server->>Client: DeviceAdded Id=0
  Server->>Client: DeviceAdded Id=0
  
  Note over Client,Server: Once the devices the client is intrested in have been discoved, the client can instruct the server to stop scaning. Once all device managers have stopped scanning, the server will notify the client.
  Client->>+Server: StopScaning Id=4
  Server->>-Client: Ok Id=4
  Server->>Client: ScaningFinished Id=0
  
  Note over Client,Server: Devices may disconnect at any time. The server will notify the client when this happens.
  Server->>Client: DeviceRemoved Id=0
  
  Note over Client,Server: The client may instruct devices to perform actions (these will vary gratly depending on the type of device).
  Client->>+Server: VibrateCmd Id=5
  Server->>-Client: Ok Id=5
  
  Note over Client,Server: The client may instruct the server to stop a device.
  Client->>+Server: StopDeviceCmd Id=6
  Server->>-Client: Ok Id=6
  
  Note over Client,Server: The client may also instruct the server to stop all devices. This is good form for a client that is shutting down.
  Client->>+Server: StopAllDevices Id=7
  Server->>-Client: Ok Id=7
```