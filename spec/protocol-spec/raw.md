# Raw Device Messages

Raw device messages do not state a specific command. Rather, they are
used to send raw Uint8 arrays to devices.

Raw messages are mostly meant for internal library use, development of
new protocols, or communication with systems that do not have
protocols built into a released Buttplug library.

**IMPORTANT:** Raw messages can be extremely dangerous, possibly
allowing applications to brick devices via access to
firmware/bootloader modes. User Interfaces for Buttplug Servers should
start with Raw Message capabilities disabled, and allow users to
change the visibility of Raw messages as needed. Applications
accessing the server should only be able to send Raw messages to
devices if the user has performed an opt-in gesture of some kind.

---
## Endpoints

Raw messages use the idea of "endpoints", a string representing a
communication point on a device, to specify where a message should go.
What an endpoint actually refers to depends on the device bus type:

* Serial: Usually just RX/TX (which is where we got the common
  endpoint names from, since most devices emulate serial regardless of
  their bus)
* USB: Endpoints (which is where we got the name from)
* Bluetooth Classic: Depends on the profile
* Bluetooth LE: Characteristics

The most common endpoint names will be `rx` and `tx`. For the Buttplug
reference implementation, endpoint names will be defined in the
[Buttplug Device Configuration
file](https://github.com/buttplugio/buttplug-device-config).

---
## RawDataWriteCmd

**Description:** Client request to have the server write a byte array
to a device endpoint.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to write to.
* _Endpoint_ (string): Name of endpoint to write data to.
* _Data_ (array of unsigned 8-bit int): Raw data to write to endpoint.
* _WriteWithResponse_ (boolean): True if BLE WriteWithResponse required, False otherwise.

**Expected Response:**

* Ok message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RawDataWriteCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "RawDataWriteCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Endpoint": "tx",
      "Data": [0x0, 0x1, 0x0],
      "WriteWithResponse": false
    }
  }
]
```
---
## RawDataReadCmd

**Description:** Client request to have the server read a byte array
from a device endpoint.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to read data from.
* _Endpoint_ (string): Name of endpoint to read data from.
* _ExpectedLength_ (unsigned int): Amount of data to read, 0 if "Read all currently available".
* _WaitForData_ (boolean): True if return should only be sent when there is data available, or until expected length is met.

**Expected Response:**

* RawDataReading message with matching Id on successful request.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RawDataReadCmd Id=1
    Server->>-Client: RawDataReading Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "RawDataReadCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Endpoint": "tx",
      "ExpectedLength": 0,
      "WaitForData": false
    }
  }
]
```

---
## RawDataReading

**Description:** Server response when data is read (in response to
RawDataReadCmd) or received (after RawDataSubscribe) from a device
endpoint.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id. Can be 0 in cases of subscription data.
* _DeviceIndex_ (unsigned int): Index of device to data was read from.
* _Endpoint_ (string): Name of endpoint to data was read from.
* _Data_ (array of unsigned 8-bit int): Raw data read from endpoint.

**Serialization Example:**

```json
[
  {
    "RawDataReading": {
      "Id": 1,
      "DeviceIndex": 0,
      "Endpoint": "rx",
      "Data": [0x0, 0x1, 0x0]
    }
  }
]
```

---
## RawDataSubscribeCmd

**Description:** Client request to have the server subscribe and send
all data that comes in from an endpoint that is not explicitly read.
Usually useful for Bluetooth notify endpoints, or other streaming data
endpoints.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to subscribe to.
* _Endpoint_ (string): Name of endpoint to subscribe to.

**Expected Response:**

* Ok if subscription is successful, followed by RawDataReading
  messages on all new readings.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RawDataSubscribeCmd Id=1
    Server->>-Client: Ok Id=1
    Server->>+Client: RawDataReading Id=0
    Server->>+Client: RawDataReading Id=0
</mermaid>

**Serialization Example:**

```json
[
  {
    "RawDataSubscribeCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Endpoint": "tx"
    }
  }
]
```

---
## RawDataUnsubscribeCmd

**Description:** Client request to have the server unsubscribe from an
endpoint to which it had previously subscribed.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _DeviceIndex_ (unsigned int): Index of device to subscribe to.
* _Endpoint_ (string): Name of endpoint to subscribe to.

**Expected Response:**

* Ok if unsubscription is successful.
* Error message on value or message error.

**Flow Diagram:**

<mermaid>
sequenceDiagram
    Client->>+Server: RawDataUnsubscribeCmd Id=1
    Server->>-Client: Ok Id=1
</mermaid>

**Serialization Example:**

```json
[
  {
    "RawDataUnsubscribeCmd": {
      "Id": 1,
      "DeviceIndex": 0,
      "Endpoint": "tx"
    }
  }
]
```

