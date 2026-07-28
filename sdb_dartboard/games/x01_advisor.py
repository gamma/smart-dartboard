from __future__ import annotations

from functools import lru_cache
from typing import Any, Dict, List, Optional, Tuple


def dart(label: str) -> Dict[str, Any]:
    if label == "SBull":
        return {"label": "SBull", "field": 25, "ring": "single_bull", "multiplier": 1, "score": 25}
    if label == "DBull":
        return {"label": "DBull", "field": 25, "ring": "double_bull", "multiplier": 2, "score": 50}
    prefix, field = label[0], int(label[1:])
    ring, multiplier = {
        "S": ("single_outer", 1),
        "D": ("double", 2),
        "T": ("triple", 3),
    }[prefix]
    return {"label": label, "field": field, "ring": ring, "multiplier": multiplier, "score": field * multiplier}


DARTS: List[Dict[str, Any]] = [dart(f"S{i}") for i in range(1, 21)] + [dart(f"D{i}") for i in range(1, 21)] + [dart(f"T{i}") for i in range(1, 21)] + [dart("SBull"), dart("DBull")]
DART_BY_LABEL = {item["label"]: item for item in DARTS}

# Standard double-out checkout chart. This is intentionally opinionated and
# table-first: common/professional routes are preferred over mathematically valid
# but awkward algorithmic routes. The algorithm below is only fallback.
STANDARD_DOUBLE_OUT: Dict[int, List[str]] = {
    170:["T20","T20","DBull"], 167:["T20","T19","DBull"], 164:["T20","T18","DBull"], 161:["T20","T17","DBull"],
    160:["T20","T20","D20"], 158:["T20","T20","D19"], 157:["T20","T19","D20"], 156:["T20","T20","D18"],
    155:["T20","T19","D19"], 154:["T20","T18","D20"], 153:["T20","T19","D18"], 152:["T20","T20","D16"],
    151:["T20","T17","D20"], 150:["T20","T18","D18"], 149:["T20","T19","D16"], 148:["T20","T16","D20"],
    147:["T20","T17","D18"], 146:["T20","T18","D16"], 145:["T20","T15","D20"], 144:["T20","T20","D12"],
    143:["T20","T17","D16"], 142:["T20","T14","D20"], 141:["T20","T15","D18"], 140:["T20","T20","D10"],
    139:["T19","T14","D20"], 138:["T20","T18","D12"], 137:["T20","T19","D10"], 136:["T20","T20","D8"],
    135:["T20","T17","D12"], 134:["T20","T14","D16"], 133:["T20","T19","D8"], 132:["T20","T16","D12"],
    131:["T20","T13","D16"], 130:["T20","T20","D5"], 129:["T19","T16","D12"], 128:["T18","T18","D10"],
    127:["T20","T17","D8"], 126:["T19","T19","D6"], 125:["SBull","T20","D20"], 124:["T20","T16","D8"],
    123:["T19","T16","D9"], 122:["T18","T18","D7"], 121:["T20","T11","D14"], 120:["T20","S20","D20"],
    119:["T19","T12","D13"], 118:["T20","S18","D20"], 117:["T20","S17","D20"], 116:["T20","S16","D20"],
    115:["T20","S15","D20"], 114:["T20","S14","D20"], 113:["T20","S13","D20"], 112:["T20","S12","D20"],
    111:["T20","S11","D20"], 110:["T20","S10","D20"], 109:["T20","S9","D20"], 108:["T20","S8","D20"],
    107:["T19","S10","D20"], 106:["T20","S6","D20"], 105:["T20","S5","D20"], 104:["T18","S10","D20"],
    103:["T19","S6","D20"], 102:["T20","S10","D16"], 101:["T17","S10","D20"], 100:["T20","D20"],
    99:["T19","S10","D16"], 98:["T20","D19"], 97:["T19","D20"], 96:["T20","D18"], 95:["T19","D19"],
    94:["T18","D20"], 93:["T19","D18"], 92:["T20","D16"], 91:["T17","D20"], 90:["T18","D18"],
    89:["T19","D16"], 88:["T16","D20"], 87:["T17","D18"], 86:["T18","D16"], 85:["T15","D20"],
    84:["T20","D12"], 83:["T17","D16"], 82:["DBull","D16"], 81:["T15","D18"], 80:["T20","D10"],
    79:["T19","D11"], 78:["T18","D12"], 77:["T19","D10"], 76:["T20","D8"], 75:["T17","D12"],
    74:["T14","D16"], 73:["T19","D8"], 72:["T16","D12"], 71:["T13","D16"], 70:["T18","D8"],
    69:["T19","D6"], 68:["T20","D4"], 67:["T17","D8"], 66:["T10","D18"], 65:["T15","D10"],
    64:["T16","D8"], 63:["T13","D12"], 62:["T10","D16"], 61:["T15","D8"],
    60:["S20","D20"], 59:["S19","D20"], 58:["S18","D20"], 57:["S17","D20"], 56:["S16","D20"],
    55:["S15","D20"], 54:["S14","D20"], 53:["S13","D20"], 52:["S12","D20"], 51:["S11","D20"],
    50:["S10","D20"], 49:["S9","D20"], 48:["S16","D16"], 47:["S15","D16"], 46:["S14","D16"],
    45:["S13","D16"], 44:["S12","D16"], 43:["S11","D16"], 42:["S10","D16"], 41:["S9","D16"],
    40:["D20"], 39:["S7","D16"], 38:["D19"], 37:["S5","D16"], 36:["D18"], 35:["S3","D16"],
    34:["D17"], 33:["S1","D16"], 32:["D16"], 31:["S15","D8"], 30:["D15"], 29:["S13","D8"],
    28:["D14"], 27:["S11","D8"], 26:["D13"], 25:["S9","D8"], 24:["D12"], 23:["S7","D8"],
    22:["D11"], 21:["S5","D8"], 20:["D10"], 19:["S3","D8"], 18:["D9"], 17:["S1","D8"],
    16:["D8"], 15:["S7","D4"], 14:["D7"], 13:["S5","D4"], 12:["D6"], 11:["S3","D4"],
    10:["D5"], 9:["S1","D4"], 8:["D4"], 7:["S3","D2"], 6:["D3"], 5:["S1","D2"],
    4:["D2"], 3:["S1","D1"], 2:["D1"],
}

