# Test Sequences Guide

This document provides a step-by-step breakdown of every test sequence in the conformance test harness. For each step, it shows exactly what your client should send, what the server will respond, and what criteria determine pass/fail.

All messages are wrapped in JSON arrays and sent over WebSocket text frames. Field names use PascalCase. Device indices for conformance devices are 0, 1, and 2.

---

## 1. Core Protocol Sequence

**Sequence ID:** `core_protocol`

**Description:** Full protocol exercise without ping — handshake, enumeration, all output/input commands, stop, device removal

**Max Ping Time:** 0 (no ping required)

### Step 1: Handshake

**Client sends:**
```json
[{"RequestServerInfo": {"Id": 1, "ClientName": "MyClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
```

**Server responds:**
```json
[{"ServerInfo": {"Id": 1, "ServerName": "conformance-runner", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0, "MaxPingTime": 0}}]
```

**Pass criteria:** Server responds with ServerInfo with Id=1, MaxPingTime=0.

---

### Step 2: Start Scanning

**Client sends:**
```json
[{"StartScanning": {"Id": 2}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 2}}, {"DeviceList": {"Id": 0, "Devices": {"0": {"DeviceIndex": 0, "DeviceName": "Conformance Test Vibrator", "DeviceFeatures": {...}}, "1": {"DeviceIndex": 1, "DeviceName": "Conformance Test Positioner", "DeviceFeatures": {...}}, "2": {"DeviceIndex": 2, "DeviceName": "Conformance Test Multi", "DeviceFeatures": {...}}}}}]
```

**Pass criteria:** Ok response with Id=2. DeviceList contains exactly 3 devices (indices 0, 1, 2).

---

### Step 3: Verify Devices Received

**Client action:** Process the DeviceList and identify the 3 devices. No message sent.

**Pass criteria:** Internal server state shows 3 connected devices.

---

### Step 4: Request Device List

**Client sends (optional explicit request):**
```json
[{"RequestDeviceList": {"Id": 3}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 3}}]
```

**Pass criteria:** Server responds with Ok.

---

### Step 5: Vibrate Command (Device 0, Feature 0)

**Client sends:**
```json
[{"OutputCmd": {"Id": 4, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 4}}]
```

**Pass criteria:** Ok response (Id=4). Device 0 receives command on Tx endpoint with feature index 0.

---

### Step 6: Vibrate Command (Device 0, Feature 1)

**Client sends:**
```json
[{"OutputCmd": {"Id": 5, "DeviceIndex": 0, "FeatureIndex": 1, "Command": {"Vibrate": {"Value": 75}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 5}}]
```

**Pass criteria:** Ok response (Id=5). Device 0 receives command on Tx endpoint with feature index 1.

---

### Step 7: Rotate Command (Device 0, Feature 2)

**Client sends:**
```json
[{"OutputCmd": {"Id": 6, "DeviceIndex": 0, "FeatureIndex": 2, "Command": {"Rotate": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 6}}]
```

**Pass criteria:** Ok response (Id=6). Device 0 receives command on Tx endpoint with feature index 2.

---

### Step 8: Oscillate Command (Device 1, Feature 2)

**Client sends:**
```json
[{"OutputCmd": {"Id": 7, "DeviceIndex": 1, "FeatureIndex": 2, "Command": {"Oscillate": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 7}}]
```

**Pass criteria:** Ok response (Id=7). Device 1 receives command on Tx endpoint with feature index 2.

---

### Step 9: Position Command (Device 1, Feature 0)

**Client sends:**
```json
[{"OutputCmd": {"Id": 8, "DeviceIndex": 1, "FeatureIndex": 0, "Command": {"Position": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 8}}]
```

**Pass criteria:** Ok response (Id=8). Device 1 receives command on Tx endpoint with feature index 0.

---

### Step 10: HwPositionWithDuration Command (Device 1, Feature 1)

