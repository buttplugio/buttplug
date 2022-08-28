# Deprecated Messages

The following messages are considered deprecated, either because they've been duplicated by new
messages, or because their message version has changed and they represent an old version of a
message.

While some older versions of messages are not required to be implemented unless a server wants to
support version fallback, other messages, such as deprecated device commands, should most likely be
implemented in all servers. Any reference servers provided by the Buttplug Core Team will support
all messages, even those that have been deprecated.

---
## DeviceList - Spec V0

**Reason for Deprecation:** Superceded by [DeviceList Version 1](enumeration.md#devicelist), which
provides more information about feature limits of generic messages.

**Description:** Server reply to a client request for a device list.

**Introduced In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _Devices_ (array): Array of device objects
  * _DeviceName_ (string): Descriptive name of the device
  * _DeviceIndex_ (unsigned integer): Index used to identify the device when sending Device
    Messages.
  * _DeviceMessages_ (array of strings): Type names of Device Messages that the device will accept.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RequestDeviceList Id=1
    Server->>-Client: DeviceList Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "DeviceList": {
      "Id": 1,
      "Devices": [
        {
          "DeviceName": "TestDevice 1",
          "DeviceIndex": 0,
          "DeviceMessages": ["SingleMotorVibrateCmd", "RawCmd", "KiirooCmd", "StopDeviceCmd"]
        },
        {
          "DeviceName": "TestDevice 2",
          "DeviceIndex": 1,
          "DeviceMessages": ["SingleMotorVibrateCmd", "LovenseCmd", "StopDeviceCmd"]
        }
      ]
    }
  }
]
```
---
## DeviceAdded - Spec V0

**Reason for Deprecation:** Superceded by [DeviceList Version 1](enumeration.md#devicelist), which
provides more information about feature limits of generic messages.

**Introduced In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceName_ (string): Descriptive name of the device
* _DeviceIndex_ (unsigned integer): Index used to identify the device when sending Device Messages.
* _DeviceMessages_ (array of strings): Type names of Device Messages that the device will accept.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    participant Client
    participant Server
    Server->>Client: DeviceAdded Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "DeviceAdded": {
      "Id": 0,
      "DeviceName": "TestDevice 1",
      "DeviceIndex": 0,
      "DeviceMessages": ["SingleMotorVibrateCmd", "RawCmd", "KiirooCmd", "StopDeviceCmd"]
    }
  }
]
```
---
## DeviceList - Spec V1

