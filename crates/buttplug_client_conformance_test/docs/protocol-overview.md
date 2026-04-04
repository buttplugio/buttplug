# Buttplug Protocol v4 Overview

This document describes the Buttplug protocol v4 as implemented by the conformance test harness. It is the authoritative reference for JSON message format, field names, and protocol flow.

## Wire Format

All Buttplug messages are transmitted as **JSON arrays over WebSocket text frames**. Each frame contains one or more messages.

### Single Message Example

```json
[{"RequestServerInfo": {"Id": 1, "ClientName": "MyClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
```

### Multiple Messages in One Frame

```json
[
  {"Ping": {"Id": 2}},
  {"StartScanning": {"Id": 3}}
]
```

## Message ID Rules

- **0 = Server-initiated** (unsolicited messages like `DeviceRemoved`, `InputReading`)
- **Non-zero = Client request → Server response pair**
  - Client sends message with ID `N`
  - Server responds with same ID `N` to correlate request and response
- **IDs must be unique** for all in-flight requests waiting for responses
- **ID can be reused** after receiving the response

Example:
```json
← [{"RequestServerInfo": {"Id": 1, ...}}]       (client)
→ [{"ServerInfo": {"Id": 1, ...}}]              (server response)
← [{"StartScanning": {"Id": 2}}]                (client - new ID because previous is answered)
```

## Field Name Convention

All JSON field names use **PascalCase** (e.g., `ClientName`, `DeviceIndex`, `ProtocolVersionMajor`). This is enforced by serde serialization rules.

## Connection Lifecycle

### 1. Handshake

Client **MUST** send `RequestServerInfo` as the first message.

```json
← [{"RequestServerInfo": {"Id": 1, "ClientName": "MyClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
→ [{"ServerInfo": {"Id": 1, "ServerName": "Buttplug Server", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0, "MaxPingTime": 0}}]
```

### 2. Ping Requirement

If `MaxPingTime` > 0 (milliseconds):
- Client **MUST** send a `Ping` message within that interval
- Continue sending `Ping` messages while connected
- If client fails to ping in time, server **WILL** disconnect

If `MaxPingTime` == 0:
- Ping is optional (server has no keepalive requirement)

### 3. Device Discovery

```json
← [{"StartScanning": {"Id": 2}}]
→ [{"Ok": {"Id": 2}}]
→ [{"DeviceList": {"Id": 0, "Devices": {...}}}]  (unsolicited after scan completes)
→ [{"ScanningFinished": {"Id": 0}}]
```

### 4. Device Commands

After device list is received:

```json
← [{"OutputCmd": {"Id": 3, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 50}}}}]
→ [{"Ok": {"Id": 3}}]
```

### 5. Device Disconnection Notification

Server sends unsolicited messages when devices disconnect:

```json
→ [{"DeviceRemoved": {"Id": 0, "DeviceIndex": 0}}]
```

Note: `Id` is 0 because this is server-initiated.

### 6. Reconnection

After `ServerInfo`, client can:
- Request device list again: `RequestDeviceList` → `DeviceList`
- Start scanning again: `StartScanning` → (scan process)

## Message Types

### RequestServerInfo (Client → Server)

Initiates handshake. **MUST be the first message sent by client.**

