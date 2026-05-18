# Output Observability - Human Test Plan

## Prerequisites

- Build the project: `cargo build`
- All automated tests passing: `cargo test -p buttplug_tests --test test_output_observations`
- Intiface Central or equivalent Frontend implementation available
- A Buttplug-compatible device (physical or simulated via WebSocket device forwarder)
- A Buttplug client application capable of sending vibrate/stop commands

## Phase 1: End-to-End Observation Flow (AC4.1, AC4.3)

| Step | Action | Expected |
|------|--------|----------|
| 1 | Build Intiface Engine with `emit_output_observations: true` in EngineOptions. If using EngineOptionsBuilder: call `.emit_output_observations(true)` before `.finish()`. | Engine compiles and starts without errors. |
| 2 | Start the engine with a Frontend connected (Intiface Central or WebSocket frontend). | Engine reports "EngineStarted" to the frontend. |
| 3 | Connect a test device (physical BLE or WebSocket forwarder). | Frontend receives `DeviceConnected` message. |
| 4 | From a client, send a Vibrate command at 50%. | Frontend receives `DeviceOutputObservation` with `device_index` matching connected device, `feature_index: 0`, `output_type: "Vibrate"`, `value: 50.0`. |
| 5 | Send the same Vibrate at 50% again. | No `DeviceOutputObservation` appears (dedup). |
| 6 | Send Vibrate at 75%. | Frontend receives `DeviceOutputObservation` with `value: 75.0`. |
| 7 | Send StopDeviceCmd. | Frontend receives `DeviceOutputObservation` with `value: 0.0` for each stoppable feature. |
| 8 | Send StopDeviceCmd again (already at zero). | No `DeviceOutputObservation` appears (dedup at zero). |
| 9 | Disconnect client and stop engine. | Clean shutdown, no crashes or panics. |

## Phase 2: Disabled Mode Verification

| Step | Action | Expected |
|------|--------|----------|
| 1 | Start Intiface Engine with `emit_output_observations: false` (the default). | Engine starts normally. |
| 2 | Connect a device and send vibrate commands. | No `DeviceOutputObservation` messages appear. All other messages work normally. |
| 3 | Check engine logs. | No observation-related channel errors or panics. |

## Phase 3: Multi-Device Stop-All (supplements AC3.2)

| Step | Action | Expected |
|------|--------|----------|
| 1 | With observations enabled, connect two or more devices. | Frontend shows `DeviceConnected` for each. |
| 2 | Send vibrate commands to each device. | `DeviceOutputObservation` messages for each device/feature. |
| 3 | Send StopAllDevices. | `DeviceOutputObservation` with `value: 0.0` for every stoppable feature on every device. |

## Full Session Lifecycle

1. Start engine with observations enabled and Frontend connected.
2. Connect a client and device.
3. Send vibrate commands at 25, 50, 75, 100%.
4. Verify each non-duplicate produces a `DeviceOutputObservation`.
5. Disconnect client (not device).
6. Reconnect client.
7. Send vibrate at same value as before disconnect.
8. Document whether observation is emitted (dedup state may reset on reconnect).
9. Send StopAllDevices and verify zero-value observations.
10. Disconnect everything, verify clean shutdown.

## Traceability

| Acceptance Criterion | Automated Test | Manual Step |
|---------------------|---------------|-------------|
| AC1.1 - EngineOptions field | Structural | -- |
| AC1.2 - External options and builder | Structural | -- |
| AC1.3 - Disabled = no channel | `test_ac5_1_disabled_no_observation_stream` | Phase 2 step 2 |
| AC2.1 - Observation emission | `test_ac2_1_observation_emission` | Phase 1 step 4 |
| AC2.2 - Dedup suppression | `test_ac2_2_observation_dedup` | Phase 1 step 5 |
| AC2.3 - Tap point before protocol | `test_ac2_3_observation_before_protocol` | -- |
| AC3.1 - StopDevice as zero | `test_ac3_1_stop_as_zero` | Phase 1 step 7 |
| AC3.2 - StopAllDevices | `test_ac3_2_stop_all_devices` (single device) | Phase 3 (multi-device) |
| AC3.3 - Stop dedup at zero | `test_ac3_3_stop_dedup` | Phase 1 step 8 |
| AC4.1 - E2E observation flow | None (human only) | Phase 1 steps 1-9 |
| AC4.2 - EngineMessage fields | Structural | -- |
| AC4.3 - Frontend receives observations | None (human only) | Phase 1 step 4 |
| AC5.1 - Disabled = None stream | `test_ac5_1_disabled_no_observation_stream` | Phase 2 steps 1-3 |
| AC5.2 - No allocation when disabled | Structural | Phase 2 step 3 |