**Reason for Deprecation:** Superceded by [DeviceList Version 3](enumeration.md#devicelist), which
provides optional display name and message timing information, as well as simplifies the format of message attributes.

**Description:** Server reply to a client request for a device list.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 1 (See [Deprecated
Messages](deprecated.md#devicelist-version-0) for older versions.)

**Fields:**

* _Id_ (unsigned int): Message Id
* _Devices_ (array): Array of device objects
  * _DeviceName_ (string): Descriptive name of the device
  * _DeviceIndex_ (unsigned integer): Index used to identify the device when sending Device Messages.
  * _DeviceMessages_ (dictionary): Accepted Device Messages 
    * Keys (string): Type names of Device Messages that the device will accept
    * Values ([Message
      Attributes](enumeration.md#message-attributes-for-devicelist-and-deviceadded)):
      Attributes for the Device Messages.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RequestDeviceList Id=1
    Server->>-Client: DeviceList Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "DeviceList": {
      "Id": 1,
      "Devices": [
        {
          "DeviceName": "TestDevice 1",
          "DeviceIndex": 0,
          "DeviceMessages": {
            "VibrateCmd": { "FeatureCount": 2 },
            "StopDeviceCmd": {}
          }
        },
        {
          "DeviceName": "TestDevice 2",
          "DeviceIndex": 1,
          "DeviceMessages": {
            "LinearCmd": { "FeatureCount": 1 },
            "StopDeviceCmd": {}
          }
        }
      ]
    }
  }
]
```
---
## DeviceAdded - Spec V1

**Reason for Deprecation:** Superceded by [DeviceAdded Version 3](enumeration.md#deviceadded), which
provides more information about feature limits of generic messages, as well as simplifies the format of message attributes.

**Description:** Sent by the server whenever a device is added to the
system. Can happen at any time after identification, as it is assumed
many server implementations will support devices with hotplugging
capabilities that do not require specific scanning/discovery sessions.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version**: 1 (See [Deprecated
Messages](deprecated.md#deviceadded-version-0) for older versions.)

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceName_ (string): Descriptive name of the device
* _DeviceIndex_ (unsigned integer): Index used to identify the device
  when sending Device Messages.
* _DeviceMessages_ (dictionary): Accepted Device Messages 
  * Keys (string): Type names of Device Messages that the device will accept
  * Values ([Message
    Attributes](enumeration.md#message-attributes-for-devicelist-and-deviceadded)):
    Attributes for the Device Messages.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    participant Client
    participant Server
    Server->>Client: DeviceAdded Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "DeviceAdded": {
      "Id": 0,
      "DeviceName": "TestDevice 1",
      "DeviceIndex": 0,
      "DeviceMessages": {
        "VibrateCmd": { "FeatureCount": 2 },
        "StopDeviceCmd": {}
      }
    }
  }
]
```
---

## Message Attributes - Spec V2

**Reason for Deprecation:** Superceded by Message Attributes Version 3, changing format to be one
message attributes object per device feature, and adding sensor/actuator types, feature descriptors,
etc...

**Introduced In Spec Version:** 1

**Last Updated In Spec Version**: 2

**Description:** A collection of message attributes. This object is always the child of a Device
Message type name within a [DeviceList](enumeration.md#devicelist) or
[DeviceAdded](enumeration.md#deviceadded) message. Not all attributes are relevant for all Device
Messages on all Devices; in these cases the attributes will not be included.

**Attributes:**

* _FeatureCount_ (unsigned int): Number of features the Device Message may address. This attribute
  is used to define the capabilities of generic device control messages. The meaning of "feature" is
  specific to the context of the message the attribute is attached to. For instance, the
  FeatureCount attribute of a VibrateCmd message will refer to the number of vibration motors that
  can be controlled on a device advertising the VibrateCmd message.
* _StepCount_ (array of unsigned int, minimum value: 1): For each feature, lists the number of
  discrete steps the feature can use. Returning to the VibrateCmd example from the above
  _FeatureCount_ specification, if a device had 2 motors, and each motor has 20 steps of vibration
  speeds from 0%-100% (this is exactly what the Lovense Edge is), the _StepCount_ attribute would be
  [20, 20]. Having the array allows use to specify different amounts of steps for multiple vibrators
  on the device.

---
## Message Attributes - Spec V1

**Reason for Deprecation:** Superceded by Message Attributes Version 2, adding step count.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version**: 2

**Description:** A collection of message attributes. This object is always the child of a Device
Message type name within a [DeviceList](enumeration.md#devicelist) or
[DeviceAdded](enumeration.md#deviceadded) message. Not all attributes are relevant for all Device
Messages on all Devices; in these cases the attributes will not be included.

**Attributes:**

* _FeatureCount_ (unsigned int): Number of features the Device Message may address. This attribute
  is used to define the capabilities of generic device control messages. The meaning of "feature" is
  specific to the context of the message the attribute is attached to. For instance, the
  FeatureCount attribute of a VibrateCmd message will refer to the number of vibration motors that
  can be controlled on a device advertising the VibrateCmd message.

---
## RequestServerInfo - Spec V0

**Reason for Deprecation:** Superceded by [RequestServerInfo Version
1](identification.md#requestserverinfo), adding message version check.

**Description:** Sent by the client to register itself with the server, and request info from the server.

**Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _ClientName_ (string): Name of the client, for the server to use for UI if needed. Cannot be null.

**Expected Response:**

* ServerInfo message on success
* Error message on malformed message, null client name, or other error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: RequestServerInfo Id=0
    Server->>Client: ServerInfo Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "RequestServerInfo": {
      "Id": 1,
      "ClientName": "Test Client"
    }
  }
]
```
---
## ServerInfo - Spec V0

**Reason for Deprecation:** Superceded by [ServerInfo Version 1](identification.md#serverinfo),
removing unused version info.

**Description:** Send by server to client, contains information about the server name \(optional\),
template version, and ping time expectations.

**Introduced In Spec Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _ServerName_ \(string\): Name of the server. Can be null \(0-length\).
* _MajorVersion_ \(uint\): Major version of the server software.
* _MinorVersion_ \(uint\): Minor version of the server software.
* _BuildVersion_ \(uint\): Build version of the server software.
* _MessageVersion_ \(uint\): Message template version of the server software.
* _MaxPingTime_ \(uint\): Maximum internal for pings from the client, in milliseconds. If a client
  takes to longer than this time between sending Ping messages, the server is expected to
  disconnect.

**Expected Response:**

None. Server-To-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: RequestServerInfo Id=0
    Server->>Client: ServerInfo Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "ServerInfo": {
      "Id": 1,
      "ServerName": "Test Server",
      "MajorVersion": 1,
      "MinorVersion": 0,
      "BuildVersion": 0,
      "MessageVersion": 1,
      "MaxPingTime": 100
    }
  }
]
```
---
## RawCmd

**Reason for Deprecation:** Message is ill-defined (doesn't specify where the data should go,
assumes all devices have one endpoint which is very not true), was never actually implemented in any
reference implemenation. Being superceded by [Raw\*Cmd](raw.md). As the message was never in any
protocol implementation, it can safely be ignored when implementing new servers, but should also not
be used to name new messages.

**Description:** Used to send a raw byte string to a device. Should only be used for development,
and should not be exposed to untrusted clients.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device
* _Command_ (Array of bytes): Command to send, array of ints with a range of [0-255]. Minimum length
  is 1.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RawCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "RawCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Command": [0, 2, 4]
    }
  }
]
```
---
## SingleMotorVibrateCmd

**Reason for Deprecation:** Superceded by [VibrateCmd](deprecated.md#vibratecmd), which provided
granular control of an unlimited number of motors. Should most likely still be implemented in
servers, in order to support older applications, but is not recommended for use in new client
applications.

**Description:** Causes a device that supports vibration to run all vibration motors at a certain
speed.

**Introduced In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device
* _Speed_ (double): Vibration speed with a range of [0.0-1.0]

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: SingleMotorVibrateCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "SingleMotorVibrateCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Speed": 0.5
    }
  }
]
```
---
## KiirooCmd

**Reason for Deprecation:** Only implemented in early versions of the C# library, did not cover
nearly enough of the vast spectrum of possible commands for Kiiroo devices. Replaced by... pretty
much everything generic.

**Description:** Causes a device that supports Kiiroo style commands to run whatever event may be
related. More information on Kiiroo commands can be found in STPIHKAL.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device
* _Command_ (string): Parsed into an unsigned integer in range [0-4] for position/speed.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: KiirooCmd Id=1
    Server->>Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "KiirooCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Command": "4"
    }
  }
]
```
---
## FleshlightLaunchFW12Cmd

**Reason for Deprecation:** Superceded by [LinearCmd](generic.md#linearcmd), which provided an
easier way to reason about movement time and position.

**Description:** Causes a device that supports Fleshlight Launch (Firmware Version 1.2) style
commands to run whatever event may be related. More information on Fleshlight Launch commands can be
found in STPIHKAL.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device
* _Position_ (unsigned int): Unsigned integer in range [0-99], denoting position to move to.
* _Speed_ (unsigned int): Unsigned integer in range [0-99], denoting speed to requested position at.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: FleshlightLaunchFW12Cmd Id=1
    Server->>Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "FleshlightLaunchFW12Cmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Position": 95,
      "Speed": 90
    }
  }
]
```
---
## LovenseCmd

