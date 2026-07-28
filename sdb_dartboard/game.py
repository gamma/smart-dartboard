from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
from uuid import uuid4

from .games import registry
from .games.cricket import CRICKET_TARGETS


@dataclass
class Player:
    id: str
    name: str
    score: int = 0
    marks: Dict[str, int] = field(default_factory=dict)
    avatar: str = "comet"
    color: str = "#28e7ff"


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
    game_type: str = "countup"
    x01_start_score: int = 501
    players: List[Player] = field(default_factory=list)
    current_player_index: int = 0
    darts_in_turn: int = 0
    turn_score: int = 0
    throws: List[ThrowEvent] = field(default_factory=list)
    status: str = "idle"
    winner_id: Optional[str] = None
    last_event: Optional[Dict[str, Any]] = None
    message: str = "Ready"
    options: Dict[str, Any] = field(default_factory=dict)
    turn_start_values: Dict[str, int] = field(default_factory=dict)
    round_number: int = 1

    def current_player(self) -> Optional[Player]:
        if not self.players:
            return None
        return self.players[self.current_player_index % len(self.players)]

    def snapshot(self) -> Dict[str, Any]:
        return {
            "players": [
                {
                    "id": player.id,
                    "name": player.name,
                    "score": player.score,
                    "marks": dict(player.marks),
                    "avatar": player.avatar,
                    "color": player.color,
                }
                for player in self.players
            ],
            "current_player_index": self.current_player_index,
            "darts_in_turn": self.darts_in_turn,
            "turn_score": self.turn_score,
            "status": self.status,
            "winner_id": self.winner_id,
            "message": self.message,
            "turn_start_values": dict(self.turn_start_values),
            "round_number": self.round_number,
        }

    def restore_snapshot(self, snap: Dict[str, Any]) -> None:
        by_id = {player.id: player for player in self.players}
        restored: List[Player] = []
        for data in snap["players"]:
            player = by_id.get(data["id"], Player(id=data["id"], name=data["name"]))
            player.name = data["name"]
            player.score = data["score"]
            player.marks = dict(data.get("marks", {}))
            player.avatar = data.get("avatar", "comet")
            player.color = data.get("color", "#28e7ff")
            restored.append(player)
        self.players = restored
        self.current_player_index = snap["current_player_index"]
        self.darts_in_turn = snap["darts_in_turn"]
        self.turn_score = snap["turn_score"]
        self.status = snap["status"]
        self.winner_id = snap["winner_id"]
        self.message = snap["message"]
        self.turn_start_values = dict(snap.get("turn_start_values", {}))
        self.round_number = int(snap.get("round_number", 1))

    def as_dict(self) -> Dict[str, Any]:
        return {
            "game_type": self.game_type,
            "x01_start_score": self.x01_start_score,
            "players": [
                {
                    "id": player.id,
                    "name": player.name,
                    "score": player.score,
                    "marks": player.marks,
                    "avatar": player.avatar,
                    "color": player.color,
                }
                for player in self.players
            ],
            "current_player_index": self.current_player_index,
            "current_player_id": self.current_player().id if self.current_player() else None,
            "darts_in_turn": self.darts_in_turn,
            "turn_score": self.turn_score,
            "round_number": self.round_number,
            "status": self.status,
            "winner_id": self.winner_id,
            "last_event": self.last_event,
            "message": self.message,
            "options": self.options,
            "mode": registry.get(self.game_type).metadata.as_dict() if self.status != "idle" else None,
            "cricket_targets": CRICKET_TARGETS,
            "throws": [
                {
                    "seq": throw.seq,
                    "type": throw.type,
                    "label": throw.label,
                    "score": throw.score,
                    "player_id": throw.player_id,
                    "raw": throw.raw,
                }
                for throw in self.throws[-30:]
            ],
        }


