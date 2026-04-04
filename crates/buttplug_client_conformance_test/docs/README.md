# Buttplug Client Conformance Test Harness

A standalone WebSocket server that validates Buttplug protocol client implementations. The harness runs a real Buttplug server with simulated hardware, allowing any client implementation (in any language) to validate protocol correctness against known server behavior.

## What This Is

This harness provides a **conformance test suite for Buttplug clients**. It:

- Runs a real Buttplug server with a simulated hardware layer (no physical devices needed)
- Listens on a WebSocket port for client connections
- Drives each client through scripted protocol sequences
- Validates that the client sends correct messages, handles responses, and respects timing requirements
- Produces human-readable and JSON output for CI/CD integration

Use this to validate your Buttplug client implementation before deploying to production.

## Quick Start

### Build

```bash
cargo build -p buttplug_client_conformance_test --release
```

The binary is available at `./target/release/buttplug-client-conformance-test`.

### Run All Sequences

```bash
./target/release/buttplug-client-conformance-test
```

This starts the server and runs all sequences sequentially. For each sequence:
1. Server prints the sequence name and waits for a connection
2. Connect your client to `ws://127.0.0.1:19999`
3. Client is driven through the test steps
4. Server disconnects after sequence completes
5. Server moves to next sequence

### Run a Specific Sequence

```bash
./target/release/buttplug-client-conformance-test --sequence core_protocol
```

Available sequences: `core_protocol`, `ping_required`, `error_handling`, `ping_timeout`, `reconnection`

### Custom Port

```bash
./target/release/buttplug-client-conformance-test --port 20000
```

### JSON Output

```bash
./target/release/buttplug-client-conformance-test --format json
```

Outputs results as JSON for automated CI/CD pipelines.

## How It Works

### Connection Flow

1. **Client connects** to `ws://127.0.0.1:<port>`
2. **Server prints** which sequence is running and waits for connection
3. **Server drives test steps** by:
   - Sending expected messages
   - Validating client responses
   - Reporting pass/fail for each step
4. **Client disconnects** (or server disconnects after timeout)
5. **Server repeats** with next sequence

### The RED-GREEN Loop

This harness is designed for iterative development:

1. Run the harness: `./target/release/buttplug-client-conformance-test`
2. See which test step fails
3. Fix your client implementation
4. Reconnect and run again
5. Repeat until all tests pass

### Test Sequences

| Sequence | Purpose |
|----------|---------|
| `core_protocol` | Full protocol exercise without ping requirements |
| `ping_required` | Validates ping/keepalive mechanism |
| `error_handling` | Tests error response handling |
| `ping_timeout` | Validates server-initiated disconnect on ping timeout |
| `reconnection` | Tests client reconnection after server disconnect |

Each sequence is independent and can be run separately.

## CLI Options

```
USAGE:
    buttplug-client-conformance-test [OPTIONS]

OPTIONS:
    -h, --help                     Print help information
    -s, --sequence <SEQUENCE>      Run a specific sequence (core_protocol, ping_required, error_handling, ping_timeout, reconnection)
    -p, --port <PORT>              WebSocket server port (default: 19999)
    -f, --format <FORMAT>          Output format: human (default) or json
    --no-color                     Disable colored output
```

## Output Formats

### Human-Readable (default)

```
Running sequence: core_protocol
Waiting for client connection on ws://127.0.0.1:19999...
Client connected
Step 1: Client sends RequestServerInfo
  ✓ PASS: Server responds with ServerInfo
Step 2: Client scans for devices
  ✓ PASS: Server responds with DeviceList (3 devices)
...
Sequence passed: core_protocol
```

### JSON Output

```json
{
  "sequence": "core_protocol",
  "status": "pass",
  "steps": [
    {
      "step": 1,
      "name": "Client sends RequestServerInfo",
      "status": "pass"
    },
    ...
  ],
  "duration_ms": 1234
}
```

## Documentation

- **[protocol-overview.md](protocol-overview.md)** — v4 protocol reference for client implementors
- **[test-sequences.md](test-sequences.md)** — Step-by-step test flow for each sequence
- **[device-definitions.md](device-definitions.md)** — Details of the three simulated test devices

## Debugging a Failed Test

1. **Read the error message** — The harness prints which step failed and why
2. **Check [test-sequences.md](test-sequences.md)** — See exactly what the server expects at that step
3. **Check [protocol-overview.md](protocol-overview.md)** — Verify message JSON format and field names
4. **Check your client logs** — Compare the JSON you're sending to what the docs specify
5. **Run with a specific sequence** — Use `--sequence <name>` to focus on the failing test

## Testing Your Client

1. Implement your Buttplug client
2. Run the conformance harness: `./target/release/buttplug-client-conformance-test`
3. Connect your client to `ws://127.0.0.1:19999`
4. If tests fail, check the error message and the documentation
5. Fix your client and reconnect
6. Repeat until all sequences pass

All tests passing means your client correctly implements the Buttplug protocol.
