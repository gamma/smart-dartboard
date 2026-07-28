#!/usr/bin/env python3
from __future__ import annotations

import asyncio
import logging

from bleak import BleakScanner


async def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(message)s")
    print("Scanning for BLE devices...", flush=True)
    devices = await BleakScanner.discover(timeout=8.0, return_adv=True)
    for device, adv in devices.values():
        name = device.name or adv.local_name or ""
        uuids = ",".join(adv.service_uuids or [])
        print(f"address={device.address} name={name!r} rssi={adv.rssi} uuids={uuids}")


if __name__ == "__main__":
    asyncio.run(main())
