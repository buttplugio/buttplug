# HISMITH S2 / Servo machines (fork notes)

This fork adds depth control and extra functions for the HISMITH servo machines (`hismith-mini`
protocol, model code `1101`). The official app's built-in product catalog names code `1101`
"Hismith S2" (type "SMS", command family `CC`); `1005` is the S1 and `1102` the S3, which have no
position/stroke control in the app. The frames come from static analysis of the official HiSmith app
2.7.7. They are **not yet verified on real hardware**.

All frames are `CC <opcode> <value> <(opcode+value) & 0xFF>` written to characteristic
`0000ffe9-0000-1000-8000-00805f9b34fb`.

## Features exposed for model `1101`

| Index | Description | Buttplug output | Range | Frame |
|---|---|---|---|---|
| 0 | Fucking Machine Oscillation Speed | Oscillate | 0-100 | `CC 03` |
| 1 | Depth (moves a fixed-width stroke window) | Position | 0-100 | `CC 07` + `CC 08` |
| 2 | Stroke width (size of the depth window) | Position | 0-100, clamped to >= 10 | re-places the window |
| 3 | Run mode (0 = auto, 1 = manual) | Position | 0-1 | `CC 01 A0` / `CC 01 A1` |
| 4 | Preset mode (1-8, 0 = none) | Position | 0-8 | `CC 04 <n>` (0 sends nothing) |

Depth 0..100 slides a window of the current width along the machine's travel: with the default
width 40, depth 0 -> 0..40, 50 -> 30..70, 100 -> 60..100; with width 10 and depth 100 the window is
90..100. The two limits are written in the order that keeps the window valid (deeper: end first,
shallower: start first); the first command always writes both because the machine's real window is
unknown. Width 10 is the smallest window the official app allows; whether the machine accepts
less is untested. The depth/width window works in auto mode (the machine strokes on its own at the
Oscillate speed); a speed command leaves a preset mode (to be confirmed on hardware).

A definition with more than one Position feature besides this layout (the older seven-actuator
user config: stroke start/end, position, smoothness, run mode, preset) keeps the raw per-opcode
behaviour; the new layout is recognised by its Position feature count (1 or 4).

## Configuring speed and depth

Every range above is just the default. If you saved a user device config for an earlier version of
this fork, regenerate it: a user config must list every feature of the device, and features 5 and 6
were added later. Buttplug rescales client values (0.0-1.0) to the range
of the feature, so lowering a range is the easy way to configure limits, for example a safe maximum
speed (the official app defaults to 40) or a maximum depth.

1. Connect the machine once and note the device identifier in the log:
   `Hismith Device Identifier: 1101` (if you see another number, the S2 reports a different
   model code - add that code to the `identifier` list of the servo entry in
   `crates/buttplug_server_device_config/device-config/protocols/hismith-mini.yml`).
2. Put the limits in your user device config. A working example (speed capped at 20) is
   `crates/buttplug_tests/tests/util/device_test/device_test_case/config/hismith_servo_user_config.json`:
   set `address` to the machine's Bluetooth address, and change each feature's
   `"value": [min, max]`. Keep `base_id` as is; `id` values are yours.
3. Or edit the defaults in `hismith-mini.yml` and rebuild.

The test `test_hismith_servo_user_config.yaml` shows the effect: with speed capped to 0-20,
100% speed is sent as `CC 03 14 17`.

## Other functions in the official app (not exposed yet)

| Function | Frame | Notes |
|---|---|---|
| Auto / manual run mode | `CC 01 A0` / `CC 01 A1` | no Buttplug output type; needs a switch type |
| Reversal on / off | `CC 01 C1` / `CC 01 C0` | already reachable as Rotate direction in this protocol |
| Voice/music mode on / off | `CC 01 B1` / `CC 01 B0` | microphone-driven mode |
| Speed mode (preset pattern) | `CC 04 <n>` | pattern numbers are served by the HiSmith web API |
| Vibration / vibration mode | `CC 05` / `CC 06` | only if the machine has a vibrator |
| Burst ("piu") on / off | `CC 0B 01` / `CC 0B 00` | |
| Shutdown | `CC 02 00 02` | |
| Battery | write `CC 00 00 00`, answer `CC 00 <percent>` on `ffe4` | needs a notify endpoint in the config |

