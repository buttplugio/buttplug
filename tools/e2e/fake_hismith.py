#!/usr/bin/env python3
"""A fake HISMITH S2 (or Lube Launcher) for the Buttplug "websocket device" hardware manager.

Start intiface_engine (or any Buttplug server) with the device websocket server and the user
config from this folder, then run this. Every frame the server writes to the device is printed as
hex, one per line, e.g. "CC 03 32 35". Needs: pip install websockets

    intiface_engine --websocket-port 12345 --use-device-websocket-server \
        --user-device-config-file tools/e2e/user-config-websocket.json
    python tools/e2e/fake_hismith.py            # HISMITH S2, model 1101
    python tools/e2e/fake_hismith.py --piupiu   # Lube Launcher, model 4002
"""
import argparse
import asyncio
import json
import sys
import time

import websockets

DEVICES = {
    "s2": {"identifier": "HISMITH S2", "address": "fake-hismith-s2", "model": "1101"},
    "piupiu": {"identifier": "Hismith Piupiu", "address": "fake-hismith-piupiu", "model": "4002"},
}


async def run(url, kind, out):
    dev = DEVICES[kind]
    handshake = {"identifier": dev["identifier"], "address": dev["address"], "version": 1,
                 "reads": {"rxblemodel": dev["model"]}}
    async with websockets.connect(url, ping_interval=None) as ws:
        await ws.send(json.dumps(handshake))
        print(f"# connected as {dev['identifier']} (model {dev['model']})", file=sys.stderr, flush=True)
        start = time.monotonic()
        async for frame in ws:
            if isinstance(frame, (bytes, bytearray)):
                line = " ".join(f"{b:02X}" for b in frame)
                print(f"{time.monotonic() - start:8.3f} {line}", file=out, flush=True)
            else:
                print(f"# text frame: {frame!r}", file=sys.stderr, flush=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--url", default="ws://127.0.0.1:54817")
    ap.add_argument("--piupiu", action="store_true", help="be the Lube Launcher instead of the S2")
    ap.add_argument("--out", help="also write the frames to this file")
    a = ap.parse_args()
    out = open(a.out, "w", encoding="utf-8") if a.out else sys.stdout
    try:
        asyncio.run(run(a.url, "piupiu" if a.piupiu else "s2", out))
    except KeyboardInterrupt:
        pass
    except websockets.ConnectionClosed:
        print("# server closed the connection", file=sys.stderr)


if __name__ == "__main__":
    main()
