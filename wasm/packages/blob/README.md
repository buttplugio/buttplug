# buttplug-wasm-blob

[![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)
[![Github donate button](https://img.shields.io/badge/github-donate-ff69b4.svg)](https://www.github.com/sponsors/qdot)
[![Discourse Forums](https://img.shields.io/discourse/status?label=buttplug.io%20forums&server=https%3A%2F%2Fdiscuss.buttplug.io)](https://discuss.buttplug.io)
[![Discord](https://img.shields.io/discord/353303527587708932.svg?logo=discord)](https://discord.buttplug.io)
[![bluesky](https://img.shields.io/bluesky/followers/buttplug.io)](https://bsky.app/profile/buttplug.io)

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
