from __future__ import annotations

import asyncio
import json
import logging
from typing import Callable, Awaitable, Optional, Any, Dict

from bleak import BleakClient, BleakScanner

from .protocol import decode_packet
from .interpreter import EventInterpreter

LOG = logging.getLogger(__name__)

DEVICE_NAME = "SDB-BT"
SERVICE_UUID = "0000fff0-0000-1000-8000-00805f9b34fb"
NOTIFY_UUID = "0000fff1-0000-1000-8000-00805f9b34fb"

EventHandler = Callable[[Dict[str, Any]], Awaitable[None]]


class SdbDartboardClient:
    def __init__(
        self,
        name: str = DEVICE_NAME,
        address: Optional[str] = None,
        notify_uuid: str = NOTIFY_UUID,
        reconnect_delay: float = 2.0,
    ) -> None:
        self.name = name
        self.address = address
        self.notify_uuid = notify_uuid
        self.reconnect_delay = reconnect_delay
        self.interpreter = EventInterpreter()
        self._stop = asyncio.Event()
        self._handler: Optional[EventHandler] = None

    async def find_device(self):
        if self.address:
            return self.address
        LOG.info("Scanning for BLE device named %s ...", self.name)
        devices = await BleakScanner.discover(timeout=8.0, return_adv=True)
        for device, adv in devices.values():
            if device.name == self.name or adv.local_name == self.name:
                LOG.info("Found %s at %s", self.name, device.address)
                return device.address
        raise RuntimeError(f"Device not found: {self.name}")

    async def run(self, handler: EventHandler) -> None:
        self._handler = handler
        while not self._stop.is_set():
            try:
                address = await self.find_device()
                await self._connect_once(address)
            except asyncio.CancelledError:
                raise
            except Exception as exc:
                LOG.warning("BLE connection loop error: %s", exc)
                await asyncio.sleep(self.reconnect_delay)

    async def stop(self) -> None:
        self._stop.set()

    async def _connect_once(self, address: str) -> None:
        LOG.info("Connecting to %s ...", address)
        async with BleakClient(address) as client:
            if not client.is_connected:
                raise RuntimeError("BLE connection failed")
            LOG.info("Connected; subscribing to %s", self.notify_uuid)

            queue: asyncio.Queue[Dict[str, Any]] = asyncio.Queue()

            def on_notify(sender, data: bytearray):
                decoded = decode_packet(bytes(data))
                event = self.interpreter.interpret(decoded)
                queue.put_nowait(event)

            await client.start_notify(self.notify_uuid, on_notify)
            LOG.info("Subscribed. Waiting for dartboard events.")

            while client.is_connected and not self._stop.is_set():
                try:
                    event = await asyncio.wait_for(queue.get(), timeout=0.5)
                except asyncio.TimeoutError:
                    continue
                if self._handler:
                    await self._handler(event)

            try:
                await client.stop_notify(self.notify_uuid)
            except Exception:
                pass
            LOG.info("Disconnected")


async def print_event(event: Dict[str, Any]) -> None:
    print(json.dumps(event, ensure_ascii=False), flush=True)
