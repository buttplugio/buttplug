import { ButtplugMessage, IButtplugClientConnector, fromJSON } from 'buttplug';
import { EventEmitter } from 'eventemitter3';

export class ButtplugWasmClientConnector extends EventEmitter implements IButtplugClientConnector {
  private static _loggingActivated = false;
  private static wasmInstance: any;
  private _connected: boolean = false;
  private client: any;

  constructor() {
    super();
  }

  public get Connected(): boolean {
    return this._connected;
  }

  private static maybeLoadWasm = async () => {
    if (ButtplugWasmClientConnector.wasmInstance == undefined) {
      const wasm = await import('@wasm/buttplug_wasm.js');
      await wasm.default();
      ButtplugWasmClientConnector.wasmInstance = wasm;
    }
  };

  public static activateLogging = async (logLevel: string = "debug") => {
    await ButtplugWasmClientConnector.maybeLoadWasm();
    if (ButtplugWasmClientConnector._loggingActivated) {
      console.log("Logging already activated, ignoring.");
      return;
    }
    console.log("Turning on logging.");
    ButtplugWasmClientConnector.wasmInstance.buttplug_activate_env_logger(logLevel);
    ButtplugWasmClientConnector._loggingActivated = true;
  };

  public initialize = async (): Promise<void> => {};

  public connect = async (): Promise<void> => {
    await ButtplugWasmClientConnector.maybeLoadWasm();
    this.client = ButtplugWasmClientConnector.wasmInstance.buttplug_create_embedded_wasm_server(
      (msgs: Uint8Array) => {
        this.emitMessage(msgs);
      }
    );
    this._connected = true;
  };

  public disconnect = async (): Promise<void> => {
    if (this.client != null) {
      ButtplugWasmClientConnector.wasmInstance.buttplug_free_embedded_wasm_server(this.client);
      this.client = null;
    }
    this._connected = false;
  };

  public send = (msg: ButtplugMessage): void => {
    ButtplugWasmClientConnector.wasmInstance.buttplug_client_send_json_message(
      this.client,
      new TextEncoder().encode('[' + msg.toJSON() + ']'),
      (output: Uint8Array) => {
        this.emitMessage(output);
      }
    );
  };

  private emitMessage = (msg: Uint8Array) => {
    const str = new TextDecoder().decode(msg);
    this.emit('message', fromJSON(str));
  };
}
