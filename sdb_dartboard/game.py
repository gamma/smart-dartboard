from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
from uuid import uuid4

CRICKET_TARGETS = [20, 19, 18, 17, 16, 15, 25]


@dataclass
class Player:
    id: str
    name: str
    score: int = 0
    marks: Dict[str, int] = field(default_factory=dict)


@dataclass
class ThrowEvent:
    seq: int
    type: str
    label: str
    score: int
    player_id: Optional[str]
    raw: Dict[str, Any]
    snapshot_before: Dict[str, Any]


@dataclass
class GameState:
    game_type: str = "countup"  # countup, x01, cricket
    x01_start_score: int = 501
    players: List[Player] = field(default_factory=list)
    current_player_index: int = 0
    darts_in_turn: int = 0
    turn_score: int = 0
    throws: List[ThrowEvent] = field(default_factory=list)
    status: str = "idle"  # idle, running, hold, finished
    winner_id: Optional[str] = None
    last_event: Optional[Dict[str, Any]] = None
    message: str = "Ready"

    def current_player(self) -> Optional[Player]:
        if not self.players:
            return None
        return self.players[self.current_player_index % len(self.players)]

    def snapshot(self) -> Dict[str, Any]:
        return {
            "players": [{"id": p.id, "name": p.name, "score": p.score, "marks": dict(p.marks)} for p in self.players],
            "current_player_index": self.current_player_index,
            "darts_in_turn": self.darts_in_turn,
            "turn_score": self.turn_score,
            "status": self.status,
            "winner_id": self.winner_id,
            "message": self.message,
        }

    def restore_snapshot(self, snap: Dict[str, Any]) -> None:
        by_id = {p.id: p for p in self.players}
        restored: List[Player] = []
        for pdata in snap["players"]:
            p = by_id.get(pdata["id"], Player(id=pdata["id"], name=pdata["name"]))
            p.name = pdata["name"]
            p.score = pdata["score"]
            p.marks = dict(pdata.get("marks", {}))
            restored.append(p)
        self.players = restored
        self.current_player_index = snap["current_player_index"]
        self.darts_in_turn = snap["darts_in_turn"]
        self.turn_score = snap["turn_score"]
        self.status = snap["status"]
        self.winner_id = snap["winner_id"]
        self.message = snap["message"]

    def as_dict(self) -> Dict[str, Any]:
        return {
            "game_type": self.game_type,
            "x01_start_score": self.x01_start_score,
            "players": [{"id": p.id, "name": p.name, "score": p.score, "marks": p.marks} for p in self.players],
            "current_player_index": self.current_player_index,
            "current_player_id": self.current_player().id if self.current_player() else None,
            "darts_in_turn": self.darts_in_turn,
            "turn_score": self.turn_score,
            "status": self.status,
            "winner_id": self.winner_id,
            "last_event": self.last_event,
            "message": self.message,
            "cricket_targets": CRICKET_TARGETS,
            "throws": [
                {"seq": t.seq, "type": t.type, "label": t.label, "score": t.score, "player_id": t.player_id, "raw": t.raw}
                for t in self.throws[-30:]
            ],
        }


