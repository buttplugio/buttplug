# Generic Sensor Messages

Messages for requesting and receiving information about generic
sensors on devices, including batteries, radio levels, accelerometers,
buttons, etc.

---
## BatteryLevelCmd

**Description:** Requests that a device send its battery level.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id

**Expected Response:**

* [BatteryLevelReading](sensors.html#batterylevelreading) message with
  matching Id on successful request.
* Error message on value or message error.

<mermaid>
sequenceDiagram
    Client->>Server: BatteryLevelCmd Id=1
    Server->>Client: BatteryLevelReading Id=1 BatteryLevel=0.5
</mermaid>

**Serialization Example:**

```json
[
  {
    "BatteryLevelCmd": {
      "Id": 1
    }
  }
]
```
---
## BatteryLevelReading

**Description:** Message containing a battery level reading from a
device, as requested by [BatteryLevelCmd](sensors.html#batterylevelcmd).

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _BatteryLevel_ (double): Battery Level with a range of [0.0-1.0]

**Expected Response:**

* BatteryLevelReading message with matching Id on successful request.
* Error message on value or message error.

<mermaid>
sequenceDiagram
    Client->>Server: BatteryLevelCmd Id=1
    Server->>Client: BatteryLevelReading Id=1 BatteryLevel=0.5
</mermaid>

**Serialization Example:**

```json
[
  {
    "BatteryLevelReading": {
      "Id": 1,
      "BatteryLevel": 0.5
    }
  }
]
```
---
## RSSILevelCmd

**Description:** Requests that a device send its RSSI level.

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id

**Expected Response:**

* [RSSILevelReading](sensors.html#rssilevelreading) message with
  matching Id on successful request.
* Error message on value or message error.

<mermaid>
sequenceDiagram
    Client->>Server: RSSILevelCmd Id=1
    Server->>Client: RSSILevelReading Id=1 RSSILevel=-40
</mermaid>

**Serialization Example:**

```json
[
  {
    "RSSILevelCmd": {
      "Id": 1
    }
  }
]
```
---
## RSSILevelReading

**Description:** Message containing a RSSI level reading from a
device, as requested by [RSSILevelCmd](sensors.html#rssilevelcmd).

**Introduced In Spec Version:** 2

**Last Updated In Spec Version:** 2

**Fields:**

* _Id_ (unsigned int): Message Id
* _RSSILevel_ (int): RSSI Level, usually expressed as db gain, usually [-100:0]

**Expected Response:**

* RSSILevelReading message with matching Id on successful request.
* Error message on value or message error.

<mermaid>
sequenceDiagram
    Client->>Server: RSSILevelCmd Id=1
    Server->>Client: RSSILevelReading Id=1 RSSILevel=-40
</mermaid>

**Serialization Example:**

```json
[
  {
    "RSSILevelReading": {
      "Id": 1,
      "RSSILevel": -40
    }
  }
]
```