SETUP_LEAVE_PRIORITY = [40, 32, 36, 24, 16, 20, 10, 8, 4, 2, 50, 60, 80, 100, 120, 140, 160, 170,
                        111, 110, 109, 108, 107, 106, 105, 104, 103, 102, 101]
SETUP_RANK = {leave: index for index, leave in enumerate(SETUP_LEAVE_PRIORITY)}
PREFERENCE = {"T20":1000,"T19":990,"T18":980,"T17":970,"T16":960,"T15":950,"DBull":940,
              "D20":900,"D16":890,"D18":880,"D12":870,"D10":860,"D8":850,"D4":840,"D2":830,"D1":820}


def _sequence(labels: List[str]) -> List[Dict[str, Any]]:
    return [DART_BY_LABEL[label] for label in labels]


def _is_double(d: Dict[str, Any]) -> bool:
    return int(d["multiplier"]) == 2


def _valid_finish(score: int, out_rule: str, d: Dict[str, Any]) -> bool:
    return d["score"] == score and (out_rule != "double" or _is_double(d))


def _rank_sequence(seq: List[Dict[str, Any]]) -> tuple:
    first = seq[0]["label"] if seq else ""
    bull_count = sum(1 for d in seq if d["field"] == 25)
    return (len(seq), -PREFERENCE.get(first, 0), -seq[0]["score"], bull_count, first)


@lru_cache(maxsize=2048)
def algorithmic_checkout(score: int, darts_left: int, out_rule: str = "double") -> Optional[Tuple[Dict[str, Any], ...]]:
    if score <= 0 or darts_left <= 0 or (out_rule == "double" and score == 1):
        return None
    candidates: List[List[Dict[str, Any]]] = []
    for d in DARTS:
        value = int(d["score"])
        if value > score:
            continue
        if _valid_finish(score, out_rule, d):
            candidates.append([d])
        elif darts_left > 1:
            rest = algorithmic_checkout(score - value, darts_left - 1, out_rule)
            if rest:
                candidates.append([d, *list(rest)])
    if not candidates:
        return None
    return tuple(sorted(candidates, key=_rank_sequence)[0])


def checkout_sequence(score: int, darts_left: int, out_rule: str = "double") -> Optional[List[Dict[str, Any]]]:
    if score <= 0 or darts_left <= 0:
        return None
    if out_rule == "double" and score in STANDARD_DOUBLE_OUT:
        seq = _sequence(STANDARD_DOUBLE_OUT[score])
        if len(seq) <= darts_left:
            return seq
    fallback = algorithmic_checkout(score, darts_left, out_rule)
    return list(fallback) if fallback else None


