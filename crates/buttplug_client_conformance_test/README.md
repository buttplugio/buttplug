# Buttplug Client Conformance Test Harness

Validates that Buttplug client implementations correctly speak the Buttplug v4 protocol. The harness runs a real Buttplug server with simulated hardware behind a WebSocket endpoint — connect your client, run the sequences, fix what fails.

Language-agnostic: anything that can open a WebSocket and send JSON can use this.

## Building

```bash
cargo build -p buttplug_client_conformance_test --release
```

Binary lands at `./target/release/buttplug-client-conformance-test`.

## Running

```bash
# Run all sequences (harness waits for a connection before each one)
./target/release/buttplug-client-conformance-test

# Single sequence
./target/release/buttplug-client-conformance-test --sequence core_protocol

# Client-driven mode (for real client libraries — see below)
./target/release/buttplug-client-conformance-test --client-driven

# Custom port, longer timeout, JSON output for CI
./target/release/buttplug-client-conformance-test --port 20000 --timeout 10000 --format json
```

Exit code 0 = all pass, 1 = any failure.

## Implementing Conformance Testing in Your Client

### Overview

Your client connects to the harness over WebSocket and exercises the Buttplug v4 protocol. The harness validates that your client sends correct messages, handles responses properly, and respects timing requirements.

There are two modes:

| Mode | Flag | Use when... |
|------|------|-------------|
| Harness-driven | (default) | Building a thin client that reacts to server messages |
| Client-driven | `--client-driven` | Testing a real client library that initiates its own handshake and scanning |

**Most client library authors want `--client-driven` mode.**

### Client-Driven Mode

In client-driven mode, your client is in control. The harness:
- Does NOT inject protocol messages on your behalf
- Polls for expected state (device commands received, connections established) with 50ms intervals
- Waits up to `--timeout` ms for each step to pass

Your client must:
1. Connect to the WebSocket endpoint
2. Send `RequestServerInfo` to initiate the handshake
3. Call `StartScanning` to discover devices
4. Send device commands as specified by each sequence
5. Handle unsolicited messages (DeviceRemoved, InputReading, etc.)

### What Your Client Needs to Implement

To pass all five sequences, your client must support:

| Capability | Tested In |
|-----------|-----------|
| WebSocket connection | All sequences |
| Handshake (RequestServerInfo / ServerInfo) | All sequences |
| Device scanning (StartScanning / DeviceList) | core_protocol, error_handling, reconnection |
| Output commands (OutputCmd with all output types) | core_protocol |
| Input commands (InputCmd Read / Subscribe / Unsubscribe) | core_protocol |
| Unsolicited message handling (InputReading, DeviceRemoved) | core_protocol |
| Stop commands (StopCmd per-device and all-device) | core_protocol |
| Ping keepalive (when MaxPingTime > 0) | ping_required |
| Error response handling (Error messages) | error_handling |
| Ping timeout detection (server disconnects) | ping_timeout |
| Reconnection after server drop | reconnection |

### Test Sequences

#### 1. `core_protocol`

Full protocol exercise. MaxPingTime=0 (no keepalive needed).

Flow: Handshake → StartScanning → receive DeviceList (3 devices) → send OutputCmd for each output type (Vibrate, Rotate, Oscillate, Position, HwPositionWithDuration, Constrict, Spray, Temperature, Led) → InputCmd Read (Battery) → InputCmd Subscribe/Unsubscribe (Pressure) → StopCmd → receive DeviceRemoved.

#### 2. `ping_required`

MaxPingTime=1000ms. Your client must send `Ping` messages at regular intervals (at least once per second). The harness verifies pings arrive on time while device operations proceed normally.

#### 3. `error_handling`

MaxPingTime=0. The harness sends commands to invalid device/feature indices and verifies your client handles Error responses without crashing or disconnecting.

Your client should:
- Parse the Error response (match by Id)
- NOT close the connection
- Continue sending valid commands afterward

#### 4. `ping_timeout`

MaxPingTime=500ms. Your client should intentionally NOT ping (or this tests that your client detects the disconnection when the server drops it). The server will close the connection after ~500ms of silence.

Your client should detect the WebSocket close and enter a disconnected state.

#### 5. `reconnection`

MaxPingTime=0. Tests clean reconnection after the server drops the connection.

Flow: Handshake → Scan → server closes connection → server rebuilds on same port → client reconnects → fresh Handshake → Scan → send commands.

Your client must:
- Detect the initial disconnect
- Clean up internal state (clear device list)
- Reconnect to the same endpoint
- Perform a fresh handshake (not resume)
- Re-enumerate devices

### Simulated Devices

The harness provides three canonical test devices after scanning:

