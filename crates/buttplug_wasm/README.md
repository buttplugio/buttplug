# buttplug_wasm (Internal)

This is the Rust FFI layer that compiles Buttplug to WebAssembly via `wasm-bindgen`. It is an internal build artifact and is **not** meant for direct consumption.

## What this crate does

- Exposes Buttplug server functionality to JavaScript/TypeScript through wasm-bindgen
- Produces WASM output consumed by the [`buttplug-wasm`](https://www.npmjs.com/package/buttplug-wasm) npm package
- Includes the WebBluetooth hardware manager for browser-native device discovery

## For users

Install the `buttplug-wasm` npm package instead:

```bash
npm install buttplug-wasm
```

## Building (for contributors)

```bash
wasm-pack build --target web crates/buttplug_wasm
```

This outputs to `pkg/` (gitignored). The output is consumed by `../../wasm/` during its vite build.

## Why `publish = false`?

This crate is not useful on its own — it's a thin FFI shim over `buttplug_server`. The publishable artifact is the npm package that bundles the compiled WASM binary with a TypeScript connector class.