**Reason for Deprecation:** Never implemented in any reference version of the library. Superceded by
a combination of [VibrateCmd](generic.md#linearcmd), [RotateCmd](generic.md#rotatecmd),
[BatteryCmd](generic.md#batterycmd), and the [Raw*Cmd](raw.md) commands.

**Description:** Causes a device that supports Lovense style commands to run whatever event may be
related. More information on Lovense commands can be found in STPIHKAL.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device
* _Command_ (string): String command for Lovense devices. Must be a valid Lovense command accessible
  on most of their devices. See STPIHKAL for more info. Implementations should check this for
  validity.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: LovenseCmd Id=1
    Server->>Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "LovenseCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Command": "Vibrate:20;"
    }
  }
]
```
---
## VorzeA10CycloneCmd

**Reason for Deprecation:** Superceded by a combination of [VibrateCmd](generic.md#linearcmd) and
[RotateCmd](generic.md#rotatecmd).

**Description:** Causes a device that supports Vorze A10 Cyclone style commands to run whatever
event may be related. More information on Vorze commands can be found in STPIHKAL.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device
* _Speed_ (unsigned int): Unsigned integer in range [0-100], denoting speed to rotate at.
* _Clockwise_ (boolean): Rotation direction

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: VorzeA10CycloneCmd Id=1
    Server->>Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "VorzeA10CycloneCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Speed": 50,
      "Clockwise": true
    }
  }
]
```
---
## Test

