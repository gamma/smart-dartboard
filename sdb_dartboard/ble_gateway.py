"""Linux BLE sidecar for the Rust runtime.

The gateway deliberately does not decode dartboard packets. It owns only the
platform BLE connection and forwards raw, bounded FFF1 notifications to the
authenticated Rust ingress.
"""

from __future__ import annotations

import argparse
import asyncio
from dataclasses import dataclass
import json
import logging
import os
import random
import urllib.error
import urllib.request
import uuid
from typing import Any

LOG = logging.getLogger(__name__)

DEFAULT_DEVICE_NAME = "SDB-BT"
SERVICE_UUID = "0000fff0-0000-1000-8000-00805f9b34fb"
NOTIFY_UUID = "0000fff1-0000-1000-8000-00805f9b34fb"


@dataclass(frozen=True)
class GatewayConfig:
    runtime_url: str
    token: str
    device_name: str = DEFAULT_DEVICE_NAME
    device_address: str | None = None
    queue_size: int = 256
    scan_timeout: float = 8.0

    @classmethod
    def from_environment(cls) -> "GatewayConfig":
        token = os.environ.get("SDB_BOARD_TOKEN", "").strip()
        if not token:
            raise ValueError("SDB_BOARD_TOKEN is required")
        queue_size = int(os.environ.get("SDB_BLE_QUEUE_SIZE", "256"))
        if not 1 <= queue_size <= 4096:
            raise ValueError("SDB_BLE_QUEUE_SIZE must be between 1 and 4096")
        return cls(
            runtime_url=os.environ.get("SDB_RUNTIME_URL", "http://127.0.0.1:8000")
            .strip()
            .rstrip("/"),
            token=token,
            device_name=os.environ.get("SDB_DEVICE_NAME", DEFAULT_DEVICE_NAME).strip(),
            device_address=os.environ.get("SDB_DEVICE_ADDRESS", "").strip() or None,
            queue_size=queue_size,
        )


@dataclass(frozen=True)
class RawNotification:
    connection_id: str
    payload: bytes


class NotificationBuffer:
    """Bounded handoff from Bleak's callback into the HTTP sender."""

    def __init__(self, capacity: int) -> None:
        self.queue: asyncio.Queue[RawNotification] = asyncio.Queue(maxsize=capacity)
        self.overflowed = asyncio.Event()

    def put(self, connection_id: str, payload: bytes) -> bool:
        try:
            self.queue.put_nowait(RawNotification(connection_id, bytes(payload)))
        except asyncio.QueueFull:
            self.overflowed.set()
            return False
        return True


class GatewayError(RuntimeError):
    def __init__(self, failure_code: str, message: str) -> None:
        super().__init__(message)
        self.failure_code = failure_code


class RuntimeIngressClient:
    def __init__(self, base_url: str, token: str) -> None:
        self.base_url = base_url
        self.token = token

    async def status(
        self,
        phase: str,
        *,
        connection_id: str | None = None,
        failure_code: str | None = None,
        detail: str | None = None,
    ) -> None:
        payload: dict[str, Any] = {"phase": phase}
        if connection_id:
            payload["connection_id"] = connection_id
        if failure_code:
            payload["failure_code"] = failure_code
        if detail:
            payload["detail"] = detail[:256]
        await self._post("/api/v2/board/status", payload)

    async def packet(self, notification: RawNotification) -> dict[str, Any]:
        return await self._post(
            "/api/v2/board/packets",
            {
                "connection_id": notification.connection_id,
                "raw_hex": notification.payload.hex(),
            },
        )

    async def _post(self, path: str, payload: dict[str, Any]) -> dict[str, Any]:
        return await asyncio.to_thread(self._post_sync, path, payload)

    def _post_sync(self, path: str, payload: dict[str, Any]) -> dict[str, Any]:
        body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
        request = urllib.request.Request(
            f"{self.base_url}{path}",
            data=body,
            method="POST",
            headers={
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/json",
            },
        )
        with urllib.request.urlopen(request, timeout=5.0) as response:
            return json.load(response)