```json
{
  "RequestServerInfo": {
    "Id": 1,
    "ClientName": "MyClient",
    "ProtocolVersionMajor": 4,
    "ProtocolVersionMinor": 0
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Message ID (1+ for client requests) |
| `ClientName` | string | Name of the client (for logging) |
| `ProtocolVersionMajor` | integer | Major protocol version (4) |
| `ProtocolVersionMinor` | integer | Minor protocol version (0) |

### ServerInfo (Server → Client)

Response to `RequestServerInfo`. Contains server capabilities.

```json
{
  "ServerInfo": {
    "Id": 1,
    "ServerName": "Buttplug Server",
    "ProtocolVersionMajor": 4,
    "ProtocolVersionMinor": 0,
    "MaxPingTime": 0
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Matches request ID |
| `ServerName` | string | Name of the server |
| `ProtocolVersionMajor` | integer | Major protocol version (4) |
| `ProtocolVersionMinor` | integer | Minor protocol version (0) |
| `MaxPingTime` | u32 | Max milliseconds between pings; 0 = no ping required |

### Ping (Client → Server)

Keepalive message. Send within `MaxPingTime` interval if server requests it.

```json
{
  "Ping": {
    "Id": 2
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Message ID |

### Ok (Server → Client)

Success response to a client command.

```json
{
  "Ok": {
    "Id": 2
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Matches request ID |

### Error (Server → Client)

Error response to a client command.

```json
{
  "Error": {
    "Id": 3,
    "ErrorCode": 1,
    "ErrorMessage": "Invalid device index"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Matches request ID (0 if server-initiated) |
| `ErrorCode` | u8 | Error class (0-4, see below) |
| `ErrorMessage` | string | Human-readable error description |

#### Error Codes

| Code | Name | Meaning |
|------|------|---------|
| 0 | `ErrorUnknown` | Unknown error |
| 1 | `ErrorHandshake` | Handshake error (invalid client name, version mismatch) |
| 2 | `ErrorPing` | Ping timeout |
| 3 | `ErrorMessage` | Invalid message format or semantics |
| 4 | `ErrorDevice` | Device error (invalid index, feature not supported) |

### StartScanning (Client → Server)

Begins device discovery.

```json
{
  "StartScanning": {
    "Id": 2
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Message ID |

### StopScanning (Client → Server)

Stops device discovery.

```json
{
  "StopScanning": {
    "Id": 3
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Message ID |

### RequestDeviceList (Client → Server)

Requests the current device list without scanning.

```json
{
  "RequestDeviceList": {
    "Id": 4
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Message ID |

### DeviceList (Server → Client)

List of connected devices. Sent after `StartScanning` completes or in response to `RequestDeviceList`.

```json
{
  "DeviceList": {
    "Id": 1,
    "Devices": {
      "0": {
        "DeviceIndex": 0,
        "DeviceName": "Conformance Test Vibrator",
        "DeviceDisplayName": null,
        "DeviceMessageTimingGap": 0,
        "DeviceFeatures": {
          "0": {
            "FeatureIndex": 0,
            "FeatureDescription": "Vibrator 1",
            "Output": {"Vibrate": [[0, 100]]}
          },
          "1": {
            "FeatureIndex": 1,
            "FeatureDescription": "Vibrator 2",
            "Output": {"Vibrate": [[0, 100]]}
          }
        }
      }
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Matches request ID (or 0 for unsolicited messages) |
| `Devices` | object | Map of device index → device info |

**Device Info Fields:**
- `DeviceIndex` (u32) — Unique device identifier for commands
- `DeviceName` (string) — Internal name (immutable)
- `DeviceDisplayName` (string or null) — User-facing name
- `DeviceMessageTimingGap` (u32) — Milliseconds to wait between commands (0 = no gap)
- `DeviceFeatures` (object) — Map of feature index → feature info

**Feature Info Fields:**
- `FeatureIndex` (u32) — Feature identifier within device
- `FeatureDescription` (string) — Human-readable feature name
- `Output` (object, optional) — Output capabilities with type-keyed properties
- `Input` (object, optional) — Input capabilities with type-keyed properties

Output/Input objects have type names as keys (e.g., `Vibrate`, `Battery`) with nested properties:
- Output types (e.g., `Vibrate`): `[[min, max]]` — range array for value
- HwPositionWithDuration: `{"Value": [[min, max]], "Duration": [[min, max]]}`
- Input types (e.g., `Battery`): `{"Value": [[min, max]], "Command": [list of commands]}` — ranges and supported commands

### ScanningFinished (Server → Client)

Indicates scanning is complete. Sent unsolicited after device discovery ends.

```json
{
  "ScanningFinished": {
    "Id": 0
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Always 0 (server-initiated) |

### StopCmd (Client → Server)

Stops all device commands.

```json
{
  "StopCmd": {
    "Id": 5,
    "DeviceIndex": 0
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Message ID |
| `DeviceIndex` | u32 | Device to stop |

### OutputCmd (Client → Server)

Sends a command to a device feature.

```json
{
  "OutputCmd": {
    "Id": 6,
    "DeviceIndex": 0,
    "FeatureIndex": 0,
    "Command": {"Vibrate": {"Value": 50}}
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Message ID |
| `DeviceIndex` | u32 | Target device |
| `FeatureIndex` | u32 | Target feature |
| `Command` | object | Command envelope (see below) |

#### Output Commands

All output commands wrap a single value or value+duration. The envelope name must match the feature type.

**Simple Value Commands:**
- `Vibrate` — Vibration speed (0-100)
- `Rotate` — Rotation speed (-100 to 100, negative = reverse)
- `Oscillate` — Oscillation amount (0-100)
- `Constrict` — Constriction amount (0-100)
- `Spray` — Spray amount (0-100)
- `Temperature` — Temperature (-100 to 100)
- `Led` — LED brightness (0-100)
- `Position` — Position (0-100)

Example:
```json
{"Vibrate": {"Value": 50}}
{"Rotate": {"Value": -25}}
{"Temperature": {"Value": 80}}
```

**Complex Commands:**
- `HwPositionWithDuration` — Position with movement duration

Example:
```json
{"HwPositionWithDuration": {"Value": 100, "Duration": 2000}}
```

### InputCmd (Client → Server)

Requests input sensor data (read or subscribe).

```json
{
  "InputCmd": {
    "Id": 7,
    "DeviceIndex": 0,
    "FeatureIndex": 2,
    "Type": "Battery",
    "Command": "Read"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Message ID |
| `DeviceIndex` | u32 | Target device |
| `FeatureIndex` | u32 | Target sensor feature |
| `Type` | string | Sensor type (`Battery`, `Rssi`, `Button`, `Pressure`) |
| `Command` | string | `Read` (one-shot) or `Subscribe` (stream) |

### InputReading (Server → Client)

Unsolicited sensor data. Sent in response to `InputCmd` with `Subscribe`, or as one-shot response to `Read`.

```json
{
  "InputReading": {
    "Id": 0,
    "DeviceIndex": 0,
    "FeatureIndex": 2,
    "Reading": {"Battery": {"Value": 75}}
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | 0 if unsolicited (subscription); matches request ID if response to Read |
| `DeviceIndex` | u32 | Source device |
| `FeatureIndex` | u32 | Source sensor |
| `Reading` | object | Sensor value wrapper |

#### Input Reading Types

- `Battery` — Battery level (0-100%)
- `Rssi` — Signal strength (-128 to 0 dBm)
- `Button` — Button state (0 or 1)
- `Pressure` — Pressure value (0-65535)

Examples:
```json
{"Battery": {"Value": 75}}
{"Rssi": {"Value": -45}}
{"Button": {"Value": 1}}
{"Pressure": {"Value": 32768}}
```

### DeviceRemoved (Server → Client)

Notification that a device has disconnected. Sent unsolicited.

```json
{
  "DeviceRemoved": {
    "Id": 0,
    "DeviceIndex": 0
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `Id` | u32 | Always 0 (server-initiated) |
| `DeviceIndex` | u32 | Device that was removed |

## Protocol State Machine

```
┌─────────────────────────────────────────────────────────────┐
│ CLIENT STARTS                                               │
└─────────────┬───────────────────────────────────────────────┘
              │
              ├─→ Send RequestServerInfo (MUST be first)
              │
┌─────────────▼───────────────────────────────────────────────┐
│ WAITING FOR ServerInfo                                      │
└─────────────┬───────────────────────────────────────────────┘
              │
              ├─→ Receive ServerInfo
              │
              ├─→ If MaxPingTime > 0: Start ping timer
              │
┌─────────────▼───────────────────────────────────────────────┐
│ CONNECTED (can send commands)                               │
├─────────────────────────────────────────────────────────────┤
│ • StartScanning → (scan loop) → DeviceList + ScanningFinished
│ • RequestDeviceList → DeviceList                            │
│ • OutputCmd → Ok or Error                                   │
│ • InputCmd → InputReading (on demand or subscription)        │
│ • StopCmd → Ok or Error                                     │
│ • Ping → Ok (if MaxPingTime requires it)                    │
│ • Receive DeviceRemoved (unsolicited, any time)             │
│ • Receive InputReading (unsolicited if subscribed)          │
└─────────────┬───────────────────────────────────────────────┘
              │
              ├─→ Server disconnects or error
              │   → Connection closes
              │
              ├─→ Or client can reconnect (re-handshake)
              │
└─────────────┴───────────────────────────────────────────────┘
```

## Common Patterns

### Device Command Flow

1. Receive `DeviceList` to get available features
2. For each feature, identify the output type (`Vibrate`, `Rotate`, etc.)
3. Send `OutputCmd` with matching command type
4. Receive `Ok` or `Error`

### Sensor Reading (One-Shot)

1. Send `InputCmd` with `Type` = sensor type, `Command` = `Read`
2. Receive `InputReading` with matching ID
3. Extract value from reading

### Sensor Reading (Subscription)

1. Send `InputCmd` with `Type` = sensor type, `Command` = `Subscribe`
2. Receive `Ok`
3. Receive multiple `InputReading` messages with `Id` = 0 (unsolicited)
4. To stop: Send `InputCmd` with `Command` = `Unsubscribe`

### Error Handling

All commands that fail receive an `Error` message with:
- `Id` matching the request
- `ErrorCode` indicating the class
- `ErrorMessage` with details

Always check responses for errors before assuming success.

## Example Conversation

```json
→ [{"RequestServerInfo": {"Id": 1, "ClientName": "TestClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
← [{"ServerInfo": {"Id": 1, "ServerName": "Test Server", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0, "MaxPingTime": 0}}]

→ [{"StartScanning": {"Id": 2}}]
← [{"Ok": {"Id": 2}}]
← [{"DeviceList": {"Id": 0, "Devices": {"0": {"DeviceIndex": 0, "DeviceName": "Device1", ...}}}}]
← [{"ScanningFinished": {"Id": 0}}]

→ [{"OutputCmd": {"Id": 3, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 75}}}}]
← [{"Ok": {"Id": 3}}]

→ [{"InputCmd": {"Id": 4, "DeviceIndex": 0, "FeatureIndex": 2, "Type": "Battery", "Command": "Read"}}]
← [{"InputReading": {"Id": 4, "DeviceIndex": 0, "FeatureIndex": 2, "Reading": {"Battery": {"Value": 90}}}}]
```

(Arrow direction: → = client sends, ← = server receives/sends)