**Client sends:**
```json
[{"OutputCmd": {"Id": 9, "DeviceIndex": 1, "FeatureIndex": 1, "Command": {"HwPositionWithDuration": {"Value": 500, "Duration": 1000}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 9}}]
```

**Pass criteria:** Ok response (Id=9). Device 1 receives command on Tx endpoint with feature index 1.

---

### Step 11: Constrict Command (Device 2, Feature 0)

**Client sends:**
```json
[{"OutputCmd": {"Id": 10, "DeviceIndex": 2, "FeatureIndex": 0, "Command": {"Constrict": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 10}}]
```

**Pass criteria:** Ok response (Id=10). Device 2 receives command on Tx endpoint with feature index 0.

---

### Step 12: Spray Command (Device 2, Feature 1)

**Client sends:**
```json
[{"OutputCmd": {"Id": 11, "DeviceIndex": 2, "FeatureIndex": 1, "Command": {"Spray": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 11}}]
```

**Pass criteria:** Ok response (Id=11). Device 2 receives command on Tx endpoint with feature index 1.

---

### Step 13: Temperature Command (Device 2, Feature 2)

**Client sends:**
```json
[{"OutputCmd": {"Id": 12, "DeviceIndex": 2, "FeatureIndex": 2, "Command": {"Temperature": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 12}}]
```

**Pass criteria:** Ok response (Id=12). Device 2 receives command on Tx endpoint with feature index 2.

---

### Step 14: Led Command (Device 2, Feature 3)

**Client sends:**
```json
[{"OutputCmd": {"Id": 13, "DeviceIndex": 2, "FeatureIndex": 3, "Command": {"Led": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 13}}]
```

**Pass criteria:** Ok response (Id=13). Device 2 receives command on Tx endpoint with feature index 3.

---

### Step 15: Battery Read (Device 0)

**Client sends (if implemented input reading):**
```json
[{"InputCmd": {"Id": 14, "DeviceIndex": 0, "FeatureIndex": 3, "Type": "Battery", "Command": "Read"}}]
```

**Server may push (unsolicited):**
```json
[{"InputReading": {"Id": 0, "DeviceIndex": 0, "FeatureIndex": 3, "Reading": {"Battery": {"Value": 85}}}}]
```

**Pass criteria:** Client receives InputReading for battery value (85 = 85%). Device 0 is still connected.

---

### Step 16: Sensor Subscribe (Device 2, Pressure)

**Client sends:**
```json
[{"InputCmd": {"Id": 15, "DeviceIndex": 2, "FeatureIndex": 5, "Type": "Pressure", "Command": "Subscribe"}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 15}}]
```

**Pass criteria:** Ok response (Id=15).

---

### Step 17: Sensor Notification (Device 2, Pressure)

**Server pushes (unsolicited):**
```json
[{"InputReading": {"Id": 0, "DeviceIndex": 2, "FeatureIndex": 5, "Reading": {"Pressure": {"Value": 32768}}}}]
```

**Pass criteria:** Client receives InputReading. Subscription is active.

---

### Step 18: Sensor Unsubscribe (Device 2, Pressure)

**Client sends:**
```json
[{"InputCmd": {"Id": 16, "DeviceIndex": 2, "FeatureIndex": 5, "Type": "Pressure", "Command": "Unsubscribe"}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 16}}]
```

**Pass criteria:** Ok response (Id=16). Device 2 remains connected.

---

### Step 19: Stop Single Device (Device 0)

**Client sends:**
```json
[{"StopCmd": {"Id": 17, "DeviceIndex": 0}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 17}}]
```

**Pass criteria:** Ok response (Id=17). Device 0 receives stop command.

---

### Step 20: Stop All Devices

**Client sends:**
```json
[{"StopCmd": {"Id": 18}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 18}}]
```

**Pass criteria:** Ok response (Id=18). All devices remain connected.

---

### Step 21: Device Removal (Device 1)

