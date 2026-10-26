# End-to-end check without hardware

`run_e2e.sh` builds `intiface_engine` from this fork, starts it with the **websocket device**
hardware manager, connects a fake HISMITH S2 (then a fake Lube Launcher) to it, and drives the
server with a spec v3 client that sends the same JSON MultiFunPlayer / Buttplug.Net sends. The
fake device prints every frame the server writes; the client compares them with the expected
`CC ..` bytes (speed, stroke window, position, smoothness, run mode, preset modes, stop, spray,
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