class BleGateway:
    def __init__(self, config: GatewayConfig) -> None:
        self.config = config
        self.runtime = RuntimeIngressClient(config.runtime_url, config.token)
        self.buffer = NotificationBuffer(config.queue_size)
        self.stop_requested = asyncio.Event()
        self._known_address_failed = False

    async def run(self) -> None:
        sender = asyncio.create_task(self._send_notifications())
        backoff = 1.0
        try:
            while not self.stop_requested.is_set():
                try:
                    await self._connect_once()
                    backoff = 1.0
                except asyncio.CancelledError:
                    raise
                except FileNotFoundError as error:
                    await self._report_failure(
                        "unavailable", "adapter_unavailable", str(error)
                    )
                    backoff = max(backoff, 30.0)
                except GatewayError as error:
                    LOG.warning("BLE gateway error: %s", error)
                    await self._report_failure(
                        "reconnecting", error.failure_code, str(error)
                    )
                except Exception as error:  # BLE backends expose platform errors.
                    LOG.warning("BLE connection failed: %s", error)
                    await self._report_failure(
                        "reconnecting", "connection_failed", str(error)
                    )
                if not self.stop_requested.is_set():
                    delay = min(backoff, 30.0) + random.uniform(0.0, 0.25)
                    await asyncio.sleep(delay)
                    backoff = min(backoff * 2.0, 30.0)
        finally:
            sender.cancel()
            await asyncio.gather(sender, return_exceptions=True)

    async def stop(self) -> None:
        self.stop_requested.set()

    async def _connect_once(self) -> None:
        from bleak import BleakClient

        await self._best_effort_status("scanning")
        device = await self._find_device()
        connection_id = uuid.uuid4().hex
        await self._best_effort_status("connecting", connection_id=connection_id)
        try:
            async with BleakClient(device) as client:
                if not client.is_connected:
                    raise RuntimeError("BLE client did not reach connected state")
                await self._best_effort_status("discovering", connection_id=connection_id)
                services = client.services
                if services.get_service(SERVICE_UUID) is None:
                    raise GatewayError(
                        "service_missing", f"required BLE service is missing: {SERVICE_UUID}"
                    )
                if services.get_characteristic(NOTIFY_UUID) is None:
                    raise GatewayError(
                        "characteristic_missing",
                        f"required BLE characteristic is missing: {NOTIFY_UUID}",
                    )
                await self._best_effort_status("subscribing", connection_id=connection_id)

                def on_notify(_sender: Any, data: bytearray) -> None:
                    if not self.buffer.put(connection_id, bytes(data)):
                        LOG.error(
                            "BLE notification queue overflow; disconnecting instead of silently dropping"
                        )

                await client.start_notify(NOTIFY_UUID, on_notify)
                await self.runtime.status("ready", connection_id=connection_id)
                self._known_address_failed = False
                LOG.info("Board ready on connection %s", connection_id)

                while (
                    client.is_connected
                    and not self.stop_requested.is_set()
                    and not self.buffer.overflowed.is_set()
                ):
                    await asyncio.sleep(0.25)

                await client.stop_notify(NOTIFY_UUID)
                await self.buffer.queue.join()
                if self.buffer.overflowed.is_set():
                    self.buffer.overflowed.clear()
                    raise GatewayError("queue_overflow", "notification queue overflow")
        except Exception:
            if self.config.device_address:
                self._known_address_failed = True
            raise

    async def _find_device(self) -> Any:
        from bleak import BleakScanner

        if self.config.device_address and not self._known_address_failed:
            return self.config.device_address
        devices = await BleakScanner.discover(
            timeout=self.config.scan_timeout, return_adv=True
        )
        service_uuid = SERVICE_UUID.lower()
        for device, advertisement in devices.values():
            names = {device.name, advertisement.local_name}
            advertised_services = {
                value.lower() for value in (advertisement.service_uuids or [])
            }
            if self.config.device_name in names or service_uuid in advertised_services:
                LOG.info("Found %s at %s", self.config.device_name, device.address)
                return device
        raise GatewayError("device_not_found", f"board not found: {self.config.device_name}")

    async def _send_notifications(self) -> None:
        while True:
            notification = await self.buffer.queue.get()
            delay = 0.25
            try:
                while True:
                    try:
                        response = await self.runtime.packet(notification)
                        LOG.debug("Runtime packet disposition: %s", response.get("disposition"))
                        break
                    except asyncio.CancelledError:
                        raise
                    except urllib.error.HTTPError as error:
                        if error.code == 400:
                            await self._best_effort_status(
                                "ready", connection_id=notification.connection_id
                            )
                        else:
                            LOG.error("Runtime rejected BLE gateway credentials: HTTP %s", error.code)
                        await asyncio.sleep(delay)
                        delay = min(delay * 2.0, 5.0)
                    except (OSError, urllib.error.URLError) as error:
                        LOG.warning("Runtime ingress unavailable: %s", error)
                        await asyncio.sleep(delay)
                        delay = min(delay * 2.0, 5.0)
            finally:
                self.buffer.queue.task_done()

    async def _best_effort_status(self, phase: str, **values: Any) -> None:
        try:
            await self.runtime.status(phase, **values)
        except (OSError, urllib.error.URLError) as error:
            LOG.warning("Could not publish board status %s: %s", phase, error)

    async def _report_failure(self, phase: str, code: str, detail: str) -> None:
        await self._best_effort_status(phase, failure_code=code, detail=detail)


def main() -> None:
    parser = argparse.ArgumentParser(description="Smart Dartboard BLE gateway")
    parser.add_argument("--log-level", default=os.environ.get("SDB_LOG_LEVEL", "INFO"))
    args = parser.parse_args()
    logging.basicConfig(
        level=getattr(logging, args.log_level.upper(), logging.INFO),
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    )
    try:
        config = GatewayConfig.from_environment()
    except (TypeError, ValueError) as error:
        parser.error(str(error))
    try:
        asyncio.run(BleGateway(config).run())
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
