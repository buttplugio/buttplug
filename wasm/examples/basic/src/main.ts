import { ButtplugClient, ButtplugClientDevice } from "buttplug";
import { ButtplugWasmClientConnector } from "buttplug-wasm";

// ── DOM refs ──────────────────────────────────────────────────────────────────

const btnConnect    = document.getElementById("btn-connect")!    as HTMLButtonElement;
const btnLogging    = document.getElementById("btn-logging")!    as HTMLButtonElement;
const btnScan       = document.getElementById("btn-scan")!       as HTMLButtonElement;
const btnStopScan   = document.getElementById("btn-stop-scan")!  as HTMLButtonElement;
const btnDisconnect = document.getElementById("btn-disconnect")! as HTMLButtonElement;
const deviceListEl  = document.getElementById("device-list")!;
const logEl         = document.getElementById("log")!;

// ── State ─────────────────────────────────────────────────────────────────────

let client: ButtplugClient | null = null;

// ── Utilities ─────────────────────────────────────────────────────────────────

function log(msg: string) {
  const ts = new Date().toISOString().slice(11, 23);
  logEl.textContent += `[${ts}] ${msg}\n`;
  logEl.scrollTop = logEl.scrollHeight;
}

function clearChildren(el: Element) {
  while (el.firstChild) {
    el.removeChild(el.firstChild);
  }
}

function renderDevices(devices: ButtplugClientDevice[]) {
  clearChildren(deviceListEl);

  if (devices.length === 0) {
    const em = document.createElement("em");
    em.style.color = "#666";
    em.textContent = "None connected";
    deviceListEl.appendChild(em);
    return;
  }

  for (const device of devices) {
    const row = document.createElement("div");
    row.className = "device-row";

    const name = document.createElement("span");
    name.className = "device-name";
    name.textContent = device.name;

    const slider = document.createElement("input");
    slider.type = "range";
    slider.min = "0";
    slider.max = "100";
    slider.value = "50";
    slider.title = "Speed";

    const btnVibrate = document.createElement("button");
    btnVibrate.textContent = "Vibrate";
    btnVibrate.addEventListener("click", async () => {
      const speed = Number(slider.value) / 100;
      try {
        await device.vibrate(speed);
        log(`${device.name}: vibrate @ ${speed.toFixed(2)}`);
      } catch (e) {
        log(`${device.name}: vibrate error — ${e}`);
      }
    });

    const btnStop = document.createElement("button");
    btnStop.textContent = "Stop";
    btnStop.addEventListener("click", async () => {
      try {
        await device.stop();
        log(`${device.name}: stopped`);
      } catch (e) {
        log(`${device.name}: stop error — ${e}`);
      }
    });

    row.append(name, slider, btnVibrate, btnStop);
    deviceListEl.appendChild(row);
  }
}

function setConnectedState(connected: boolean) {
  btnConnect.disabled    = connected;
  btnLogging.disabled    = !connected;
  btnScan.disabled       = !connected;
  btnStopScan.disabled   = true;
  btnDisconnect.disabled = !connected;
}

// ── Event handlers ────────────────────────────────────────────────────────────

btnConnect.addEventListener("click", async () => {
  btnConnect.disabled = true;

  try {
    const connector = new ButtplugWasmClientConnector();
    client = new ButtplugClient("buttplug-wasm example");

    client.addListener("deviceadded", (device: ButtplugClientDevice) => {
      log(`Device added: ${device.name}`);
      renderDevices(client!.devices);
    });

    client.addListener("deviceremoved", (device: ButtplugClientDevice) => {
      log(`Device removed: ${device.name}`);
      renderDevices(client!.devices);
    });

    client.addListener("disconnect", () => {
      log("Server disconnected.");
      setConnectedState(false);
      renderDevices([]);
      client = null;
    });

    await client.connect(connector);
    log("Connected.");
    setConnectedState(true);
  } catch (e) {
    log(`Connect failed: ${e}`);
    btnConnect.disabled = false;
  }
});

btnLogging.addEventListener("click", async () => {
  try {
    await ButtplugWasmClientConnector.activateLogging("debug");
    log("WASM logging enabled (see browser console).");
    btnLogging.disabled = true;
  } catch (e) {
    log(`Logging error: ${e}`);
  }
});

btnScan.addEventListener("click", async () => {
  if (!client) return;
  try {
    await client.startScanning();
    log("Scanning started.");
    btnScan.disabled     = true;
    btnStopScan.disabled = false;
  } catch (e) {
    log(`Scan error: ${e}`);
  }
});

btnStopScan.addEventListener("click", async () => {
  if (!client) return;
  try {
    await client.stopScanning();
    log("Scanning stopped.");
    btnScan.disabled     = false;
    btnStopScan.disabled = true;
  } catch (e) {
    log(`Stop scan error: ${e}`);
  }
});

btnDisconnect.addEventListener("click", async () => {
  if (!client) return;
  try {
    await client.disconnect();
    log("Disconnected.");
    setConnectedState(false);
    renderDevices([]);
    client = null;
  } catch (e) {
    log(`Disconnect error: ${e}`);
  }
});

// ── Init ──────────────────────────────────────────────────────────────────────

log("Ready. Click Connect to start.");
log("Requires Chrome or Edge (WebBluetooth support).");