**Server pushes (unsolicited):**
```json
[{"DeviceRemoved": {"Id": 0, "DeviceIndex": 1}}]
```

**Pass criteria:** Client receives DeviceRemoved for device 1. Client should remove device 1 from its device list.

---

## 2. Ping Required Sequence

**Sequence ID:** `ping_required`

**Description:** Validates client sends periodic Ping when server advertises max_ping_time > 0

**Max Ping Time:** 1000 milliseconds

### Step 1: Handshake with Ping

**Client sends:**
```json
[{"RequestServerInfo": {"Id": 1, "ClientName": "MyClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
```

**Server responds:**
```json
[{"ServerInfo": {"Id": 1, "ServerName": "conformance-runner", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0, "MaxPingTime": 1000}}]
```

**Pass criteria:** MaxPingTime is 1000. Client must send Ping messages at least once per 1000ms to keep connection alive.

---

### Step 2: First Ping Received

**Client must send (within 1000ms of handshake):**
```json
[{"Ping": {"Id": 2}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 2}}]
```

**Pass criteria:** Client sends Ping within 1000ms. Server responds with Ok. Connection remains active.

---

### Step 3: Second Ping Received

**Client must send (within 1000ms of previous Ping):**
```json
[{"Ping": {"Id": 3}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 3}}]
```

**Pass criteria:** Client sends another Ping. Server stays connected.

---

### Step 4: Ping with Device Operations

**Server triggers scanning and prepares devices.**

**Client must send Ping (within 1000ms):**
```json
[{"Ping": {"Id": 4}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 4}}]
```

**Pass criteria:** Ping continues even while operating on other commands. All 3 devices are available.

---

## 3. Error Handling Sequence

**Sequence ID:** `error_handling`

**Description:** Validates client handles error responses and continues operating

**Max Ping Time:** 0 (no ping required)

### Step 1: Handshake

**Client sends:**
```json
[{"RequestServerInfo": {"Id": 1, "ClientName": "MyClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
```

**Server responds:**
```json
[{"ServerInfo": {"Id": 1, "ServerName": "conformance-runner", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0, "MaxPingTime": 0}}]
```

**Pass criteria:** Standard handshake.

---

### Step 2: Scan and Enumerate

**Client sends:**
```json
[{"StartScanning": {"Id": 2}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 2}}, {"DeviceList": {"Id": 0, "Devices": [...]}}]
```

**Pass criteria:** DeviceList received with 3 devices.

---

### Step 3: Invalid Device Index

**Client sends:**
```json
[{"OutputCmd": {"Id": 3, "DeviceIndex": 99, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 50}}}}]
```

**Server responds with error:**
```json
[{"Error": {"Id": 3, "ErrorCode": 4, "ErrorMessage": "Device index 99 not found"}}]
```

**Pass criteria:** Server responds with Error (not Ok). Connection remains active (server_connected = true).

---

### Step 4: Valid Command After Error

**Client sends:**
```json
[{"OutputCmd": {"Id": 4, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 4}}]
```

**Pass criteria:** Ok response (Id=4). Connection is still good after previous error. Device 0 receives command.

---

### Step 5: Invalid Feature Index

**Client sends:**
```json
[{"OutputCmd": {"Id": 5, "DeviceIndex": 0, "FeatureIndex": 99, "Command": {"Vibrate": {"Value": 50}}}}]
```

**Server responds with error:**
```json
[{"Error": {"Id": 5, "ErrorCode": 4, "ErrorMessage": "Feature index 99 not found"}}]
```

**Pass criteria:** Server returns Error. Connection stays active.

---

### Step 6: Valid Command After Second Error

**Client sends:**
```json
[{"OutputCmd": {"Id": 6, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Rotate": {"Value": 75}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 6}}]
```

**Pass criteria:** Ok response (Id=6). Client recovers and continues operating normally.

---

## 4. Ping Timeout Sequence

**Sequence ID:** `ping_timeout`

