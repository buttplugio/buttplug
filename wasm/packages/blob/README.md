# buttplug-wasm-blob

Buttplug WASM server binary with a typed FFI wrapper. This package bundles a complete Buttplug server compiled to WebAssembly with Web Bluetooth support.

## Who is this for?

- **Building your own buttplug client?** Use this package directly and write your own connector.
- **Using the official buttplug JS client?** Install [`buttplug-wasm`](https://www.npmjs.com/package/buttplug-wasm) instead, which provides a ready-made connector.

## Installation

```bash
npm install buttplug-wasm-blob
```

## API

```typescript
import {
  loadButtplugWasm,
  createServer,
  freeServer,
  sendMessage,
  activateLogging,
} from 'buttplug-wasm-blob';

// Load the WASM module (call once, lazy-loaded)
await loadButtplugWasm();

// Optional: enable debug logging
activateLogging("debug");

// Create a server — the callback receives server events as JSON bytes
const handle = createServer((msg: Uint8Array) => {
  const json = new TextDecoder().decode(msg);
  console.log("Server event:", JSON.parse(json));
});

// Send a message to the server — wrap as JSON array bytes
const request = JSON.stringify({ RequestServerInfo: { Id: 1, ClientName: "MyApp", ProtocolVersionMajor: 4, ProtocolVersionMinor: 0 } });
sendMessage(handle, new TextEncoder().encode('[' + request + ']'), (response: Uint8Array) => {
  console.log("Response:", JSON.parse(new TextDecoder().decode(response)));
});

// Clean up
freeServer(handle);
```

## Message Format

All messages are JSON-encoded Buttplug protocol v4 messages, wrapped in arrays and passed as `Uint8Array` (UTF-8 bytes). See the [Buttplug protocol spec](https://buttplug-spec.docs.buttplug.io/) for message formats.

## Requirements

- A browser with Web Bluetooth support (Chrome, Edge, Opera)
- HTTPS context (Web Bluetooth requires secure origins)

## License

BSD-3-Clause
