import asyncio
import os
import unittest
from unittest.mock import patch

from sdb_dartboard.ble_gateway import GatewayConfig, NotificationBuffer


class GatewayConfigTests(unittest.TestCase):
    def test_requires_an_ingress_token(self):
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(ValueError, "SDB_BOARD_TOKEN"):
                GatewayConfig.from_environment()

    def test_reads_bounded_configuration(self):
        with patch.dict(
            os.environ,
            {
                "SDB_BOARD_TOKEN": "secret",
                "SDB_RUNTIME_URL": "http://runtime:8000/",
                "SDB_BLE_QUEUE_SIZE": "32",
                "SDB_DEVICE_ADDRESS": "AA:BB:CC:DD:EE:FF",
            },
            clear=True,
        ):
            config = GatewayConfig.from_environment()
        self.assertEqual(config.runtime_url, "http://runtime:8000")
        self.assertEqual(config.queue_size, 32)
        self.assertEqual(config.device_address, "AA:BB:CC:DD:EE:FF")


class NotificationBufferTests(unittest.IsolatedAsyncioTestCase):
    async def test_copies_raw_packets_and_surfaces_overflow(self):
        buffer = NotificationBuffer(1)
        mutable = bytearray.fromhex("0100000005000d00020f")
        self.assertTrue(buffer.put("link", mutable))
        mutable[0] = 0xFF
        self.assertFalse(buffer.put("link", bytes(10)))
        self.assertTrue(buffer.overflowed.is_set())
        notification = await asyncio.wait_for(buffer.queue.get(), timeout=0.1)
        self.assertEqual(notification.connection_id, "link")
        self.assertEqual(notification.payload.hex(), "0100000005000d00020f")
        buffer.queue.task_done()


if __name__ == "__main__":
    unittest.main()
