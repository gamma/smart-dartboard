#!/usr/bin/env python3
from __future__ import annotations

import argparse
import asyncio
import json
import logging

from sdb_dartboard.client import SdbDartboardClient


async def main() -> None:
    ap = argparse.ArgumentParser(description="Live SDB-BT dartboard event logger")
    ap.add_argument("--name", default="SDB-BT", help="BLE device local name")
    ap.add_argument("--address", help="BLE device address, optional")
    ap.add_argument("--log-level", default="INFO")
    args = ap.parse_args()

    logging.basicConfig(level=getattr(logging, args.log_level.upper()), format="%(asctime)s %(levelname)s %(message)s")

    async def on_event(event):
        typ = event.get("type")
        if typ == "hit":
            print(f"HIT {event['label']} score={event['score']} seq={event['seq']}", flush=True)
        elif typ == "miss":
            print(f"MISS seq={event['seq']}", flush=True)
        elif typ == "button":
            print(f"BUTTON {event.get('button')} {event.get('action')} seq={event['seq']}", flush=True)
        else:
            print("EVENT " + json.dumps(event, ensure_ascii=False), flush=True)

    client = SdbDartboardClient(name=args.name, address=args.address)
    await client.run(on_event)


if __name__ == "__main__":
    asyncio.run(main())
