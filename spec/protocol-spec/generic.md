# Generic Device Messages

Generic device messages pertain to classes of devices, versus specific devices. For instance, the generic VibrateCmd should be supported by all vibrating devices, and StopDeviceCmd should be supported by all devices in order to stop them from whatever their current action may be.

---
## StopDeviceCmd

**Description:** Client request to have the server stop a device from whatever actions it may be taking. This message should be supported by all devices, and the server should know how to stop any device it supports.

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device to stop.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: StopDeviceCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "StopDeviceCmd": {
      "Id": 1,
      "DeviceIndex": 0
    }
  }
]
```
---
## StopAllDevices

**Description:** Sent by the client to tell the server to stop all devices. Can be used for emergency situations, on client shutdown for cleanup, etc… While this is considered a Device Message, since it pertains to all currently connected devices, it does not specify a device index \(and does not end with 'Cmd'\).

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: StopAllDevices Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "StopAllDevices": {
      "Id": 1
    }
  }
]
```
---
## RawCmd

**Description:** Used to send a raw byte string to a device. Should only be used for development, and should not be exposed to untrusted clients.

**Introduced In Version:** 0

**Message Version:** 0

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device
* _Command_ \(Array of bytes\): Command to send, array of ints with a range of \[0-255\]. Minimum length is 1.

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
## VibrateCmd

**Description:** Causes a device that supports vibration to run specific vibration motors at a certain speeds. Devices with multiple vibrator features may take multiple values. The [FeatureCount](enumeration.md#messageattributes) attribute for the message in the [DeviceList](enumeration.md#devicelist)/[DeviceAdded](enumeration.md#deviceadded) message will contain that information.

**Introduced In Version:** 1

**Message Version:** 1

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device
* _Speeds_ \(array\): Vibration speeds
  * _Index_ \(unsigned int\): Index of vibration motor
  * _Speed_ \(double\): Vibration speed with a range of \[0.0-1.0\]

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
## LinearCmd

**Description:** Causes a device that supports linear movement to move to a position over a certain amount of time. Devices with multiple linear actuator features may take multiple values. The [FeatureCount](enumeration.md#messageattributes) attribute for the message in the [DeviceList](enumeration.md#devicelist)/[DeviceAdded](enumeration.md#deviceadded) message will contain that information.

**Introduced In Version:** 1

**Message Version:** 1

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device
* _Vectors_ \(array\): Linear actuator speeds and positions
  * _Index_ \(unsigned int\): Index of linear actuator
  * _Duration_ \(unsigned int\): Movement time in milliseconds
  * _Position_ \(double\): Target position with a range of \[0.0-1.0\]

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: LinearCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "LinearCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Vectors": [
        {
          "Index": 0,
          "Duration": 500,
          "Position": 0.3
        },
        {
          "Index": 1,
          "Duration": 1000,
          "Position": 0.8
        }
      ]
    }
  }
]
```
---
## RotateCmd

**Description:** Causes a device that supports rotation to rotate at a certain speeds in specified directions. Devices with multiple rotating features may have multiple values. The [FeatureCount](enumeration.md#messageattributes) attribute for the message in the [DeviceList](enumeration.md#devicelist)/[DeviceAdded](enumeration.md#deviceadded) message will have this information.

**Introduced In Version:** 1

**Message Version:** 1

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device
* _Rotations_ \(array\): Rotation speeds
  * _Index_ \(unsigned int\): Index of rotation motor
  * _Speed_ \(double\): Rotation speed with a range of \[0.0-1.0\]
  * _Clockwise_ \(boolean\): Direction of rotation \(clockwise may be subjective\)

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RotateCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "RotateCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Rotations": [
        {
          "Index": 0,
          "Speed": 0.5,
          "Clockwise": true
        },
        {
          "Index": 1,
          "Speed": 1.0,
          "Clockwise": false
        }
      ]
    }
  }
]
```

