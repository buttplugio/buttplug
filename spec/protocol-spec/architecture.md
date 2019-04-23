# Architecture

## Protocol

The Buttplug Standard defines a message based protocol between a
client and a server. Note that the use of client and server here does
not explicitly denote network connection. These terms are used as a
generic way to denote different communication endpoints.

Client are expected to request information from the server about
devices that are connected, and to send information to those devices
via the server. Servers will handle device enumeration, connection
management to hardware, and failure recoveries (for instance, stopping
all connected devices on client disconnect).

While serialization formats are not yet standardized, current
references implementations of the Standard use JSON for serialization.
More information on this is available in the Messages section.

## Stages

Buttplug sessions consist of 3 stages. While these stages need not be
discrete, due to the way Buttplug will likely be used, they will
usually end up being so. Applications may hide or combine some of the
stages depending on requirements.

### Identification

During the identification stage, a client will establish connection
with the server, and send over its identifying information. The server
may trigger some sort of UI event at this point to ask if the user
will allow the client to connect and interact.

### Enumeration

After the client/server connection is set up, device enumeration can
begin. The client can ask the server to scan for devices on various
busses or media (serial, usb, bluetooth, network, etc), and return a
list of devices it can communicate with.

### Consummation

Once devices are found and selected, we can assume the user will begin
interacting with connected devices. At this point, the client will
mostly be sending and receiving device commands. It can usually (but
not always) be assumed that continued enumeration may not be possible
due to the context of situations that Buttplug software will be used
in.

### Example lifecycle

The following lifecycle covers the general message flow expected
between a Buttplug client and a Buttplug server.

<mermaid>
sequenceDiagram
  Participant Client
  Participant Server
&nbsp;
  Note over Client,Server: Once a connection is established,<br/>perform the protocol handshake,<br/>which exchanges information<br/>about identification, versions,<br/>ping times, etc...
  Client->>+Server: RequestServerInfo Id=1
  Server->>-Client: ServerInfo Id=1
&nbsp;
  Note over Client,Server: If the server has a non-zero<br/>PingTimeout, the client must send<br/>a ping message to theserver<br/>before the specified timeout.<br/>A common strategy is to set<br/>the client Ping time to 1/2 the<br/>requested server ping time.
  loop [PingTime/2]
    Client->>+Server: Ping ID=N++
    Server->>-Client: Ok ID=N++
  end
&nbsp;
  Note over Client,Server: The client calls RequestDeviceList<br/>to get a list of already connected<br/>devices.
  Client->>+Server: RequestDeviceList Id=2
  Server->>-Client: DeviceList Id=2
&nbsp;  
  Note over Client,Server: To discover new devices, the client<br/>instructs the server to start<br/>scanning.
  Client->>+Server: StartScanning Id=3
  Server->>-Client: Ok Id=3
&nbsp;  
  Note over Client,Server: While the server is scanning, the<br/>server will notify the client of new</br>devices.
  Server->>Client: DeviceAdded Id=0
  Server->>Client: DeviceAdded Id=0
&nbsp;  
  Note over Client,Server: Once devices have been discovered,<br/> the client instruct the server to</br> stop scanning. Once all device<br/>managers have stopped scanning,<br/>the server will notify the client.
  Client->>+Server: StopScanning Id=4
  Server->>-Client: Ok Id=4
  Server->>Client: ScanningFinished Id=0
&nbsp;  
  Note over Client,Server: Devices may disconnect at any time.<br/>The server will notify the client<br/>when this happens.
  Server->>Client: DeviceRemoved Id=0
&nbsp;  
  Note over Client,Server: The client may instruct devices to<br/>perform actions. Actions vary per<br/>device. Device capabilities are<br/>relayed as part of DeviceAdded and<br/>DeviceList messages.
  Client->>+Server: VibrateCmd Id=5
  Server->>-Client: Ok Id=5
&nbsp;  
  Note over Client,Server: The client may instruct the server to<br/>stop a device from whatever it<br/>may be doing.
  Client->>+Server: StopDeviceCmd Id=6
  Server->>-Client: Ok Id=6
&nbsp;  
  Note over Client,Server: The client may instruct the server to<br/>stop all devices. This is considered<br/> good form for a client that is<br/>shutting down.
  Client->>+Server: StopAllDevices Id=7
  Server->>-Client: Ok Id=7
</mermaid>
