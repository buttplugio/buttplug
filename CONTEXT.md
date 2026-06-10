# Buttplug

A framework for interfacing with intimate hardware devices using a client-server architecture. Clients send abstract commands (vibrate, rotate, etc.) and servers translate them into device-specific protocols over various communication buses.

## Language

**Protocol** (Device Protocol):
The vendor- or product-specific command set used to control a particular device (e.g. Lovense, Kiiroo, WeVibe). Implemented as a `ProtocolHandler`. When unqualified, "protocol" means this.
_Avoid_: Using "protocol" alone when referring to the Buttplug spec version or the communication bus.

**Buttplug Protocol** (Spec Version):
The versioned JSON message format (v0–v4) used for client-server communication. Negotiated during handshake. Referred to in code as `ButtplugMessageSpecVersion`.
_Avoid_: "Protocol" without the "Buttplug" prefix when referring to this.

**Connector**:
The link between a **Client** and a **Server**. Encapsulates two concerns: the communication bus (WebSocket, direct/in-process, stdio, network) and the serialization format (JSON, CBOR, XML — anything serde can handle).
_Avoid_: "Transport" (not a domain term in buttplug).

**Hardware Manager**:
A component that discovers and communicates with devices over a specific communication bus — Bluetooth LE, HID, Serial, USB, etc. Each bus has its own manager implementation (`HardwareCommunicationManager` trait). Named by bus: `btleplug` (BLE), `serial`, `hid`, `xinput`, etc.
_Avoid_: "Transport" when referring to hardware communication.

**Client**:
The consumer of the buttplug API. Sends abstract commands (vibrate at 50%, rotate clockwise), receives device lists and events. Never communicates with hardware directly — always goes through a **Server** via a **Connector**.
_Avoid_: Conflating with HTTP or network clients.

**Server**:
The device management engine. Receives abstract commands from a **Client**, manages **Hardware Managers**, matches discovered hardware to **Protocols**, and translates commands into device-specific operations. Owns the entire device lifecycle. Can run in a separate process or embedded in-process with the client.
_Avoid_: Conflating with web servers or HTTP servers.

**Device**:
The abstract thing a user controls — a collection of features and capabilities. In client context, a `ButtplugClientDevice`; in server context, a managed entry in `ServerDeviceManager` wrapping a protocol handler and hardware. The unqualified term refers to the concept, not a specific layer.
_Avoid_: Using "device" when specifically meaning the physical communication channel (use **Hardware**) or the static capability description (use **Device Definition**).

**Hardware**:
The physical or simulated communication channel to an actual thing. Represented by the `Hardware` struct, which wraps bus-specific I/O behind a common interface. Created and managed by a **Hardware Manager**.
_Avoid_: "Device" when referring specifically to the communication layer.

**Device Definition**:
The static description of what a device model supports — features, endpoints, display name. `BaseDeviceDefinition` in config files, `ServerDeviceDefinition` at runtime.
_Avoid_: "Device config" (ambiguous with user configuration).

**Feature**:
A discrete capability of a **Device**, identified by `feature_index`. A feature can have an **Output**, an **Input**, or both — grouping a related actuator and sensor together (e.g. a motor with an encoder). Represented by `DeviceFeature`.
_Avoid_: "Actuator" or "sensor" as top-level terms; use Output and Input.

**Output**:
The actuator side of a **Feature** — something that makes the device do something physical. Types include Vibrate, Rotate, Oscillate, Constrict, Temperature, Led, Position, Spray.
_Avoid_: "Actuator" (used historically but Output is canonical).

**Input**:
The sensor side of a **Feature** — something that reports a value from the device. Types include Battery, Rssi, Button, Pressure, Depth, Position.
_Avoid_: "Sensor" (used historically but Input is canonical).

**Specifier**:
A description of how to detect a device on a given communication bus — BLE service UUIDs, HID vendor/product IDs, serial port patterns, etc. Tells the system what to look for during scanning. Represented by `ProtocolCommunicationSpecifier` variants.
_Avoid_: "Identifier" when referring to detection criteria.

**Device Identifier**:
The fingerprint of a discovered device — protocol name, optional model/variant string, and address. Ranges from general (model-level `BaseDeviceIdentifier`) to specific (instance-level `UserDeviceIdentifier` with a unique address). The identification pipeline narrows from **Specifier** (what to look for) → discovery → **Device Identifier** (what we found).
_Avoid_: "Specifier" when referring to a matched/discovered device.

**Endpoint**:
A communication channel on a piece of **Hardware** — the target for reads and writes. Borrowed from USB terminology. Named variants (`Tx`, `Rx`, `Command`, `Firmware`, `TxVibrate`) cover common uses; `Generic0`–`Generic31` cover protocols that don't fit named slots.
_Avoid_: Conflating with API or network endpoints.

