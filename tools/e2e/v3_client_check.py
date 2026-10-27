#!/usr/bin/env python3
"""Spec v3 client (the messages Buttplug.Net / MultiFunPlayer send) that drives the fake device and
checks the bytes it received. Exit code 0 = every expectation met.

    python tools/e2e/v3_client_check.py --frames frames.txt            # S2
    python tools/e2e/v3_client_check.py --frames frames.txt --piupiu   # Lube Launcher
"""
import argparse
import asyncio
import json
import sys
import time

import websockets

PRESET_MODE_MAX = 8


class V3Client:
    def __init__(self, url):
        self.url, self.ws, self.msg_id, self.devices = url, None, 1, {}

    async def __aenter__(self):
        self.ws = await websockets.connect(self.url, ping_interval=None)
        return self

    async def __aexit__(self, *a):
        await self.ws.close()

    async def send(self, name, body, expect="Ok", timeout=5.0):
        body = dict(body, Id=self.msg_id)
        self.msg_id += 1
        await self.ws.send(json.dumps([{name: body}]))
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            raw = await asyncio.wait_for(self.ws.recv(), timeout=max(0.01, deadline - time.monotonic()))
            for msg in json.loads(raw):
                (kind, payload), = msg.items()
                if kind == "DeviceAdded":
                    self.devices[payload["DeviceIndex"]] = payload
                elif kind == "DeviceList":
                    for d in payload["Devices"]:
                        self.devices[d["DeviceIndex"]] = d
                if payload.get("Id") == body["Id"]:
                    if kind == "Error":
                        raise RuntimeError(f"{name}: {payload}")
                    if expect and kind != expect:
                        raise RuntimeError(f"{name}: expected {expect}, got {kind}")
                    return payload
        raise TimeoutError(name)

    async def wait_for_device(self, timeout=15.0):
        deadline = time.monotonic() + timeout
        while not self.devices and time.monotonic() < deadline:
            try:
                raw = await asyncio.wait_for(self.ws.recv(), timeout=0.5)
            except asyncio.TimeoutError:
                continue
            for msg in json.loads(raw):
                (kind, payload), = msg.items()
                if kind == "DeviceAdded":
                    self.devices[payload["DeviceIndex"]] = payload
        if not self.devices:
            raise TimeoutError("no DeviceAdded")
        return next(iter(self.devices.values()))

    async def scalar(self, device_index, index, value, actuator_type):
        await self.send("ScalarCmd", {"DeviceIndex": device_index,
                                      "Scalars": [{"Index": index, "Scalar": value, "ActuatorType": actuator_type}]})

    async def stop(self, device_index):
        await self.send("StopDeviceCmd", {"DeviceIndex": device_index})


def read_frames(path):
    out = []
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        t, _, hexes = line.partition(" ")
        out.append((float(t), hexes.strip()))
    return out


async def settle(frames_path, previous_count, wait=0.4):
    await asyncio.sleep(wait)
    return read_frames(frames_path)[previous_count:]


async def run_s2(c, dev, frames_path):
    idx = dev["DeviceIndex"]
    actuators = dev["DeviceMessages"]["ScalarCmd"]
    results = []

    async def step(label, send, expected):
        before = len(read_frames(frames_path))
        await send()
        got = [h for _, h in await settle(frames_path, before)]
        results.append((label, expected, got))

    # Two-slider layout: 0 speed (Oscillate), 1 depth, 2 stroke width (0..100, minimum 10), 3 run mode, 4 preset mode.
    # Speed 50 % (step count 100) -> CC 03 32 35
    await step("speed 0.5", lambda: c.scalar(idx, 0, 0.5, "Oscillate"), ["CC 03 32 35"])
    await step("depth 1.0 (window 60..100, end first)", lambda: c.scalar(idx, 1, 1.0, "Position"), ["CC 08 64 6C", "CC 07 3C 43"])
    await step("depth 0.0 (window 0..40, start first)", lambda: c.scalar(idx, 1, 0.0, "Position"), ["CC 07 00 07", "CC 08 28 30"])
    await step("width 0 -> clamped to 10 at depth 0 (window 0..10)", lambda: c.scalar(idx, 2, 0.0, "Position"), ["CC 08 0A 12", "CC 07 00 07"])
    await step("width 100 at depth 0 (window 0..100)", lambda: c.scalar(idx, 2, 1.0, "Position"), ["CC 08 64 6C", "CC 07 00 07"])
    await step("width 40 again (window 0..40)", lambda: c.scalar(idx, 2, 0.4, "Position"), ["CC 08 28 30", "CC 07 00 07"])
    await step("depth 0.5 at width 40 (window 30..70)", lambda: c.scalar(idx, 1, 0.5, "Position"), ["CC 08 46 4E", "CC 07 1E 25"])
    await step("run mode manual", lambda: c.scalar(idx, 3, 1.0, "Position"), ["CC 01 A1 A2"])
    await step("run mode auto", lambda: c.scalar(idx, 3, 0.0, "Position"), ["CC 01 A0 A1"])
    await step("preset mode 3 (3/8)", lambda: c.scalar(idx, 4, 3 / PRESET_MODE_MAX, "Position"), ["CC 04 03 07"])
    await step("preset mode 3 again (server de-dupes identical commands)", lambda: c.scalar(idx, 4, 3 / PRESET_MODE_MAX, "Position"), [])
    await step("preset mode 0 (sends nothing)", lambda: c.scalar(idx, 4, 0.0, "Position"), [])
    await step("preset mode 3 after 0 (how MultiFunPlayer re-selects)", lambda: c.scalar(idx, 4, 3 / PRESET_MODE_MAX, "Position"), ["CC 04 03 07"])
    await step("stop (speed 0 only)", lambda: c.stop(idx), ["CC 03 00 03"])
    return actuators, results