def _score(seq: List[Dict[str, Any]]) -> int:
    return sum(int(d["score"]) for d in seq)


@lru_cache(maxsize=4096)
def exact_scoring_sequence(total: int, darts_count: int) -> Optional[Tuple[Dict[str, Any], ...]]:
    if darts_count == 0:
        return tuple() if total == 0 else None
    if total <= 0:
        return None
    dart_pool = sorted(DARTS, key=lambda d: (-PREFERENCE.get(d["label"], 0), -d["score"]))
    candidates: List[List[Dict[str, Any]]] = []
    for d in dart_pool:
        value = int(d["score"])
        if value > total:
            continue
        rest = exact_scoring_sequence(total - value, darts_count - 1)
        if rest is not None:
            candidates.append([d, *list(rest)])
    if not candidates:
        return None
    def setup_seq_rank(seq: List[Dict[str, Any]]) -> tuple:
        def dart_penalty(d: Dict[str, Any]) -> int:
            label = d["label"]
            field = int(d["field"])
            ring = d["ring"]
            if ring in ("double", "double_bull"):
                return 500
            if ring == "triple" and field < 15:
                return 20
            if ring.startswith("single"):
                return 10
            return 0
        return (
            sum(dart_penalty(d) for d in seq),
            -sum(PREFERENCE.get(d["label"], 0) for d in seq),
            -_score(seq),
        )
    candidates.sort(key=setup_seq_rank)
    return tuple(candidates[0])


def setup_plan(score: int, darts_left: int, out_rule: str = "double") -> Optional[Dict[str, Any]]:
    if darts_left <= 0 or score <= 1:
        return None
    # Prefer a full remaining-darts plan that leaves a known good checkout/setup.
    # Example: 171 with 3 darts -> T20, T20, S11 leaves 40.
    leave_candidates = []
    for leave in SETUP_LEAVE_PRIORITY + sorted(STANDARD_DOUBLE_OUT.keys(), reverse=True):
        if leave <= 1 or leave >= score:
            continue
        if out_rule == "double" and leave == 1:
            continue
        if leave in {item["leave"] for item in leave_candidates}:
            continue
        leave_candidates.append({
            "leave": leave,
            "rank": SETUP_RANK.get(leave, 999),
            "next_turn_checkout": checkout_sequence(leave, 3, out_rule) or [],
        })
    options = []
    for candidate in leave_candidates:
        needed = score - candidate["leave"]
        plan = exact_scoring_sequence(needed, darts_left)
        if not plan:
            continue
        plan_list = list(plan)
        options.append({
            "plan": plan_list,
            "target": plan_list[0],
            "leave": candidate["leave"],
            "next_turn_checkout": candidate["next_turn_checkout"],
            "leave_rank": candidate["rank"],
            "preference": sum(PREFERENCE.get(d["label"], 0) for d in plan_list),
        })
    if not options:
        return None
    options.sort(key=lambda item: (
        item["leave_rank"],
        0 if item["next_turn_checkout"] else 1,
        -item["preference"],
        -_score(item["plan"]),
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
        result.update({
            "status": "checkout",
            "message": "Finish möglich",
            "primary": checkout[0],
            "sequence": checkout,
        })
        return result

    setup = setup_plan(score, darts_left, out_rule)
    if setup:
        plan = setup["plan"]
        leave = setup["leave"]
        next_turn = setup.get("next_turn_checkout") or []
        message = "Stellen: " + " · ".join(d["label"] for d in plan) + f" lässt {leave}"
        if next_turn:
            message += " – nächster Turn " + " · ".join(d["label"] for d in next_turn)
        result.update({
            "status": "setup",
            "message": message,
            "primary": plan[0],
            "sequence": plan,
            "setup": setup,
        })
        return result

    safe = [d for d in DARTS if 1 < score - d["score"]]
    if safe:
        safe.sort(key=lambda d: (-PREFERENCE.get(d["label"], 0), -d["score"]))
        target = safe[0]
        result.update({
            "status": "score_down",
            "message": f"Runterspielen: {target['label']} lässt {score - target['score']}",
            "primary": target,
            "sequence": [target],
            "setup": {"target": target, "plan": [target], "leave": score - target["score"], "next_turn_checkout": []},
        })
    return result
