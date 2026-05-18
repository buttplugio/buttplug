# buttplug-wasm

[![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)
[![Github donate button](https://img.shields.io/badge/github-donate-ff69b4.svg)](https://www.github.com/sponsors/qdot)
[![Discourse Forums](https://img.shields.io/discourse/status?label=buttplug.io%20forums&server=https%3A%2F%2Fdiscuss.buttplug.io)](https://discuss.buttplug.io)
[![Discord](https://img.shields.io/discord/353303527587708932.svg?logo=discord)](https://discord.buttplug.io)
[![bluesky](https://img.shields.io/bluesky/followers/buttplug.io)](https://bsky.app/profile/buttplug.io)

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

## Filing Issues and Contributing

If you have issues or feature requests, please feel free to [file an
issue](https://github.com/buttplugio/buttplug/issues).

**We are not looking for unsolicited code contributions or pull requests, and will not accept
pull requests that do not have a matching issue where the matter was previously discussed in an issue on this repo or in one of our communication channels, listed below.** 

Pull requests should only be submitted after talking to [qdot](https://github.com/qdot) via issues
(or on [Discord](https://discord.buttplug.io), [our forums](https://discuss.buttplug.io), or via DMs
on one of our social media accounts if you would like to stay anonymous and out of recorded info on
the repo) and receiving approval to develop code based on an issue. Any random or non-issue pull
requests will most likely be closed without merging.

If you'd like to contribute in a non-technical way, we need money to keep up with supporting the
latest and greatest hardware. We have multiple ways to donate!

- [Patreon](https://patreon.com/qdot)
- [Github Sponsors](https://github.com/sponsors/qdot)
- [Ko-Fi](https://ko-fi.com/qdot76367)

## License

BSD-3-Clause
