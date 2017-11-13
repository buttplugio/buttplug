# Generic Sensor Messages

A number of devices have sensors attached or can act as input devices, so we aim to support this functionality when it makes sense to do so (devices that advertise themselves as working games controllers do not need our help).

## StartAccelerometerCmd

**Description:** Client request to have the server subscribe and forward accelerometer data from the specified device.

**Introduced In Version:** 1

**Message Version:** 1

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device to subscribe to accelerometer data.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

**Serialization Example:**

```json
[
  {
    "StartAccelerometerCmd": {
      "Id": 1,
      "DeviceIndex": 0
    }
  }
]
```

## StopAccelerometerCmd

**Description:** Client request to have the server unsubscribe and stop forwarding accelerometer data from the specified device.

**Introduced In Version:** 1

**Message Version:** 1

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device to unsubscribe from accelerometer data.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

**Serialization Example:**

```json
[
  {
    "StartAccelerometerCmd": {
      "Id": 1,
      "DeviceIndex": 0
    }
  }
]
```

## AccelerometerData

**Description:** Message encapsulating accelerometer data from a device that the client has requested accelerometer data from.

**Introduced In Version:** 1

**Message Version:** 1

**Fields:**

* _Id_ \(unsigned int\): Message Id
* _DeviceIndex_ \(unsigned int\): Index of device sending data.
* _AccelerometerX_ \(int\): X-axis accelerometer reading.
* _AccelerometerY_ \(int\): Y-axis accelerometer reading.
* _AccelerometerZ_ \(int\): Z-axis accelerometer reading.

**Expected Response:**

None. Server-to-Client message only.

**Flow Diagram:**

**Serialization Example:**

```json
[
  {
    "AccelerometerData": {
      "Id": 1,
      "DeviceIndex": 0
      "AccelerometerX": 
      "AccelerometerY": 
      "AccelerometerZ": 
    }
  }
]
```