**Reason for Deprecation:** Violates the assumption that server and client should not be able to
send the same message type. Not particularly useful either, since the whole protocol is made up of
messages, so if you've send/received one, you're... pretty much good.

**Description:** The Test message is used for development and testing purposes. Sending a Test
message with a string to the server will cause the server to return a Test message. If the string is
"Error", the server will return an error message instead.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _TestString_ (string): String to echo back from server.

**Expected Response:**

* Test message with matching Id and TestString on successful request.
* Error message on value or message error, or TestString being 'Error'.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: Test Id=5 TestString=X
    Server->>-Client: Test Id=5 TestString=X
</mermaid>

**Serialization Example:**

```json
[
  {
    "Test": {
      "Id": 5,
      "TestString": "Moo"
    }
  }
]
```
---
## RequestLog

**Reason for Deprecation:** Requesting logging means that the client, whoever that may be, can
request dumps of information from the server. When the client/server are in the same process, that's
fine. However, when the client may be remote to the server (for instance, a web app accessing
intiface desktop), this allows WAY too much information leakage, as the logging messages may be
quite verbose, unless the server is setup to ignore it. Also, this has nothing to do with
controlling sex toys. It was more for debugging situations that have never really arisen. If we need
logs, we can get them from servers, and we don't really need to fly them over the line.

**Description:** Requests that the server send all internal log messages to the client. Useful for
debugging.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _LogLevel_ (string): The highest level of message to receive. Sending "Off" turns off messages,
  while sending "Trace" denotes that all log messages should be sent to the client. Valid LogLevel
  values:
  * Off
  * Fatal
  * Error
  * Warn
  * Info
  * Debug
  * Trace

**Expected Response:**

