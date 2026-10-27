# End-to-end check without hardware

> The stock HISMITH S2 definition has five features: speed (Oscillate, index 0), depth (Position,
> 1; 0..100 slides a stroke window), stroke width (Position, 2; clamped to a minimum of 10), run
> mode (Position, 3; 0 auto / 1 manual) and preset mode (Position, 4; 0..8). `v3_client_check.py`
> checks exactly these.

`run_e2e.sh` builds `intiface_engine` from this fork, starts it with the **websocket device**
hardware manager, connects a fake HISMITH S2 (then a fake Lube Launcher) to it, and drives the
server with a spec v3 client that sends the same JSON MultiFunPlayer / Buttplug.Net sends. The
fake device prints every frame the server writes; the client compares them with the expected
`CC ..` bytes (speed, stroke window, window width, run mode, preset modes, stop, spray,
de-duplication and the 100 ms spray rate limit).

```
pip install websockets
tools/e2e/run_e2e.sh            # exit code 0 = all expectations met; details in /tmp/hismith-e2e
```

The fake device works because this fork's websocket device manager accepts an optional `reads`
map in the handshake (`{"identifier": "HISMITH S2", "address": "...", "version": 1,
"reads": {"rxblemodel": "1101"}}`): the protocol identifies a Hismith by reading its model
characteristic, and the stock manager cannot read. Without `reads` nothing changes.

## Testing MultiFunPlayer against the fake S2 (Windows, no machine needed)

```
intiface_engine.exe --websocket-port 12345 --use-device-websocket-server --user-device-config-file tools\e2e\user-config-websocket.json --log info
python tools\e2e\fake_hismith.py              (or --piupiu for the Lube Launcher)
```

MultiFunPlayer then sees "Hismith Servo Sex Machine" with all seven actuators; every slider,
key press, sequence step and voice command shows up as bytes in the fake device's window.

## Last recorded run (Linux, this fork, no hardware)

`run_e2e.sh`: **PASS** for both fake devices, 18/18 byte checks:

* S2: `CC 03 32 35`, `CC 07 14 1B`, `CC 08 50 58`, `CC 0A 32 3C`, `CC 09 05 0E`, `CC 01 A1 A2`,
  `CC 01 A0 A1`, `CC 04 03 07`; identical preset repeated -> nothing; `0` then `3` -> `CC 04 03 07`;
  Stop -> only `CC 03 00 03`. Engine log: `Hismith Device Identifier: 1101`.
* Lube Launcher: `1` -> `CC 0B 01 0C`; `1` again -> nothing (server de-duplication); `0` -> nothing;
  `1` -> second shot; `1,0,1` inside 100 ms -> one shot (rate limit); Stop -> nothing.
* The live `DeviceAdded` of both devices parses with the Buttplug.Net client library MultiFunPlayer
  uses (7 and 1 scalar actuators).

## Last recorded run (Linux, this fork, no hardware)

`run_e2e.sh`: **PASS** for both fake devices.

* S2 (5 features): `CC 03 32 35`; depth 1.0 -> `CC 08 64 6C`, `CC 07 3C 43`; depth 0.0 -> `CC 07 00 07`,
  `CC 08 28 30`; width 0 (clamped to 10) -> `CC 08 0A 12`, `CC 07 00 07`; width 100 -> `CC 08 64 6C`,
  `CC 07 00 07`; width 40 -> `CC 08 28 30`, `CC 07 00 07`; depth 0.5 at width 40 -> `CC 08 46 4E`,
  `CC 07 1E 25`; run mode `CC 01 A1 A2` / `CC 01 A0 A1`; preset 3 -> `CC 04 03 07` (identical repeat and
  `0` send nothing); Stop -> only `CC 03 00 03`.
* Lube Launcher unchanged: `1` -> `CC 0B 01 0C`; repeats and `0` -> nothing; `1,0,1` inside 100 ms -> one shot.
* The live `DeviceAdded` of the S2 (5 scalar actuators) parses with Buttplug.Net.
