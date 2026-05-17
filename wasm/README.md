# Buttplug WASM Packages

This workspace contains the npm packages for running a Buttplug server in the browser via WebAssembly.

## Packages

| Package | Description |
|---------|-------------|
| [`buttplug-wasm-blob`](packages/blob/) | WASM server binary + typed FFI wrapper. Use this if you're building your own connector for a third-party buttplug client library. |
| [`buttplug-wasm`](packages/connector/) | Ready-made connector for the official [`buttplug`](https://www.npmjs.com/package/buttplug) JS client library. |

## Quick Start

If you're using the official buttplug JS client:

```bash
npm install buttplug-wasm buttplug
```

```typescript
import { ButtplugClient } from 'buttplug';
import { ButtplugWasmClientConnector } from 'buttplug-wasm';

const connector = new ButtplugWasmClientConnector();
const client = new ButtplugClient("My App");
await client.connect(connector);
```

## Building

```bash
npm install
npm run build        # builds wasm-pack + both packages
```

## License

BSD-3-Clause
