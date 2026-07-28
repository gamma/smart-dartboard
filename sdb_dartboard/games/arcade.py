from __future__ import annotations

import random
from typing import Any, Dict, Iterable, List

from .x01_advisor import DARTS

TARGET_POOL_BASIC = [d for d in DARTS if d["field"] != 25 and d["ring"] in {"single_outer", "single_inner"}]
TARGET_POOL_NORMAL = [d for d in DARTS if d["label"] not in {"SBull"}]
TARGET_POOL_HARD = [d for d in DARTS if d["ring"] in {"triple", "double", "double_bull"}]


def zone_id(dart: Dict[str, Any]) -> str:
    label = str(dart["label"])
    if label == "SBull":
        return "SBULL"
    if label == "DBull":
        return "DBULL"
    return label


def same_target(event: Dict[str, Any], target: Dict[str, Any]) -> bool:
    return int(event.get("field", 0) or 0) == int(target.get("field", 0)) and str(event.get("ring")) == str(target.get("ring"))


def same_field(event: Dict[str, Any], target: Dict[str, Any]) -> bool:
    return int(event.get("field", 0) or 0) == int(target.get("field", 0))


def choose_targets(count: int, difficulty: str = "normal", exclude: Iterable[str] = ()) -> List[Dict[str, Any]]:
    pool = {"easy": TARGET_POOL_BASIC, "normal": TARGET_POOL_NORMAL, "hard": TARGET_POOL_HARD}.get(difficulty, TARGET_POOL_NORMAL)
    excluded = set(exclude)
    available = [d for d in pool if zone_id(d) not in excluded]
    if len(available) < count:
        available = [d for d in TARGET_POOL_NORMAL if zone_id(d) not in excluded]
    return random.sample(available, min(count, len(available)))


def overlay_item(dart: Dict[str, Any], color: str, label: str = "", pulse: bool = True) -> Dict[str, Any]:
    return {"id": zone_id(dart), "field": dart["field"], "ring": dart["ring"], "color": color, "label": label, "pulse": pulse}
