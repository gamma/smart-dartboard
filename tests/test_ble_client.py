from __future__ import annotations

import importlib.util
import unittest
from unittest.mock import AsyncMock, patch

BLEAK_AVAILABLE = importlib.util.find_spec("bleak") is not None
if BLEAK_AVAILABLE:
    from sdb_dartboard.client import SdbDartboardClient


@unittest.skipUnless(BLEAK_AVAILABLE, "BLE client tests require project dependencies")
class BleClientTests(unittest.IsolatedAsyncioTestCase):
    async def test_missing_backend_uses_clear_warning_and_slow_retry(self) -> None:
        client = SdbDartboardClient(reconnect_delay=2.0)
        statuses = []

        async def handle_event(_event):
            return None

        async def handle_status(status):
            statuses.append(status)

        async def stop_after_delay(delay: float) -> None:
            self.assertEqual(delay, 30.0)
            client._stop.set()

        with (
            patch.object(
                client,
                "find_device",
                new=AsyncMock(side_effect=FileNotFoundError(2, "missing")),
            ),
            patch(
                "sdb_dartboard.client.asyncio.sleep",
                new=AsyncMock(side_effect=stop_after_delay),
            ),
            self.assertLogs("sdb_dartboard.client", level="WARNING") as logs,
        ):
            await client.run(handle_event, handle_status)

        self.assertEqual([status["status"] for status in statuses], ["searching", "error"])
        self.assertIn("BlueZ D-Bus socket not found", logs.output[0])


if __name__ == "__main__":
    unittest.main()
