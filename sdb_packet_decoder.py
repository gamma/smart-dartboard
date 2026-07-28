#!/usr/bin/env python3
"""
SDB-BT Dartboard FFF1 packet decoder.

Known packet format from LightBlue logs:
  10 bytes total
  bytes 0..3: sequence counter, little endian
  bytes 4..5: event/type, observed 0x0005
  bytes 6..8: hit code = ring, ones_digit, tens_digit
  byte 9: checksum = (ring + ones_digit + tens_digit) & 0xff
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from typing import Optional, Dict, Any


RING_MAP = {
    0x0A: ("single_inner", 1, "S"),
    0x0B: ("triple", 3, "T"),
    0x0C: ("single_outer", 1, "S"),
    0x0D: ("double", 2, "D"),
}


def normalize_hex(s: str) -> bytes:
    """Accept LightBlue style '<24000000 05000c01 000d>' or plain hex."""
    s = s.strip().replace("<", "").replace(">", "")
    s = re.sub(r"[^0-9a-fA-F]", "", s)
    if len(s) % 2:
        raise ValueError(f"Odd hex length: {len(s)}")
    return bytes.fromhex(s)


def decode_sdb_packet(data: bytes) -> Dict[str, Any]:
    if len(data) != 10:
        return {"type": "invalid_length", "length": len(data), "raw": data.hex()}

    seq = int.from_bytes(data[0:4], "little")
    event_type = int.from_bytes(data[4:6], "little")
    ring = data[6]
    ones = data[7]
    tens = data[8]
    checksum = data[9]
    expected_checksum = (ring + ones + tens) & 0xFF

    base = {
        "seq": seq,
        "event_type": event_type,
        "raw": data.hex(),
        "code": f"{ring:02x} {ones:02x} {tens:02x}",
        "checksum": checksum,
        "checksum_ok": checksum == expected_checksum,
    }

    if checksum != expected_checksum:
        return {"type": "checksum_error", **base, "expected_checksum": expected_checksum}

    # observed menu / next-player button codes and neutral/miss code.
    # Note: 00 00 ee is context-dependent: it is button release after a button press,
    # but it is also emitted for a miss/no-score event when no button is active.
    if (ring, ones, tens) == (0x00, 0x00, 0xFF):
        return {"type": "button", "button": "menu", "action": "press", **base}
    if (ring, ones, tens) == (0x00, 0x00, 0xEE):
        return {"type": "neutral", "meaning": "miss_or_button_release", **base}
    if (ring, ones, tens) == (0x00, 0x00, 0xCC):
        return {"type": "button", "button": "menu", "action": "long_press", **base}

    # observed bull codes
    if (ring, ones, tens) == (0x0C, 0x00, 0x0E):
        return {"type": "hit", "field": 25, "ring": "single_bull", "multiplier": 1, "label": "SBull", "score": 25, **base}
    if (ring, ones, tens) == (0x0D, 0x00, 0x0F):
        return {"type": "hit", "field": 25, "ring": "double_bull", "multiplier": 2, "label": "DBull", "score": 50, **base}

    field = ones + 10 * tens
    if ring in RING_MAP and 1 <= field <= 20:
        ring_name, multiplier, prefix = RING_MAP[ring]
        return {
            "type": "hit",
            "field": field,
            "ring": ring_name,
            "multiplier": multiplier,
            "label": f"{prefix}{field}",
            "score": field * multiplier,
            **base,
        }

    return {"type": "unknown", **base}


def main() -> int:
    if len(sys.argv) > 1:
        inputs = sys.argv[1:]
    else:
        inputs = [line.strip() for line in sys.stdin if line.strip()]

    for item in inputs:
        data = normalize_hex(item)
        print(decode_sdb_packet(data))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
