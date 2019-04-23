# Enumeration Messages

Messages relating to finding and getting information about devices
connected to the system.

---
## StartScanning

**Description:** Client request to have the server start scanning for
devices on all busses that it knows about. Useful for protocols like
Bluetooth, which require an explicit discovery phase.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: StartScanning Id=1
    Server->>-Client: Ok Id=1
    Server->>Client: DeviceAdded Id=0
    Server->>Client: DeviceAdded Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "StartScanning": {
      "Id": 1
    }
  }
]
```
---
## StopScanning

**Description:** Client request to have the server stop scanning for
devices. Useful for protocols like Bluetooth, which may not timeout
otherwise.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: StartScanning Id=1
    Server->>-Client: Ok Id=1
    Server->>Client: DeviceAdded Id=0
    Server->>Client: DeviceAdded Id=0
    Client->>+Server: StopScanning Id=2
    Server->>-Client: Ok Id=2
    Server->>Client: ScanningFinished Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "StopScanning": {
      "Id": 1
    }
  }
]
```
---
## ScanningFinished

**Description:** Sent by the server once it has stopped scanning on
all busses. Since systems may have timeouts that are not controlled by
the server, this is a separate message from the StopScanning flow.
ScanningFinished can happen without a StopScanning call.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id

**Expected Response:**

None. Server-to-Client only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: StartScanning Id=1
    Server->>-Client: Ok Id=1
    Server->>Client: DeviceAdded Id=0
    Server->>Client: DeviceAdded Id=0
    Server->>Client: ScanningFinished Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "ScanningFinished": {
      "Id": 0
    }
  }
]
```
---
## RequestDeviceList

**Description:** Client request to have the server send over its known
device list, without starting a full scan.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id

**Expected Response:**

* DeviceList message with matching Id on successful request.
* Error message on value or message error.

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
    "RequestDeviceList": {
      "Id": 1
    }
  }
]
```
---
## DeviceList

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
            "SingleMotorVibrateCmd": {},
            "VibrateCmd": { "FeatureCount": 2 },
            "StopDeviceCmd": {}
          }
        },
        {
          "DeviceName": "TestDevice 2",
          "DeviceIndex": 1,
          "DeviceMessages": {
            "FleshlightLaunchFW12Cmd": {},
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
## DeviceAdded

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
        "SingleMotorVibrateCmd": {},
        "VibrateCmd": { "FeatureCount": 2 },
        "StopDeviceCmd": {}
      }
    }
  }
]
```

---
## Message Attributes for DeviceList and DeviceAdded

**Description:** A collection of message attributes. This object is
always the child of a Device Message type name within a
[DeviceList](enumeration.md#devicelist) or
[DeviceAdded](enumeration.md#deviceadded) message. Not all attributes
are relevant for all Device Messages on all Devices; in these cases
the attributes will not be included.

**Attributes:**

* _FeatureCount_ (unsigned int): Number of features the Device Message
  may address. This attribute is used to define the capabilities of
  generic device control messages. The meaning of "feature" is
  specific to the context of the message the attribute is attached to.
  For instance, the FeatureCount attribute of a VibrateCmd message
  will refer to the number of vibration motors that can be controlled
  on a device advertising the VibrateCmd message.

---
## DeviceRemoved

**Description:** Sent by the server whenever a device is removed from  
the system. Can happen at any time after identification.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned integer): Index used to identify the device
  when sending Device Messages.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    participant Client
    participant Server
    Server->>Client: DeviceRemoved Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "DeviceRemoved": {
      "Id": 0,
      "DeviceIndex": 0
    }
  }
]
```
