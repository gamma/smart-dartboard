from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
from uuid import uuid4


@dataclass
class Player:
    id: str
    name: str
    score: int = 501


@dataclass
class ThrowEvent:
    seq: int
    type: str
    label: str
    score: int
    player_id: Optional[str]
    raw: Dict[str, Any]


@dataclass
class GameState:
    game_type: str = "countup"  # countup or x01
    x01_start_score: int = 501
    players: List[Player] = field(default_factory=list)
    current_player_index: int = 0
    darts_in_turn: int = 0
    turn_score: int = 0
    throws: List[ThrowEvent] = field(default_factory=list)
    status: str = "idle"  # idle/running/finished
    winner_id: Optional[str] = None
    last_event: Optional[Dict[str, Any]] = None

    def current_player(self) -> Optional[Player]:
        if not self.players:
            return None
        return self.players[self.current_player_index % len(self.players)]

    def as_dict(self) -> Dict[str, Any]:
        return {
            "game_type": self.game_type,
            "x01_start_score": self.x01_start_score,
            "players": [p.__dict__ for p in self.players],
            "current_player_index": self.current_player_index,
            "current_player_id": self.current_player().id if self.current_player() else None,
            "darts_in_turn": self.darts_in_turn,
            "turn_score": self.turn_score,
            "status": self.status,
            "winner_id": self.winner_id,
            "last_event": self.last_event,
            "throws": [t.__dict__ for t in self.throws[-30:]],
        }


class GameEngine:
    def __init__(self) -> None:
        self.state = GameState(players=[Player(id=str(uuid4()), name="Player 1")])

    def reset(self, game_type: str = "countup", players: Optional[List[str]] = None, x01_start_score: int = 501) -> GameState:
        names = players or ["Player 1"]
        initial = 0 if game_type == "countup" else x01_start_score
        self.state = GameState(
            game_type=game_type,
            x01_start_score=x01_start_score,
            players=[Player(id=str(uuid4()), name=n, score=initial) for n in names if n.strip()],
            status="running",
        )
        if not self.state.players:
            self.state.players.append(Player(id=str(uuid4()), name="Player 1", score=initial))
        return self.state

    def next_player(self) -> GameState:
        if self.state.players:
            self.state.current_player_index = (self.state.current_player_index + 1) % len(self.state.players)
        self.state.darts_in_turn = 0
        self.state.turn_score = 0
        self.state.last_event = {"type": "next_player"}
        return self.state

    def handle_event(self, event: Dict[str, Any]) -> GameState:
        self.state.last_event = event
        if self.state.status != "running":
            return self.state

        typ = event.get("type")
        if typ == "button" and event.get("action") == "press":
            self.next_player()
            return self.state

        if typ == "hit":
            score = int(event.get("score", 0))
            label = str(event.get("label", ""))
            self._apply_throw(score, label, event)
        elif typ == "miss":
            self._apply_throw(0, "MISS", event)

        return self.state

    def _apply_throw(self, score: int, label: str, raw: Dict[str, Any]) -> None:
        player = self.state.current_player()
        if player is None:
            return

        if self.state.game_type == "countup":
            player.score += score
        elif self.state.game_type == "x01":
            new_score = player.score - score
            if new_score >= 0:
                player.score = new_score
                if new_score == 0:
                    self.state.status = "finished"
                    self.state.winner_id = player.id
            else:
                # Simple bust: count dart but don't change score. Full x01 rules later.
                raw = {**raw, "bust": True}

        self.state.darts_in_turn += 1
        self.state.turn_score += score
        self.state.throws.append(ThrowEvent(
            seq=int(raw.get("seq", -1)),
            type="throw",
            label=label,
            score=score,
            player_id=player.id,
            raw=raw,
        ))

        if self.state.darts_in_turn >= 3 and self.state.status == "running":
            self.next_player()