async def run_piupiu(c, dev, frames_path):
    idx = dev["DeviceIndex"]
    actuators = dev["DeviceMessages"]["ScalarCmd"]
    results = []

    async def step(label, send, expected, wait=0.4):
        before = len(read_frames(frames_path))
        await send()
        got = [h for _, h in await settle(frames_path, before, wait)]
        results.append((label, expected, got))

    async def two_fast_shots():
        await c.scalar(idx, 0, 0.0, "Position")
        await c.scalar(idx, 0, 1.0, "Position")
        await c.scalar(idx, 0, 0.0, "Position")
        await c.scalar(idx, 0, 1.0, "Position")

    await step("spray 1", lambda: c.scalar(idx, 0, 1.0, "Position"), ["CC 0B 01 0C"])
    await step("spray 1 again (de-duplicated by the server: no shot)", lambda: c.scalar(idx, 0, 1.0, "Position"), [])
    await step("spray 0 (nothing)", lambda: c.scalar(idx, 0, 0.0, "Position"), [])
    await step("spray 1 after 0 (second shot)", lambda: c.scalar(idx, 0, 1.0, "Position"), ["CC 0B 01 0C"])
    await asyncio.sleep(0.2)
    await step("1,0,1 within 100 ms (rate limit: one shot)", two_fast_shots, ["CC 0B 01 0C"])
    await step("stop (must not squirt)", lambda: c.stop(idx), [])
    return actuators, results


async def main_async(a):
    async with V3Client(a.url) as c:
        info = await c.send("RequestServerInfo", {"ClientName": "e2e-check", "MessageVersion": 3}, expect="ServerInfo")
        assert info["MessageVersion"] == 3, info
        await c.send("RequestDeviceList", {}, expect="DeviceList")
        await c.send("StartScanning", {})
        dev = await c.wait_for_device()
        if a.device_added:
            with open(a.device_added, "w", encoding="utf-8") as f:
                f.write(json.dumps([{"DeviceAdded": dict(dev, Id=0)}]))
        actuators, results = await (run_piupiu if a.piupiu else run_s2)(c, dev, a.frames)

    print(f"device: {dev['DeviceName']}")
    for i, act in enumerate(actuators):
        print(f"  scalar {i}: {act['ActuatorType']}/{act['StepCount']} {act['FeatureDescriptor']}")
    failed = 0
    for label, expected, got in results:
        ok = expected == got
        failed += not ok
        print(f"  [{'OK' if ok else 'FAIL'}] {label}: expected {expected or 'nothing'}, got {got or 'nothing'}")
    print("RESULT:", "PASS" if not failed else f"FAIL ({failed})")
    return 0 if not failed else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--url", default="ws://127.0.0.1:12345")
    ap.add_argument("--frames", required=True, help="file the fake device writes its frames to")
    ap.add_argument("--piupiu", action="store_true")
    ap.add_argument("--device-added", help="write the DeviceAdded JSON here (for the Buttplug.Net parser check)")
    sys.exit(asyncio.run(main_async(ap.parse_args())))


if __name__ == "__main__":
    main()
