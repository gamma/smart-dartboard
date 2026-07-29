from __future__ import annotations

from typing import Any, Dict, Literal, Optional
from urllib.parse import urlsplit

from pydantic import BaseModel, ConfigDict, Field, model_validator


RING_MULTIPLIERS = {
    "single_inner": 1,
    "single_outer": 1,
    "triple": 3,
    "double": 2,
    "single_bull": 1,
    "double_bull": 2,
}


def same_origin(origin: Optional[str], host: Optional[str]) -> bool:
    """Allow native clients without Origin and same-host browser clients."""
    if not origin:
        return True
    try:
        return bool(host) and urlsplit(origin).netloc.lower() == host.lower()
    except ValueError:
        return False


class DartEventRequest(BaseModel):
    """Canonical dart input; score and label are never trusted from clients."""

    model_config = ConfigDict(extra="forbid")
    type: Literal["hit", "miss"]
    seq: int = Field(default=0, ge=0, le=9_007_199_254_740_991)
    field: Optional[int] = Field(default=None, ge=1, le=25)
    ring: Optional[
        Literal[
            "single_inner",
            "single_outer",
            "triple",
            "double",
            "single_bull",
            "double_bull",
        ]
    ] = None
    multiplier: Optional[int] = Field(default=None, ge=1, le=3)
    label: Optional[str] = Field(default=None, max_length=16)
    score: Optional[int] = Field(default=None, ge=0, le=60)

    @model_validator(mode="after")
    def validate_geometry(self):
        if self.type == "miss":
            return self
        if self.field is None or self.ring is None:
            raise ValueError("A hit needs field and ring")
        is_bull = self.ring in {"single_bull", "double_bull"}
        if is_bull != (self.field == 25):
            raise ValueError("Bull rings require field 25; number rings require 1–20")
        expected = RING_MULTIPLIERS[self.ring]
        if self.multiplier is not None and self.multiplier != expected:
            raise ValueError("Multiplier does not match ring")
        return self

    def normalized(self) -> Dict[str, Any]:
        if self.type == "miss":
            return {"type": "miss", "seq": self.seq, "label": "MISS", "score": 0}
        multiplier = RING_MULTIPLIERS[str(self.ring)]
        if self.field == 25:
            label = "DBull" if multiplier == 2 else "SBull"
        else:
            prefix = {1: "S", 2: "D", 3: "T"}[multiplier]
            label = f"{prefix}{self.field}"
        return {
            "type": "hit",
            "seq": self.seq,
            "field": self.field,
            "ring": self.ring,
            "multiplier": multiplier,
            "label": label,
            "score": int(self.field or 0) * multiplier,
        }
