export type ButtplugServerCallback = (msg: Uint8Array) => void;
export type ButtplugServerHandle = number;

let wasmInstance: any;

export async function loadButtplugWasm(): Promise<void> {
  if (wasmInstance == undefined) {
    const wasm = await import('@wasm/buttplug_wasm.js');
    await wasm.default();
    wasmInstance = wasm;
  }
}

export function createServer(callback: ButtplugServerCallback): ButtplugServerHandle {
  return wasmInstance.buttplug_create_embedded_wasm_server(callback);
}

export function freeServer(handle: ButtplugServerHandle): void {
  wasmInstance.buttplug_free_embedded_wasm_server(handle);
}

export function sendMessage(
  handle: ButtplugServerHandle,
  msg: Uint8Array,
  callback: ButtplugServerCallback,
): void {
  wasmInstance.buttplug_client_send_json_message(handle, msg, callback);
}

export function activateLogging(level: string = "debug"): void {
  wasmInstance.buttplug_activate_env_logger(level);
}
