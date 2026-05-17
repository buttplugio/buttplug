import { ButtplugMessage, IButtplugClientConnector } from 'buttplug';
import { EventEmitter } from 'eventemitter3';
import {
  loadButtplugWasm,
  createServer,
  freeServer,
  sendMessage,
  activateLogging,
} from 'buttplug-wasm-blob';

export class ButtplugWasmClientConnector extends EventEmitter implements IButtplugClientConnector {
  private static _loggingActivated = false;
  private _connected: boolean = false;
  private handle: number | null = null;

  public get Connected(): boolean {
    return this._connected;
  }

  public static activateLogging = async (logLevel: string = "debug") => {
    if (ButtplugWasmClientConnector._loggingActivated) return;
    await loadButtplugWasm();
    activateLogging(logLevel);
    ButtplugWasmClientConnector._loggingActivated = true;
  };

  public initialize = async (): Promise<void> => {};

  public connect = async (): Promise<void> => {
    await loadButtplugWasm();
    this.handle = createServer((msgs: Uint8Array) => {
      this.emitMessage(msgs);
    });
    this._connected = true;
  };

  public disconnect = async (): Promise<void> => {
    if (this.handle != null) {
      freeServer(this.handle);
      this.handle = null;
    }
    this._connected = false;
  };

  public send = (msg: ButtplugMessage): void => {
    sendMessage(
      this.handle!,
      new TextEncoder().encode('[' + JSON.stringify(msg) + ']'),
      (output: Uint8Array) => {
        this.emitMessage(output);
      },
    );
  };

  private emitMessage = (msg: Uint8Array) => {
    this.emit('message', JSON.parse(new TextDecoder().decode(msg)));
  };
}
