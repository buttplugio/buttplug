# Enumeration Messages


## StartScanning

**Description:** Client request to have the server start scanning for
devices on all busses that it knows about. Useful for protocols like
Bluetooth, which require an explicit discovery phase.

**Fields:**

- *Id* (unsigned int): Message Id

**Expected Response:**

- Ok message with matching Id on successful request.
- Error message on value or message error.

**Flow Diagram:**

![img](startscanning_diagram.svg)

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


## StopScanning

**Description:** Client request to have the server stop scanning for
devices. Useful for protocols like Bluetooth, which may not timeout
otherwise.

**Fields:**

- *Id* (unsigned int): Message Id

**Expected Response:**

- Ok message with matching Id on successful request.
- Error message on value or message error.

**Flow Diagram:**

![img](stopscanning_diagram.svg)

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


## ScanningFinished

**Description:** Sent by the server once it has stopped scanning on
all busses. Since systems may have timeouts that are not controlled by
the server, this is a separate message from the StopScanning flow.
ScanningFinished can happen without a StopScanning call.

**Fields:**

- *Id* (unsigned int): Message Id

**Expected Response:**

None. Server-to-Client only.

**Flow Diagram:**

![img](scanningfinished_diagram.svg)

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


## RequestDeviceList

**Description:** Client request to have the server send over its known
device list, without starting a full scan.

**Fields:**

- *Id* (unsigned int): Message Id

**Expected Response:**

- DeviceList message with matching Id on successful request.
- Error message on value or message error.

**Flow Diagram:**

![img](requestdevicelist_diagram.svg)

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


## DeviceList

**Description:** Server reply to a client request for a device list.

**Fields:**

- *Id* (unsigned int): Message Id
- *Devices* (array): Array of device objects
    - *DeviceName* (string): Descriptive name of the device
    - *DeviceIndex* (unsigned integer): Index used to identify the
        device when sending Device Messages.
    - *DeviceMessages* (array of strings): Type names of Device
        Messages that the device will accept.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

![img](devicelist_diagram.svg)

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


## DeviceAdded

**Description:** Sent by the server whenever a device is added to the
system. Can happen at any time after identification, as it is assumed
many server implementations will support devices with hotplugging
capabilities that do not require specific scanning/discovery sessions.

**Fields:**

- *Id* (unsigned int): Message Id
- *DeviceName* (string): Descriptive name of the device
- *DeviceIndex* (unsigned integer): Index used to identify the device
  when sending Device Messages.
- *DeviceMessages* (array of strings): Type names of Device Messages
  that the device will accept.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

![img](deviceadded_diagram.svg)

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


## DeviceRemoved

**Description:** Sent by the server whenever a device is removed from
the system. Can happen at any time after identification.

**Fields:**

- *Id* (unsigned int): Message Id
- *DeviceIndex* (unsigned integer): Index used to identify the device
  when sending Device Messages.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

![img](deviceremoved_diagram.svg)

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
