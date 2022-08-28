# Generic Sensor Messages

Messages for requesting and receiving information about generic
sensors on devices, including batteries, radio levels, accelerometers,
buttons, etc.

---
## SensorReadCmd

**Description:** Client request to have a device return the current value for a sensor

**Introduced In Spec Version:** 3

**Last Updated In Spec Version:** 3

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to read data from.
* _SensorIndex_ (unsigned int): Index of sensor on device to read data from (index relates to
  position of sensor in SensorReadCmd message attributes).
* _SensorType_ (string): Type of sensor, used as confirmation of context, must match value in
  SensorReadCmd message attributes.

**Expected Response:**

* SensorReading message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: SensorReadCmd Id=1
    Server->>-Client: SensorReading Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "SensorReadCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "SensorIndex": "0",
      "SensorType": "Pressure"
    }
  }
]
```

---
## SensorReading

**Description:** Server response when data is read (in response to SensorReadCmd) or received (after
SensorSubscribe) from a device sensor.

**Introduced In Spec Version:** 3

**Last Updated In Spec Version:** 3

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to read data from.
* _SensorIndex_ (unsigned int): Index of sensor on device that data was read from (index relates to
  position of sensor in SensorReadCmd message attributes).
* _SensorType_ (string): Type of sensor.
* _Data_ (array of signed int): Array of signed integers representing data. Signed integers are used
  due to varying return values (for instance, RSSI is negative, battery is [0, 100], buttons are [0,
  1], etc...). Information on formatting/units of measurement/etc may be included in feature
  descriptors.

**Serialization Example:**

```json
[
  {
    "SensorReading": {
      "Id": 1,
      "DeviceIndex": 0,
      "SensorIndex": 0,
      "SensorType": "Pressure",
      "Data": [591]
    }
  }
]
```

---
## SensorSubscribeCmd

**Description:** Client request to have the server subscribe and send all data that comes in from a
device sensor that is not explicitly read. Usually useful for Bluetooth notify endpoints, or other
streaming data endpoints.

**Introduced In Spec Version:** 3

**Last Updated In Spec Version:** 3

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to read data from.
* _SensorIndex_ (unsigned int): Index of sensor on device to read data from (index relates to
  position of sensor in SensorReadCmd message attributes).
* _SensorType_ (string): Type of sensor, used as confirmation of context, must match value in
  SensorReadCmd message attributes.

**Expected Response:**

* Ok if subscription is successful, followed by SensorReading messages on all new readings.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: SensorSubscribeCmd Id=1
    Server->>-Client: Ok Id=1
    Server->>+Client: SensorReading Id=0
    Server->>+Client: SensorReading Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "SensorSubscribeCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "SensorIndex": 0,
      "SensorType": "Pressure"
    }
  }
]
```

---
## SensorUnsubscribeCmd

**Description:** Client request to have the server unsubscribe from a device sensor to which it had
previously subscribed.

**Introduced In Spec Version:** 3

**Last Updated In Spec Version:** 3

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to read data from.
* _SensorIndex_ (unsigned int): Index of sensor on device to read data from (index relates to
  position of sensor in SensorReadCmd message attributes).
* _SensorType_ (string): Type of sensor, used as confirmation of context, must match value in
  SensorReadCmd message attributes.

**Expected Response:**

* Ok if unsubscription is successful.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: SensorUnsubscribeCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "SensorUnsubscribeCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "SensorIndex": 0,
      "SensorType": "Pressure"
    }
  }
]
```