* Ok message with matching Id on successful logging request. Assuming the LogLevel was not "Off",
  Log type messages will be received after this.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RequestLog Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "RequestLog": {
      "Id": 1,
      "LogLevel": "Warn"
    }
  }
]
```
---
## Log

**Reason for Deprecation:** See RequestLog reason.

**Description:** Log message from the server. Only sent after the client has sent a RequestLog
message with a level other than "Off".

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _LogLevel_ (string): The level of the log message.
  * Off 
  * Fatal
  * Error
  * Warn
  * Info
  * Debug
  * Trace
* _LogMessage_ (string): Log message.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RequestLog Id=1
    Server->>-Client: Ok Id=1
    Server->>Client: Log Id=0 LogLevel=Warn
    Server->>Client: Log Id=0 LogLevel=Trace
</mermaid>

**Serialization Example:**

```json
[
  {
    "Log": {
      "Id": 0,
      "LogLevel": "Trace",
      "LogMessage": "This is a Log Message."
    }
  }
]
```
---

## VibrateCmd

**Reason for Deprecation:** Superceded by ScalarCmd.

**Description:** Causes a device that supports vibration to run specific vibration motors at a
certain speeds. Devices with multiple vibrator features may take multiple values. The
[FeatureCount](enumeration.md#messageattributes) attribute for the message in the
[DeviceList](enumeration.md#devicelist)/[DeviceAdded](enumeration.md#deviceadded) message will
contain that information.

**Introduced In Spec Version:** 1

**Last Updated In Spec Version:** 1

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device
* _Speeds_ (array): Vibration speeds
  * _Index_ (unsigned int): Index of vibration motor
  * _Speed_ (double): Vibration speed with a range of [0.0-1.0]

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: VibrateCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "VibrateCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Speeds": [
        {
          "Index": 0,
          "Speed": 0.5
        },
        {
          "Index": 1,
          "Speed": 1.0
        }
      ]
    }
  }
]
```
---

## BatteryLevelCmd

**Reason for Deprecation:** Superceded by SensorReadCmd.

**Description:** Requests that a device send its battery level.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to query for battery reading.

**Expected Response:**

* [BatteryLevelReading](sensors.html#batterylevelreading) message with
  matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: BatteryLevelCmd Id=1 DeviceIndex=0
    Server->>Client: BatteryLevelReading Id=1 DeviceIndex=0 BatteryLevel=0.5
</mermaid>

**Serialization Example:**

```json
[
  {
    "BatteryLevelCmd": {
      "Id": 1,
      "DeviceIndex": 0
    }
  }
]
```
---
## BatteryLevelReading

**Reason for Deprecation:** Superceded by SensorReading.

**Description:** Message containing a battery level reading from a
device, as requested by [BatteryLevelCmd](sensors.html#batterylevelcmd).

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device battery reading is from.
* _BatteryLevel_ (double): Battery Level with a range of [0.0-1.0]

**Expected Response:**

* None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: BatteryLevelCmd Id=1 DeviceIndex=0
    Server->>Client: BatteryLevelReading Id=1 DeviceIndex=0 BatteryLevel=0.5
</mermaid>

**Serialization Example:**

```json
[
  {
    "BatteryLevelReading": {
      "Id": 1,
      "DeviceIndex": 0,
      "BatteryLevel": 0.5
    }
  }
]
```
---
## RSSILevelCmd

**Reason for Deprecation:** Superceded by SensorReadCmd. Also never implemented in any official reference implementation of Buttplug.

**Description:** Requests that a device send its RSSI level.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to query for RSSI level.

**Expected Response:**

* [RSSILevelReading](sensors.html#rssilevelreading) message with
  matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: RSSILevelCmd Id=1 DeviceIndex=0
    Server->>Client: RSSILevelReading Id=1 DeviceIndex=0 RSSILevel=-40
</mermaid>

**Serialization Example:**

```json
[
  {
    "RSSILevelCmd": {
      "Id": 1,
      "DeviceIndex": 0
    }
  }
]
```
---
## RSSILevelReading

**Reason for Deprecation:** Superceded by SensorReading. Also never implemented in any official reference implementation of Buttplug.

**Description:** Message containing a RSSI level reading from a
device, as requested by [RSSILevelCmd](sensors.html#rssilevelcmd).

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device the reading is from.
* _RSSILevel_ (int): RSSI Level, usually expressed as db gain, usually [-100:0]

**Expected Response:**

* None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>Server: RSSILevelCmd Id=1 DeviceIndex=0
    Server->>Client: RSSILevelReading Id=1 DeviceIndex=0 RSSILevel=-40
</mermaid>

**Serialization Example:**

```json
[
  {
    "RSSILevelReading": {
      "Id": 1,
      "DeviceIndex": 0,
      "RSSILevel": -40
    }
  }
]
```
