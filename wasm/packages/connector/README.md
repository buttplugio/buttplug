# buttplug-wasm

Buttplug WASM connector for running an embedded Buttplug server directly in the browser via WebAssembly and Web Bluetooth.

## Installation

```bash
npm install buttplug-wasm buttplug
```

## Usage

```typescript
import { ButtplugClient } from 'buttplug';
import { ButtplugWasmClientConnector } from 'buttplug-wasm';

// Optional: enable debug logging
await ButtplugWasmClientConnector.activateLogging("debug");

const connector = new ButtplugWasmClientConnector();
const client = new ButtplugClient("My App");

await client.connect(connector);
await client.startScanning();

client.on("deviceadded", (device) => {
  console.log(`Device connected: ${device.Name}`);
});
```

## How it works

This package provides a `ButtplugWasmClientConnector` that implements `IButtplugClientConnector` from the official `buttplug` JS library. Under the hood, it uses [`buttplug-wasm-blob`](https://www.npmjs.com/package/buttplug-wasm-blob) which bundles a full Buttplug server compiled to WebAssembly.

When you call `connect()`, the connector:

1. Loads and initializes the WASM module (lazy, on first call)
2. Creates an embedded Buttplug server instance
3. Communicates with the server via JSON message passing over the WASM boundary
4. Uses Web Bluetooth for device discovery and communication

No external server process needed — everything runs in-browser.

## Building your own connector?

If you're using a different buttplug JS client library, install [`buttplug-wasm-blob`](https://www.npmjs.com/package/buttplug-wasm-blob) directly for the raw WASM server FFI.

## Requirements

- A browser with Web Bluetooth support (Chrome, Edge, Opera)
- HTTPS context (Web Bluetooth requires secure origins)
- The `buttplug` npm package (peer dependency for `ButtplugClient`)

## License

BSD-3-Clause
