#!/usr/bin/env python3
from __future__ import annotations

import asyncio
import argparse
from bleak import BleakClient, BleakScanner


async def find(name: str):
    devices = await BleakScanner.discover(timeout=8.0, return_adv=True)
    for device, adv in devices.values():
        if device.name == name or adv.local_name == name:
            return device.address
    raise SystemExit(f"Device not found: {name}")


async def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", default="SDB-BT")
    ap.add_argument("--address")
    args = ap.parse_args()

    address = args.address or await find(args.name)
    print(f"Connecting to {address}...")
    async with BleakClient(address) as client:
        for service in client.services:
            print(f"Service {service.uuid}")
            for ch in service.characteristics:
                props = ",".join(ch.properties)
                print(f"  Characteristic {ch.uuid} properties={props}")


if __name__ == "__main__":
    asyncio.run(main())