| Index | Name | Output Features | Input Features |
|-------|------|----------------|---------------|
| 0 | Conformance Test Vibrator | Vibrate (×2), Rotate | Battery (Read) |
| 1 | Conformance Test Positioner | Position, HwPositionWithDuration, Oscillate | Button (Subscribe) |
| 2 | Conformance Test Multi | Constrict, Spray, Temperature, Led | RSSI (Read), Pressure (Subscribe) |

Device indices are fixed across all sequences. See [docs/device-definitions.md](docs/device-definitions.md) for complete feature tables and example JSON.

### Protocol Quick Reference

**Wire format:** JSON arrays over WebSocket text frames.

```json
[{"MessageType": {"Id": 1, "Field": "value"}}]
```

**Field names:** PascalCase always.

**Message IDs:** 0 = server-initiated (unsolicited); 1+ = client request/response correlation.

**Handshake (must be first message):**
```json
[{"RequestServerInfo": {"Id": 1, "ClientName": "YourClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]
```

**Scanning:**
```json
[{"StartScanning": {"Id": 2}}]
```

**Output command:**
```json
[{"OutputCmd": {"Id": 3, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 50}}}}]
```

**Input read:**
```json
[{"InputCmd": {"Id": 4, "DeviceIndex": 0, "FeatureIndex": 3, "Type": "Battery", "Command": "Read"}}]
```

**Input subscribe:**
```json
[{"InputCmd": {"Id": 5, "DeviceIndex": 2, "FeatureIndex": 5, "Type": "Pressure", "Command": "Subscribe"}}]
```

**Ping:**
```json
[{"Ping": {"Id": 6}}]
```

**Stop (single device):**
```json
[{"StopCmd": {"Id": 7, "DeviceIndex": 0}}]
```

**Stop (all devices):**
```json
[{"StopCmd": {"Id": 8}}]
```

See [docs/protocol-overview.md](docs/protocol-overview.md) for complete protocol documentation.

### Integration Checklist

Use this checklist when adding conformance testing to your client library's CI:

- [ ] Build the harness binary (or download a release)
- [ ] Start the harness in the background: `./buttplug-client-conformance-test --client-driven --format json &`
- [ ] Run your client test suite against `ws://127.0.0.1:12345`
- [ ] For each sequence: connect, execute the protocol flow, disconnect
- [ ] Parse JSON output or check exit code for pass/fail
- [ ] Add `RUST_LOG=debug` environment variable if you need server-side trace logs

### Debugging Failures

1. **Read the step name and error** — the harness reports exactly which step failed and why
2. **Check [docs/test-sequences.md](docs/test-sequences.md)** — step-by-step expected JSON for every sequence
3. **Enable tracing** — `RUST_LOG=debug ./buttplug-client-conformance-test ...` shows all messages
4. **Run one sequence at a time** — `--sequence core_protocol` isolates the failure
5. **Check message framing** — messages must be JSON arrays, not bare objects
6. **Check field casing** — PascalCase is mandatory (`DeviceIndex`, not `device_index`)
7. **Check Id correlation** — response Id must match your request Id

### Example: Minimal Conformance Test Script

```python
# Pseudocode for a minimal client-driven conformance run
import websocket, json, subprocess, time

# Start harness
proc = subprocess.Popen([
    "./buttplug-client-conformance-test",
    "--client-driven", "--sequence", "core_protocol",
    "--format", "json"
])
time.sleep(1)  # Wait for server to start listening

# Connect
ws = websocket.create_connection("ws://127.0.0.1:12345")

# Handshake
ws.send(json.dumps([{"RequestServerInfo": {"Id": 1, "ClientName": "TestClient", "ProtocolVersionMajor": 4, "ProtocolVersionMinor": 0}}]))
response = json.loads(ws.recv())  # ServerInfo

# Scan
ws.send(json.dumps([{"StartScanning": {"Id": 2}}]))
response = json.loads(ws.recv())  # Ok + DeviceList

# Send a vibrate command
ws.send(json.dumps([{"OutputCmd": {"Id": 3, "DeviceIndex": 0, "FeatureIndex": 0, "Command": {"Vibrate": {"Value": 50}}}}]))
response = json.loads(ws.recv())  # Ok

# ... continue through sequence steps ...

ws.close()
proc.wait()
assert proc.returncode == 0
```

## CLI Reference

| Option | Default | Description |
|--------|---------|-------------|
| `--port <PORT>` | 12345 | WebSocket listen port |
| `--sequence <NAME>` | all | Run only the named sequence |
| `--format <FORMAT>` | stdout | Output format: `stdout` or `json` |
| `--timeout <MS>` | 5000 | Per-step timeout in milliseconds |
| `--client-driven` | false | Client drives protocol flow; harness polls for expected state |

## Further Documentation

- [docs/protocol-overview.md](docs/protocol-overview.md) — Complete v4 protocol reference
- [docs/test-sequences.md](docs/test-sequences.md) — Step-by-step expected message flow for all sequences
- [docs/device-definitions.md](docs/device-definitions.md) — Simulated device feature tables and example JSON