**Intiface**:
The user-facing brand for the buttplug ecosystem. Exists because "Buttplug" can't go on app stores. Buttplug is the library (embedded in applications); Intiface is anything user-facing built on top of it.
_Avoid_: Using "Buttplug" when referring to user-facing products or applications.

**Intiface Engine**:
The bridge between the buttplug library and external frontends. Wraps a **Server** with connector support, CLI options, and frontend message broadcasting. Where "library" becomes "runnable service." Lives in this repo as the `intiface_engine` crate.
_Avoid_: Conflating with Intiface Central (the GUI, separate repo).

**Simulated Device**:
A virtual **Device** that runs the full device lifecycle without real hardware. Configured in **User Configuration** by referencing an archetype (e.g. simulated-1vibe, simulated-rotator). User-facing feature for testing and development of applications built on buttplug — not just internal test infrastructure. Combined with **Output Observations**, lets developers see exactly what their application would do with real hardware.
_Avoid_: "Mock device" (simulated devices run through the real device lifecycle, not a mock).

**Device Configuration**:
The built-in database of known devices — what protocols exist, what specifiers identify them, what features they support. Ships with buttplug, sourced from YAML files in `buttplug_server_device_config`.
_Avoid_: "Device config" without qualification (ambiguous with **User Configuration**).

**User Configuration**:
User-supplied overrides layered on top of **Device Configuration** — display names, feature allows/denies, simulated device entries. Represented by `UserConfigDefinition`.
_Avoid_: "Device config" when referring to user overrides.

**Scanning**:
The process of **Hardware Managers** searching their communication buses for devices. Triggered by the client via `StartScanning`. When hardware is found, identification happens immediately — each **Protocol**'s **Specifiers** determine both "we can talk to this" and "this device uses this protocol" in a single step.
_Avoid_: "Discovery" as a distinct stage from identification — they're the same step.

**Output Observation**:
An opt-in broadcast of every output command sent to a **Device** — device index, feature index, output type, and value. Used by frontends (e.g. Intiface Central) to visually display real-time device activity, verify hardware behaviour matches commands, and let developers see what *would* happen with simulated devices. Disabled by default to avoid overhead.
_Avoid_: Treating as internal-only debugging; it's a user-facing observability feature.

**Task Scope**:
The owner of spawned async tasks within a module. Every task is spawned through a Task Scope, which links it to a parent, derives its hierarchical name (e.g. `server/device-manager/device-3/keepalive`), registers it in the **Task Registry**, and hands it a cooperative cancellation token. Dropping a scope cancels its children. Tasks cannot be spawned without a parent scope.
_Avoid_: "Detached task" or bare spawning as the normal pattern; detachment is the rare, explicit exception.

**Task Registry**:
The queryable record of every live task — id, hierarchical path, parent, state. Populated as a side effect of spawning through a **Task Scope**. Exposed in-process for tests and embedders, and to frontends via TaskStarted/TaskEnded **Events** plus a snapshot query (same opt-in pattern as **Output Observation**).
_Avoid_: Treating as internal-only debugging; like Output Observations, it's user-facing observability.

**Command**:
A message from **Client** to **Server** requesting an action — controlling a device, starting a scan, requesting device lists. Always has a non-zero message ID; the server responds with an `Ok` or `Error` using the same ID.
_Avoid_: Referring to server-initiated messages as commands.

**Event**:
A message from **Server** to **Client** that the client didn't explicitly request — device added/removed, scanning finished, input readings from subscribed sensors. Always has message ID `0`.
_Avoid_: "Notification" (not used in the spec).

**Device Lifecycle**:
The stages a device moves through: Scanning → Identification → Connection → Configuration → Operation. Identification and protocol matching are a single step — a device is identified *via* its protocol's specifiers.

## Example Dialogue

> **Dev**: A user says their device isn't showing up. Where do I start?
>
> **Domain expert**: First check which **Hardware Manager** covers their communication bus — if it's BLE, that's the btleplug manager. Is **Scanning** even finding the hardware? If so, the **Specifiers** for that **Protocol** might not match the device's advertisement data. Check the **Device Configuration** for the protocol's BLE specifier — service UUIDs, manufacturer data patterns.
>
> **Dev**: It shows up but all the **Features** are wrong — it's showing two **Outputs** but the device only vibrates.
>
> **Domain expert**: That's a **Device Definition** issue. The protocol's base definition might be wrong, or the user's **User Configuration** could be overriding it. Check the YAML in `buttplug_server_device_config` first, then see if there's a user config layered on top. Also check whether the **Device Identifier** is matching the right variant — some protocols have multiple identifier strings for different models.
>
> **Dev**: Can I test this without the physical device?
>
> **Domain expert**: Set up a **Simulated Device** using one of the archetypes in user config. Enable **Output Observations** and you'll see every command that *would* go to hardware. **Intiface Engine** will broadcast those to any connected frontend.