## Using it from MultiFunPlayer

MultiFunPlayer talks to Buttplug/Intiface with spec v3 (`Buttplug.Net`), so the features show up as
*scalar* actuators. For model `1101` MultiFunPlayer sees:

| Actuator type | Index | Function |
|---|---|---|
| Oscillate | 0 | Speed (`CC 03`) |
| Position | 1 | Stroke Start (`CC 07`) |
| Position | 2 | Stroke End (`CC 08`) |
| Position | 3 | Position, move to depth now (`CC 0A`) |
| Position | 4 | Smoothness (`CC 09`) |

(The index is the position in the device's scalar list, which is what MultiFunPlayer shows.)
This path is covered by the test `test_hismith_servo_depth.yaml`, which sends exactly these v3
`ScalarCmd` messages.

Suggested mapping in the Buttplug.io output target: stroke axis `L0` -> `Position/3` for scripted
depth; use `Position/1` and `Position/2` for the stroke window and `Position/4` for smoothness
only if you want to drive them from an axis. The "Stop" that MultiFunPlayer sends only sets speed
to 0 (`CC 03 00 03`); Position features are deliberately left where they are.

Stock MultiFunPlayer lists only the type and index, so the four Position actuators are hard to
tell apart. The MultiFunPlayer change that adds the feature descriptions to the actuator picker is
described in the MultiFunPlayer fork notes.

## Hismith Lube Launcher (PiuPiu, model `4002`)

The Lube Launcher 2.0 is the app's "Hismith PiuPiu" (BLE name `Hismith Piupiu`). In the app one
`CC 0B 01 0C` frame is one squirt; holding the button repeats it every 100 ms, and the "off" frame
(`CC 0B 00 0B`) is never used. In this fork:

* Feature 0 is "Spray" with both a `spray` and a `position` output (0-1). Any value above 0 is one
  squirt; **0 does nothing** (before, 0 and every Stop fired it). Squirts are limited to one per
  100 ms, like the app.
* Spec v3 clients (MultiFunPlayer / Buttplug.Net) do not get the `Spray` actuator any more: their
  parser rejects the whole device when it sees an actuator type that is not in spec v3. They see
  it as `Position/0` instead.
* Not implemented: the app's battery reading (`CC 00 00 00`, answer on `ffe4`), the timer
  (done in MultiFunPlayer instead) and the "fire when the machine's speed crosses a threshold"
  rule.

## Running the engine from this fork

```
cargo build --release -p intiface-engine
target/release/intiface-engine --websocket-port 12345 --use-bluetooth-le --log info
```

MultiFunPlayer (Buttplug.io output, endpoint 127.0.0.1:12345) connects to it like to Intiface
Central. `tools/e2e/` has a hardware-free check and a fake S2 for testing clients; see its README.

## Two server behaviours worth knowing

* **Identical commands are dropped.** The server ignores an output command that equals the last
  one sent to the same feature (`device_handle.rs`, `output_cmd_hardware_commands`). So "spray =
  1" twice in a row squirts once: send 1, then 0 (0 does nothing on the launcher), then 1. The
  same applies to re-selecting the preset mode that was sent last (send 0 first). MultiFunPlayer in
  the companion fork does this; other clients must too.
* **Spray is hidden from spec v3 clients.** `Spray` is not a spec v3 actuator type and
  Buttplug.Net rejects a whole device that lists one, so v3 clients do not get it. This also
  affects the one JoyHub device with a spray output (`joyhub.yml`, index 7): v3 clients lose that
  actuator, where before they lost the whole device.