**Description:** Validates server disconnects client that fails to send Ping within max_ping_time

**Max Ping Time:** 500 milliseconds

**Important:** In this sequence, your client should intentionally NOT send any Ping messages. This tests the timeout behavior.

### Step 1: Handshake with Short Ping

**Client sends:**
```json
[{"RequestServerInfo": {"Id": 1, "ClientName": "MyClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
```

**Server responds:**
```json
[{"ServerInfo": {"Id": 1, "ServerName": "conformance-runner", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0, "MaxPingTime": 500}}]
```

**Pass criteria:** MaxPingTime is 500. Client acknowledges the requirement.

---

### Step 2: Wait for Ping Timeout

**Client action:** Do NOT send any Ping messages. Wait for server to close the connection.

**Server closes connection after ~500ms:**
Connection is dropped (WebSocket closes).

**Pass criteria:** Connection closes due to ping timeout. Client should detect the disconnection.

---

### Step 3: Verify Disconnected State

**Client action:** Confirm connection is closed.

**Pass criteria:** Connection is in disconnected state.

---

## 5. Reconnection Sequence

**Sequence ID:** `reconnection`

**Description:** Validates client reconnects cleanly after server disconnect — fresh handshake, new device enumeration

**Max Ping Time:** 0 (no ping required)

### Step 1: First Connection Handshake

**Client sends:**
```json
[{"RequestServerInfo": {"Id": 1, "ClientName": "MyClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
```

**Server responds:**
```json
[{"ServerInfo": {"Id": 1, "ServerName": "conformance-runner", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0, "MaxPingTime": 0}}]
```

**Pass criteria:** Normal handshake on first connection.

---

### Step 2: First Connection Scan

**Client sends:**
```json
[{"StartScanning": {"Id": 2}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 2}}, {"DeviceList": {"Id": 0, "Devices": [...]}}]
```

**Pass criteria:** DeviceList received with 3 devices.

---

### Step 3: Server Closes Connection

**Server closes the WebSocket connection.**

**Client action:** Detect the closure and prepare to reconnect.

**Pass criteria:** Connection drops. Client should clean up and close any resources.

---

### Step 4: Rebuild Server for Reconnection

**Server action (internal):** Tear down and rebuild on the same port.

**Client action:** Wait for the server to become available again, then reconnect.

**Pass criteria:** Server is ready to accept new connections on the same port.

---

### Step 5: Reconnection Handshake

**Client reconnects and sends:**
```json
[{"RequestServerInfo": {"Id": 1, "ClientName": "MyClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
```

**Server responds:**
```json
[{"ServerInfo": {"Id": 1, "ServerName": "conformance-runner", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0, "MaxPingTime": 0}}]
```

**Pass criteria:** Fresh handshake succeeds. This is a new connection, not a resume.

---

### Step 6: Reconnection Scan

**Client sends:**
```json
[{"StartScanning": {"Id": 2}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 2}}, {"DeviceList": {"Id": 0, "Devices": [...]}}]
```

**Pass criteria:** DeviceList received with all 3 devices again. Enumeration is fresh.

---

### Step 7: Reconnection Device Command

**Client sends:**
```json
[{"OutputCmd": {"Id": 3, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 50}}}}]
```

**Server responds:**
```json
[{"Ok": {"Id": 3}}]
```

**Pass criteria:** Commands work on the reconnected session. Device 0 receives the command.

---

## Summary

Each sequence is designed to test a specific aspect of the Buttplug protocol:

- **core_protocol** — All basic operations (handshake, enumeration, commands, inputs, stop, removal)
- **ping_required** — Keep-alive ping behavior
- **error_handling** — Client resilience to errors
- **ping_timeout** — Timeout enforcement
- **reconnection** — Clean disconnect and reconnect flow

Use these step-by-step guides to verify your client implementation. If a step fails, check the server response against the documented JSON and ensure your client handles it correctly.