class GameEngine:
    def __init__(self) -> None:
        self.state = GameState(players=[Player(id=str(uuid4()), name="Player 1")])

    def reset(
        self,
        game_type: str = "countup",
        players: Optional[List[Any]] = None,
        x01_start_score: int = 501,
        options: Optional[Dict[str, Any]] = None,
    ) -> GameState:
        game_type = game_type.lower()
        mode = registry.get(game_type)
        source_players = players or ["Player 1"]
        resolved_players: List[Player] = []
        for item in source_players:
            if isinstance(item, dict):
                name = str(item.get("name", "")).strip()
                if name:
                    resolved_players.append(
                        Player(
                            id=str(item.get("id") or uuid4()),
                            name=name,
                            avatar=str(item.get("avatar", "comet")),
                            color=str(item.get("color", "#28e7ff")),
                        )
                    )
            else:
                name = str(item).strip()
                if name:
                    resolved_players.append(Player(id=str(uuid4()), name=name))
        if not resolved_players:
            resolved_players = [Player(id=str(uuid4()), name="Player 1")]

        resolved_options = {
            option.key: option.default for option in mode.metadata.options
        }
        resolved_options.update(options or {})
        if game_type == "x01" and "start_score" not in (options or {}):
            resolved_options["start_score"] = x01_start_score
        for player in resolved_players:
            mode.initialize_player(player, resolved_options)

        self.state = GameState(
            game_type=game_type,
            x01_start_score=int(resolved_options.get("start_score", x01_start_score)),
            players=resolved_players,
            status="running",
            message="Game started",
            options=resolved_options,
            turn_start_values={player.id: player.score for player in resolved_players},
        )
        return self.state

    def export_state(self) -> Dict[str, Any]:
        """Return a complete checkpoint, including data needed for Undo."""
        return {
            "game_type": self.state.game_type,
            "x01_start_score": self.state.x01_start_score,
            "options": self.state.options,
            "state": self.state.snapshot(),
            "last_event": self.state.last_event,
            "throws": [
                {
                    "seq": throw.seq,
                    "type": throw.type,
                    "label": throw.label,
                    "score": throw.score,
                    "player_id": throw.player_id,
                    "raw": throw.raw,
                    "snapshot_before": throw.snapshot_before,
                }
                for throw in self.state.throws
            ],
        }

    def import_state(self, checkpoint: Dict[str, Any]) -> GameState:
        """Restore an exact checkpoint after a process or machine restart."""
        self.state = GameState(
            game_type=checkpoint["game_type"],
            x01_start_score=int(checkpoint.get("x01_start_score", 501)),
            options=dict(checkpoint.get("options", {})),
        )
        self.state.restore_snapshot(checkpoint["state"])
        self.state.last_event = checkpoint.get("last_event")
        self.state.throws = [
            ThrowEvent(
                seq=int(data["seq"]),
                type=str(data.get("type", "throw")),
                label=str(data.get("label", "")),
                score=int(data.get("score", 0)),
                player_id=data.get("player_id"),
                raw=dict(data.get("raw", {})),
                snapshot_before=dict(data["snapshot_before"]),
            )
            for data in checkpoint.get("throws", [])
        ]
        return self.state

    def continue_turn(self) -> GameState:
        if self.state.status == "hold":
            self._advance_player()
            self.state.status = "running"
            self.state.message = "Next player"
            self.state.last_event = {"type": "continue"}
        return self.state

    def next_player(self) -> GameState:
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

    def correct_turn_throw(
        self,
        turn_index: int,
        replacement: Dict[str, Any],
    ) -> GameState:
        """Replace one throw in the active three-dart turn and replay the rest."""
        turn_count = self.state.darts_in_turn
        if turn_count <= 0:
            raise ValueError("There are no throws in the current turn")
        if turn_index < 0 or turn_index >= turn_count:
            raise ValueError("Throw index is outside the current turn")
        target_position = len(self.state.throws) - turn_count + turn_index
        if target_position < 0:
            raise ValueError("Throw history is incomplete")

        target = self.state.throws[target_position]
        current_player = self.state.current_player()
        if current_player is None or target.player_id != current_player.id:
            raise ValueError("Only throws from the current player can be corrected")

        subsequent_events = [
            dict(throw.raw) for throw in self.state.throws[target_position + 1 :]
        ]
        prefix = list(self.state.throws[:target_position])
        self.state.restore_snapshot(target.snapshot_before)
        self.state.throws = prefix

        corrected = dict(replacement)
        corrected["seq"] = target.seq
        corrected["corrected"] = True
        self.handle_event(corrected)
        for event in subsequent_events:
            replay = dict(event)
            replay.pop("bust", None)
            self.handle_event(replay)
        return self.state

    def handle_event(self, event: Dict[str, Any]) -> GameState:
        self.state.last_event = event
        event_type = event.get("type")

        if event_type == "button" and event.get("action") == "press":
            if self.state.status == "hold":
                return self.continue_turn()
            return self.next_player()

        if self.state.status != "running":
            return self.state

        if event_type == "hit":
            self._apply_throw(event)
        elif event_type == "miss":
            miss = {**event, "label": "MISS", "score": 0}
            self.state.last_event = miss
            self._apply_throw(miss)
        return self.state

    def _advance_player(self) -> None:
        was_last_player = bool(
            self.state.players
            and self.state.current_player_index == len(self.state.players) - 1
        )
        if self.state.players:
            self.state.current_player_index = (
                self.state.current_player_index + 1
            ) % len(self.state.players)
        if was_last_player:
            self.state.round_number += 1
        self.state.darts_in_turn = 0
        self.state.turn_score = 0
        player = self.state.current_player()
        if player:
            self.state.turn_start_values[player.id] = player.score

    def _hold_after_turn(self) -> None:
        if self.state.status == "running" and self.state.darts_in_turn >= 3:
            self.state.status = "hold"
            self.state.message = "Turn complete. Press continue."

    def _apply_throw(self, event: Dict[str, Any]) -> None:
        player = self.state.current_player()
        if player is None:
            return
        snapshot = self.state.snapshot()
        label = str(event.get("label", ""))
        score = int(event.get("score", 0))
        outcome = registry.get(self.state.game_type).apply_throw(
            self.state, player, event
        )

        self.state.darts_in_turn += 1
        self.state.turn_score += outcome.turn_value
        self.state.throws.append(
            ThrowEvent(
                seq=int(event.get("seq", -1)),
                type="throw",
                label=label,
                score=score,
                player_id=player.id,
                raw=event,
                snapshot_before=snapshot,
            )
        )
        self.state.message = outcome.message
        if outcome.finished:
            self.state.status = "finished"
            self.state.winner_id = outcome.winner_id or player.id
        elif outcome.force_hold:
            self.state.status = "hold"
        else:
            self._hold_after_turn()
