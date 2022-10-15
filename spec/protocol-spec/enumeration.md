# Enumeration Messages

Messages relating to finding and getting information about devices connected to the system.

---
## StartScanning

**Description:** Client request to have the server start scanning for devices on all busses that it
knows about. Useful for protocols like Bluetooth, which require an explicit discovery phase.

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

**Description:** Client request to have the server stop scanning for devices. Useful for protocols
like Bluetooth, which may not timeout otherwise.

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

**Description:** Sent by the server once it has stopped scanning on all busses. Since systems may
have timeouts that are not controlled by the server, this is a separate message from the
StopScanning flow. ScanningFinished can happen without a StopScanning call.

In reality, this event is usually only useful when working with systems that can only scan for a single device at a time, like WebBluetooth. When on normal desktop/mobile APIs, it should be assumed that running StartScanning/StopScanning will be the main usage.

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

**Description:** Client request to have the server send over its known device list, without starting
a full scan.

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

**Last Updated In Spec Version:** 3 (See [Deprecated Messages](deprecated.md) for older versions.)

**Fields:**

* _Id_ (unsigned int): Message Id
* _Devices_ (array): Array of device objects
  * _DeviceName_ (string): Descriptive name of the device, as taken from the base device
    configuration file.
  * _DeviceIndex_ (unsigned integer): Index used to identify the device when sending Device
    Messages.
  * _DeviceMessageGap_ (_optional_, unsigned integer): Recommended minimum gap between device
    commands, in milliseconds. This is only a recommendation, and will not be enforced on the
    server, as most times the server does not actually know the exact message gap timing required
    (hence this being recommended). Enforcement on the client (with developer option to disable) is
    encouraged. Optional field, not required to be included in message. Missing value should be assumed that server does not know recommended message gap.
  * _DeviceDisplayName_ (_optional_, string): User provided display name for a device. Useful for
    cases where a users may have multiple of the same device connected. Optional field, not required
    to be included in message. Missing value means that no device display name is set, and device
    name should be used.
  * _DeviceMessages_ (dictionary): Accepted Device Messages 
    * Keys (string): Type names of Device Messages that the device will accept
    * Values (Array of [Message
      Attributes](enumeration.md#message-attributes-for-devicelist-and-deviceadded)):
      Attributes for the Device Messages. Each feature is a seperate array element, and its index in the array matches how it should be addressed in generic command messages. For instance, in the example below, the Clitoral Stimulator would be Actuator Index 0 in ScalarCmd.

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
          "DeviceName": "Test Vibrator",
          "DeviceIndex": 0,
          "DeviceMessages": {
            "ScalarCmd": [
              {
                "StepCount": 20,
                "FeatureDescriptor": "Clitoral Stimulator",
                "ActuatorType": "Vibrate"
              },
              {
                "StepCount": 20,
                "FeatureDescriptor": "Insertable Vibrator",
                "ActuatorType": "Vibrate"
              }
            ],
            "StopDeviceCmd": {}
          }
        },
        {
          "DeviceName": "Test Stroker",
          "DeviceIndex": 1,
          "DeviceMessageGap": 100,
          "DeviceDisplayName": "User set name",
          "DeviceMessages": {
            "LinearCmd": [ {
              "StepCount": 100,
              "FeatureDescriptor": "Stroker"
            } ],
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

**Description:** Sent by the server whenever a device is added to the system. Can happen at any time
after identification stage (i.e. after client is connected), as it is assumed many server
implementations will support devices with hotplugging capabilities that do not require specific
scanning/discovery sessions.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version**: 3 (See [Deprecated Messages](deprecated.md) for older versions.)

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceName_ (string): Descriptive name of the device, as taken from the base device
  configuration file.
* _DeviceIndex_ (unsigned integer): Index used to identify the device when sending Device Messages.
* _DeviceMessageGap_ (_optional_, unsigned integer): Recommended minimum gap between device
  commands, in milliseconds. This is only a recommendation, and will not be enforced on the
  server, as most times the server does not actually know the exact message gap timing required
  (hence this being recommended). Enforcement on the client (with developer option to disable) is
  encouraged. Optional field, not required to be included in message. Missing value should be assumed that server does not know recommended message gap.
* _DeviceDisplayName_ (_optional_, string): User provided display name for a device. Useful for
  cases where a users may have multiple of the same device connected. Optional field, not required
  to be included in message. Missing value means that no device display name is set, and device
  name should be used.
* _DeviceMessages_ (dictionary): Accepted Device Messages 
  * Keys (string): Type names of Device Messages that the device will accept
  * Values (Array of [Message
    Attributes](enumeration.md#message-attributes-for-devicelist-and-deviceadded)): Attributes for
    the Device Messages. Each feature is a seperate array element, and its index in the array matches how it should be addressed in generic command messages. For instance, in the example below, the Clitoral Stimulator would be Actuator Index 0 in ScalarCmd.

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
      "DeviceName": "Test Vibrator",
      "DeviceIndex": 0,
      "DeviceMessageGap": 100,
      "DeviceDisplayName": "Rabbit Vibrator",
      "DeviceMessages": {
        "ScalarCmd": [
          {
            "StepCount": 20,
            "FeatureDescriptor": "Clitoral Stimulator",
            "ActuatorType": "Vibrate"
          },
          {
            "StepCount": 20,
            "FeatureDescriptor": "Insertable Vibrator",
            "ActuatorType": "Vibrate"
          }
        ],
        "StopDeviceCmd": {}
       }
    }
  }
]
```

---
## Message Attributes for DeviceList and DeviceAdded

**Description:** A collection of message attributes. This object is always an array element of a
Device Message key/value pair within a [DeviceList](enumeration.md#devicelist) or
[DeviceAdded](enumeration.md#deviceadded) message. Not all attributes are relevant for all Device
Messages on all Devices; in these cases the attributes will not be included.

**Introduced In Spec Version:** 1

**Last Updated In Spec Version**: 3 (See [Deprecated Messages](deprecated.md) for older versions.)

**Attributes:**

* _FeatureDescriptor_
  * Valid for Messages: ScalarCmd, RotateCmd, LinearCmd, SensorReadCmd
  * Type: String
  * Description: Text descriptor for a feature.
* _StepCount_ 
  * Valid for Messages: ScalarCmd, RotateCmd, LinearCmd
  * Type: unsigned int
  * Description: For each feature, lists the number of discrete steps the feature can use. This
    value can be used in calculating the 0.0-1.0 range required for ScalarCmd and other messages.
* _ActuatorType_
  * Valid for Messages: ScalarCmd
  * Type: String
  * Description: Type of actuator this feature represents.
* _SensorType_
  * Valid for Messages: SensorReadCmd
  * Type: String
  * Description: Sensor types that can be read by Sensor.
* _SensorRange_
  * Valid for Messages: SensorReadCmd (but applies to values returned by SensorReading)
  * Type: array of arrays of 2 integers
  * Description: Range of values a sensor can return. As sensors can possibly return multiple values
    in the same SensorReading message (i.e. an 3-axis accelerometer may return all 3 axes in one read), this is sent as an array of ranges. The length of this array will always match the number of readings that will be returned from a sensor, and can be used to find the reading count for a sensor.
* _Endpoints_
  * Valid for Messages: RawReadCmd, RawWriteCmd, RawSubscribeCmd
  * Type: array of strings
  * Description: Endpoints that can be used by Raw commands.

---
## DeviceRemoved

**Description:** Sent by the server whenever a device is removed from the system. Can happen at any
time after identification.

**Introduced In Spec Version:** 0

**Last Updated In Spec Version:** 0

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned integer): Index used to identify the device when sending Device Messages.

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
