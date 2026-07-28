from __future__ import annotations

from functools import lru_cache
from typing import Any, Dict, List, Optional


# Ordered by practical preference for finishing/setup: doubles first for checkouts,
# high triples for scoring, bull where useful. Labels are UI-facing.
DARTS: List[Dict[str, Any]] = []
for field in range(1, 21):
    DARTS.append({"label": f"S{field}", "field": field, "ring": "single_outer", "multiplier": 1, "score": field})
for field in range(1, 21):
    DARTS.append({"label": f"D{field}", "field": field, "ring": "double", "multiplier": 2, "score": field * 2})
for field in range(1, 21):
    DARTS.append({"label": f"T{field}", "field": field, "ring": "triple", "multiplier": 3, "score": field * 3})
DARTS.extend([
    {"label": "SBull", "field": 25, "ring": "single_bull", "multiplier": 1, "score": 25},
    {"label": "DBull", "field": 25, "ring": "double_bull", "multiplier": 2, "score": 50},
])

# Preferred first darts when multiple checkout paths exist. This keeps suggestions
# dart-plausible rather than mathematically arbitrary.
PREFERENCE = {
    "T20": 1000, "T19": 990, "T18": 980, "T17": 970, "T16": 960, "T15": 950,
    "DBull": 940, "T14": 930, "T13": 920, "T12": 910, "D20": 880, "D18": 870,
    "D16": 860, "D12": 850, "D10": 840, "D8": 830, "D4": 820, "D2": 810, "D1": 800,
}

GOOD_LEAVES_DOUBLE = [170, 167, 164, 161, 160, 158, 157, 156, 155, 154, 153, 152, 151, 150,
                      149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136,
                      135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122,
                      121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109, 108,
                      107, 106, 105, 104, 103, 102, 101, 100, 98, 96, 94, 92, 90, 88, 86,
                      84, 82, 80, 78, 76, 74, 72, 70, 68, 66, 64, 62, 60, 58, 56, 54, 52,
                      50, 48, 46, 44, 42, 40, 38, 36, 34, 32, 30, 28, 26, 24, 22, 20, 18,
                      16, 14, 12, 10, 8, 6, 4, 2]


def _is_double(dart: Dict[str, Any]) -> bool:
    return int(dart["multiplier"]) == 2


def _valid_finish(score: int, out_rule: str, dart: Dict[str, Any]) -> bool:
    if dart["score"] != score:
        return False
    return out_rule != "double" or _is_double(dart)


def _rank_sequence(seq: List[Dict[str, Any]], score: int) -> tuple:
    first = seq[0]["label"] if seq else ""
    # Prefer shorter routes, practical first dart, high first score, fewer bull dependencies.
    bull_count = sum(1 for dart in seq if dart["field"] == 25)
    return (len(seq), -PREFERENCE.get(first, 0), -seq[0]["score"], bull_count, first)


@lru_cache(maxsize=2048)
def checkout_sequence(score: int, darts_left: int, out_rule: str = "double") -> Optional[tuple]:
    if score <= 0 or darts_left <= 0:
        return None
    if out_rule == "double" and score == 1:
        return None
    candidates: List[List[Dict[str, Any]]] = []
    for dart in DARTS:
        value = int(dart["score"])
        if value > score:
            continue
        if _valid_finish(score, out_rule, dart):
            candidates.append([dart])
            continue
        if darts_left > 1:
            rest = checkout_sequence(score - value, darts_left - 1, out_rule)
            if rest:
                candidates.append([dart, *list(rest)])
    if not candidates:
        return None
    best = sorted(candidates, key=lambda seq: _rank_sequence(seq, score))[0]
    return tuple(best)


def best_setup(score: int, darts_left: int, out_rule: str = "double") -> Optional[Dict[str, Any]]:
    if darts_left <= 0 or score <= 1:
        return None
    options = []
    for dart in DARTS:
        value = int(dart["score"])
        leave = score - value
        if leave <= 1:
            continue
        # Can the remaining darts in this turn finish after hitting this setup dart?
        remaining_checkout = checkout_sequence(leave, darts_left - 1, out_rule) if darts_left > 1 else None
        # Can the next turn finish from that leave? This is the key setup quality
        # for scores that are not finishable in the current visit, e.g. 171 -> T20 leaves 111.
        next_turn_checkout = checkout_sequence(leave, 3, out_rule)
        good_index = GOOD_LEAVES_DOUBLE.index(leave) if out_rule == "double" and leave in GOOD_LEAVES_DOUBLE else 999
        if remaining_checkout or next_turn_checkout or good_index < 999:
            options.append({
                "target": dart,
                "leave": leave,
                "remaining_checkout": list(remaining_checkout) if remaining_checkout else [],
                "next_turn_checkout": list(next_turn_checkout) if next_turn_checkout else [],
                "good_leave_rank": good_index,
                "first_preference": PREFERENCE.get(dart["label"], 0),
            })
    if not options:
        return None
    options.sort(key=lambda item: (
        0 if item["remaining_checkout"] else 1,
        0 if item["next_turn_checkout"] else 1,
        -item["first_preference"],
        -item["target"]["score"],
        item["good_leave_rank"],
    ))
    return options[0]


def x01_advice(score: int, darts_left: int, out_rule: str = "double") -> Dict[str, Any]:
    darts_left = max(0, min(3, int(darts_left)))
    score = int(score)
    out_rule = out_rule or "straight"
    result: Dict[str, Any] = {
        "type": "x01_advice",
        "score": score,
        "darts_left": darts_left,
        "out_rule": out_rule,
        "status": "none",
        "message": "",
        "primary": None,
        "sequence": [],
        "setup": None,
    }
    if score <= 0 or darts_left <= 0:
        return result
    checkout = checkout_sequence(score, darts_left, out_rule)
    if checkout:
        seq = list(checkout)
        result.update({
            "status": "checkout",
            "message": "Finish möglich",
            "primary": seq[0],
            "sequence": seq,
        })
        return result
    setup = best_setup(score, darts_left, out_rule)
    if setup:
        target = setup["target"]
        leave = setup["leave"]
        follow = setup.get("remaining_checkout") or []
        next_turn = setup.get("next_turn_checkout") or []
        message = f"Stellen: {target['label']} lässt {leave}"
        if follow:
            message += " – danach " + " · ".join(dart["label"] for dart in follow)
        elif next_turn:
            message += " – nächster Turn " + " · ".join(dart["label"] for dart in next_turn)
        result.update({
            "status": "setup",
            "message": message,
            "primary": target,
            "setup": setup,
        })
        return result
    # Fallback: score as much as possible without busting or leaving 1.
    safe = [dart for dart in DARTS if 1 < score - dart["score"]]
    if safe:
        safe.sort(key=lambda dart: (-PREFERENCE.get(dart["label"], 0), -dart["score"]))
        target = safe[0]
        result.update({
            "status": "score_down",
            "message": f"Runterspielen: {target['label']} lässt {score - target['score']}",
            "primary": target,
            "setup": {"target": target, "leave": score - target["score"], "remaining_checkout": []},
        })
    return result