class GameEngine:
    def __init__(self) -> None:
        self.state = GameState(players=[Player(id=str(uuid4()), name="Player 1")])

    def reset(self, game_type: str = "countup", players: Optional[List[str]] = None, x01_start_score: int = 501) -> GameState:
        names = [n.strip() for n in (players or ["Player 1"]) if n.strip()] or ["Player 1"]
        game_type = game_type.lower()
        initial_score = 0 if game_type in ("countup", "cricket") else x01_start_score
        ps = [Player(id=str(uuid4()), name=n, score=initial_score) for n in names]
        if game_type == "cricket":
            for p in ps:
                p.marks = {str(t): 0 for t in CRICKET_TARGETS}
        self.state = GameState(
            game_type=game_type,
            x01_start_score=x01_start_score,
            players=ps,
            status="running",
            message="Game started",
        )
        return self.state

    def continue_turn(self) -> GameState:
        if self.state.status == "hold":
            self._advance_player()
            self.state.status = "running"
            self.state.message = "Next player"
            self.state.last_event = {"type": "continue"}
        return self.state

    def next_player(self) -> GameState:
        # Public forced next-player action from screen or board button.
        if self.state.status in ("running", "hold"):
            self._advance_player()
            self.state.status = "running"
            self.state.message = "Next player"
            self.state.last_event = {"type": "next_player"}
        return self.state

    def undo(self) -> GameState:
        if not self.state.throws:
            return self.state
        last = self.state.throws.pop()
        self.state.restore_snapshot(last.snapshot_before)
        self.state.last_event = {"type": "undo", "undone": last.label}
        self.state.message = f"Undo {last.label}"
        return self.state

    def handle_event(self, event: Dict[str, Any]) -> GameState:
        self.state.last_event = event
        typ = event.get("type")

        if typ == "button" and event.get("action") == "press":
            # Board button means continue if in hold, otherwise force next player.
            if self.state.status == "hold":
                return self.continue_turn()
            return self.next_player()

        if self.state.status != "running":
            return self.state

        if typ == "hit":
            self._apply_throw(event)
        elif typ == "miss":
            miss = {**event, "label": "MISS", "score": 0}
            self._apply_throw(miss)

        return self.state

    def _advance_player(self) -> None:
        if self.state.players:
            self.state.current_player_index = (self.state.current_player_index + 1) % len(self.state.players)
        self.state.darts_in_turn = 0
        self.state.turn_score = 0

    def _hold_after_turn(self) -> None:
        if self.state.status == "running" and self.state.darts_in_turn >= 3:
            self.state.status = "hold"
            self.state.message = "Turn complete. Press continue."

    def _apply_throw(self, event: Dict[str, Any]) -> None:
        player = self.state.current_player()
        if player is None:
            return
        snap = self.state.snapshot()
        label = str(event.get("label", ""))
        score = int(event.get("score", 0))

        if self.state.game_type == "countup":
            self._apply_countup(player, score)
        elif self.state.game_type == "x01":
            self._apply_x01(player, score, event)
        elif self.state.game_type == "cricket":
            self._apply_cricket(player, event)
        else:
            self._apply_countup(player, score)

        self.state.darts_in_turn += 1
        self.state.turn_score += score
        self.state.throws.append(ThrowEvent(
            seq=int(event.get("seq", -1)),
            type="throw",
            label=label,
            score=score,
            player_id=player.id,
            raw=event,
            snapshot_before=snap,
        ))
        if self.state.status == "running":
            self.state.message = f"{player.name}: {label}"
        self._hold_after_turn()

    def _apply_countup(self, player: Player, score: int) -> None:
        player.score += score

    def _apply_x01(self, player: Player, score: int, event: Dict[str, Any]) -> None:
        new_score = player.score - score
        if new_score < 0:
            event["bust"] = True
            self.state.message = "Bust"
            return
        player.score = new_score
        if new_score == 0:
            self.state.status = "finished"
            self.state.winner_id = player.id
            self.state.message = f"{player.name} wins"

    def _apply_cricket(self, player: Player, event: Dict[str, Any]) -> None:
        field = int(event.get("field", 0) or 0)
        multiplier = int(event.get("multiplier", 0) or 0)
        if field not in CRICKET_TARGETS or multiplier <= 0:
            return
        key = str(field)
        before = player.marks.get(key, 0)
        after = min(3, before + multiplier)
        overflow = max(0, before + multiplier - 3)
        player.marks[key] = after
        # Simple Cricket scoring: score overflow if at least one opponent is not closed.
        if overflow and any(op.id != player.id and op.marks.get(key, 0) < 3 for op in self.state.players):
            player.score += overflow * field
        if all(player.marks.get(str(t), 0) >= 3 for t in CRICKET_TARGETS):
            if player.score >= max((p.score for p in self.state.players if p.id != player.id), default=0):
                self.state.status = "finished"
                self.state.winner_id = player.id
                self.state.message = f"{player.name} wins"
