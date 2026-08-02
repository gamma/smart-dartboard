from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
from uuid import uuid4
import copy
import random

from .games import registry
from .games.cricket import CRICKET_TARGETS
from .games.x01_advisor import x01_advice


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
    context_before: Dict[str, Any] = field(default_factory=dict)
    round_number: int = 1
    dart_in_turn: int = 1
    mode_points: int = 0
    outcome: str = "neutral"
    source: str = "unknown"
    score_after: int = 0


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
    winner_ids: List[str] = field(default_factory=list)
    result_type: str = ""
    last_event: Optional[Dict[str, Any]] = None
    message: str = "Ready"
    options: Dict[str, Any] = field(default_factory=dict)
    turn_start_values: Dict[str, int] = field(default_factory=dict)
    round_number: int = 1
    mode_state: Dict[str, Any] = field(default_factory=dict)
    random_seed: int = 0
    random_cursor: int = 0

    def current_player(self) -> Optional[Player]:
        if not self.players:
            return None
        return self.players[self.current_player_index % len(self.players)]

    def random_index(self, upper_bound: int) -> int:
        """Return the next deterministic gameplay index.

        The persisted SplitMix64 stream intentionally matches the Rust core.
        It is gameplay randomness, not a cryptographic primitive.
        """
        if upper_bound <= 0:
            raise ValueError("random upper bound must be greater than zero")
        mask = (1 << 64) - 1
        self.random_cursor = (self.random_cursor + 1) & mask
        value = (
            self.random_seed
            + 0x9E3779B97F4A7C15 * self.random_cursor
        ) & mask
        value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & mask
        value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & mask
        value ^= value >> 31
        return value % upper_bound

    def snapshot(self) -> Dict[str, Any]:
        return {
            "players": [
                {
                    "id": player.id,
                    "name": player.name,
                    "score": player.score,
                    "marks": copy.deepcopy(player.marks),
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
            "winner_ids": list(self.winner_ids),
            "result_type": self.result_type,
            "message": self.message,
            "turn_start_values": dict(self.turn_start_values),
            "round_number": self.round_number,
            "mode_state": copy.deepcopy(self.mode_state),
            "random_seed": self.random_seed,
            "random_cursor": self.random_cursor,
        }

    def restore_snapshot(self, snap: Dict[str, Any]) -> None:
        by_id = {player.id: player for player in self.players}
        restored: List[Player] = []
        for data in snap["players"]:
            player = by_id.get(data["id"], Player(id=data["id"], name=data["name"]))
            player.name = data["name"]
            player.score = data["score"]
            player.marks = copy.deepcopy(data.get("marks", {}))
            player.avatar = data.get("avatar", "comet")
            player.color = data.get("color", "#28e7ff")
            restored.append(player)
        self.players = restored
        self.current_player_index = snap["current_player_index"]
        self.darts_in_turn = snap["darts_in_turn"]
        self.turn_score = snap["turn_score"]
        self.status = snap["status"]
        self.winner_id = snap["winner_id"]
        self.winner_ids = list(
            snap.get(
                "winner_ids",
                [self.winner_id] if self.winner_id else [],
            )
        )
        self.result_type = str(snap.get("result_type", ""))
        self.message = snap["message"]
        self.turn_start_values = dict(snap.get("turn_start_values", {}))
        self.round_number = int(snap.get("round_number", 1))
        self.mode_state = copy.deepcopy(snap.get("mode_state", {}))
        self.random_seed = int(snap.get("random_seed", 0))
        self.random_cursor = int(snap.get("random_cursor", 0))

    def advice(self) -> Optional[Dict[str, Any]]:
        if self.game_type != "x01" or self.status not in ("running", "hold"):
            return None
        player = self.current_player()
        if not player:
            return None
        darts_left = max(0, 3 - self.darts_in_turn)
        return x01_advice(player.score, darts_left, str(self.options.get("out_rule", "straight")))

    def overlay(self) -> Optional[Dict[str, Any]]:
        if self.status == "idle":
            return None
        mode = registry.get(self.game_type)
        getter = getattr(mode, "get_overlay", None)
        if getter is None:
            return None
        return getter(self)

    def telemetry_context(self) -> Dict[str, Any]:
        overlay = self.overlay() or {}

        def normalize(items: Any) -> List[Dict[str, Any]]:
            result = []
            for item in items if isinstance(items, list) else []:
                if not isinstance(item, dict) or item.get("field") is None:
                    continue
                rings = item.get("rings")
                if not isinstance(rings, list):
                    rings = [item.get("ring")] if item.get("ring") else []
                result.append({
                    "field": item.get("field"),
                    "ring": item.get("ring"),
                    "rings": rings,
                    "id": item.get("id"),
                    "role": item.get("role"),
                    "value": item.get("value", item.get("label")),
                })
            return result

        targets = normalize(overlay.get("targets", [])) + normalize(
            overlay.get("bonus", [])
        )
        dangers = normalize(overlay.get("danger", []))
        cricket = overlay.get("cricket", {})
        if isinstance(cricket, dict):
            for item in cricket.get("remaining", []):
                if not isinstance(item, dict) or item.get("field") is None:
                    continue
                field = int(item["field"])
                targets.append({
                    "field": field,
                    "ring": None,
                    "rings": (
                        ["single_bull", "double_bull"]
                        if field == 25
                        else ["single_inner", "single_outer", "double", "triple"]
                    ),
                    "id": f"cricket-{field}",
                    "role": "target",
                    "value": item.get("needed"),
                })
        for zone in normalize(overlay.get("zones", [])):
            if zone.get("role") in {"danger", "mine"}:
                dangers.append(zone)
            elif zone.get("role") in {"target", "bonus"}:
                targets.append(zone)
        player = self.current_player()
        return {
            "round_number": self.round_number,
            "dart_in_turn": self.darts_in_turn + 1,
            "player_id": player.id if player else None,
            "prompt": overlay.get("prompt"),
            "targets": targets,
            "dangers": dangers,
            "overlay": overlay,
        }

    def replay_frame(self) -> Dict[str, Any]:
        return {
            "game_type": self.game_type,
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
            "current_player_id": self.current_player().id
            if self.current_player()
            else None,
            "darts_in_turn": self.darts_in_turn,
            "turn_score": self.turn_score,
            "round_number": self.round_number,
            "status": self.status,
            "winner_id": self.winner_id,
            "winner_ids": self.winner_ids,
            "result_type": self.result_type,
            "message": self.message,
            "options": self.options,
            "random_seed": self.random_seed,
            "random_cursor": self.random_cursor,
            "last_event": self.last_event,
            "overlay": self.overlay(),
        }

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
            "random_seed": self.random_seed,
            "random_cursor": self.random_cursor,
            "status": self.status,
            "winner_id": self.winner_id,
            "winner_ids": self.winner_ids,
            "result_type": self.result_type,
            "last_event": self.last_event,
            "message": self.message,
            "options": self.options,
            "mode": registry.get(self.game_type).metadata.as_dict() if self.status != "idle" else None,
            "cricket_targets": CRICKET_TARGETS,
            "advice": self.advice(),
            "overlay": self.overlay(),
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
        self._history_origin: Optional[Dict[str, Any]] = None
        self._actions: List[Dict[str, Any]] = []
        self._next_action_id = 1
        self._replaying = False
        self.last_action: Optional[Dict[str, Any]] = None
        self.last_undo_action: Optional[Dict[str, Any]] = None

    def clear(self) -> GameState:
        """Discard the transient board state when leaving a game."""
        self.state = GameState(status="idle", players=[])
        self._history_origin = None
        self._actions = []
        self._next_action_id = 1
        self.last_action = None
        self.last_undo_action = None
        return self.state

    def reset(
        self,
        game_type: str = "countup",
        players: Optional[List[Any]] = None,
        x01_start_score: int = 501,
        options: Optional[Dict[str, Any]] = None,
        random_seed: Optional[int] = None,
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

        resolved_options = mode.metadata.resolve_options(options)
        if game_type == "x01" and "start_score" not in (options or {}):
            resolved_options["start_score"] = x01_start_score
        if not mode.metadata.min_players <= len(resolved_players) <= mode.metadata.max_players:
            raise ValueError(
                f"{mode.metadata.title} supports "
                f"{mode.metadata.min_players}–{mode.metadata.max_players} players"
            )
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
            random_seed=(
                int(random_seed) & ((1 << 64) - 1)
                if random_seed is not None
                else random.getrandbits(64)  # nosec B311 - gameplay seed
            ),
        )
        initializer = getattr(mode, "initialize_state", None)
        if initializer is not None:
            initializer(self.state, resolved_options)
        # Capture the editable origin lazily with the first action. This also
        # includes any operator/test setup applied immediately after reset.
        self._history_origin = None
        self._actions = []
        self._next_action_id = 1
        self.last_action = None
        self.last_undo_action = None
        return self.state

    def export_state(self) -> Dict[str, Any]:
        """Return a complete checkpoint, including data needed for Undo."""
        return {
            "game_type": self.state.game_type,
            "x01_start_score": self.state.x01_start_score,
            "options": self.state.options,
            "state": self.state.snapshot(),
            "last_event": self.state.last_event,
            "history_origin": copy.deepcopy(self._history_origin),
            "actions": copy.deepcopy(self._actions),
            "next_action_id": self._next_action_id,
            "throws": [
                self._throw_to_dict(throw) for throw in self.state.throws
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
            self._throw_from_dict(data)
            for data in checkpoint.get("throws", [])
        ]
        origin = checkpoint.get("history_origin")
        self._history_origin = copy.deepcopy(origin) if origin else {
            "state": self.state.snapshot(),
            "random_state": random.getstate(),
            "throws": [
                self._throw_to_dict(throw) for throw in self.state.throws
            ],
        }
        self._actions = copy.deepcopy(checkpoint.get("actions", []))
        self._next_action_id = int(
            checkpoint.get(
                "next_action_id",
                max(
                    [
                        int(action.get("id", 0))
                        for action in self._actions
                    ]
                    or [0]
                )
                + 1,
            )
        )
        self.last_action = None
        self.last_undo_action = None
        return self.state

    @staticmethod
    def _throw_to_dict(throw: ThrowEvent) -> Dict[str, Any]:
        return {
            "seq": throw.seq,
            "type": throw.type,
            "label": throw.label,
            "score": throw.score,
            "player_id": throw.player_id,
            "raw": throw.raw,
            "snapshot_before": throw.snapshot_before,
            "context_before": throw.context_before,
            "round_number": throw.round_number,
            "dart_in_turn": throw.dart_in_turn,
            "mode_points": throw.mode_points,
            "outcome": throw.outcome,
            "source": throw.source,
            "score_after": throw.score_after,
        }

    @staticmethod
    def _throw_from_dict(data: Dict[str, Any]) -> ThrowEvent:
        return ThrowEvent(
            seq=int(data["seq"]),
            type=str(data.get("type", "throw")),
            label=str(data.get("label", "")),
            score=int(data.get("score", 0)),
            player_id=data.get("player_id"),
            raw=dict(data.get("raw", {})),
            snapshot_before=dict(data["snapshot_before"]),
            context_before=dict(data.get("context_before", {})),
            round_number=int(data.get("round_number", 1)),
            dart_in_turn=int(data.get("dart_in_turn", 1)),
            mode_points=int(data.get("mode_points", 0)),
            outcome=str(data.get("outcome", "neutral")),
            source=str(data.get("source", "unknown")),
            score_after=int(data.get("score_after", 0)),
        )

    def _new_action(self, kind: str, **data: Any) -> Dict[str, Any]:
        if self._replaying:
            action = copy.deepcopy(getattr(self, "_replay_action", {}))
            if action.get("kind") != kind:
                raise ValueError(
                    f"Replay expected {action.get('kind')}, received {kind}"
                )
            return action
        if self._history_origin is None:
            self._history_origin = {
                "state": self.state.snapshot(),
                "random_state": random.getstate(),
                "throws": [
                    self._throw_to_dict(throw)
                    for throw in self.state.throws
                ],
            }
        action = {"id": self._next_action_id, "kind": kind, **data}
        self._next_action_id += 1
        return action

    def _commit_action(self, action: Dict[str, Any]) -> None:
        if self._replaying:
            return
        stored = copy.deepcopy(action)
        self._actions.append(stored)
        self.last_action = copy.deepcopy(stored)
        self.last_undo_action = None

    @staticmethod
    def _tuple_tree(value: Any) -> Any:
        if isinstance(value, list):
            return tuple(GameEngine._tuple_tree(item) for item in value)
        if isinstance(value, tuple):
            return tuple(GameEngine._tuple_tree(item) for item in value)
        return value

    def _replay_actions(self) -> None:
        if not self._history_origin:
            raise ValueError("This game has no editable action history")
        actions = copy.deepcopy(self._actions)
        self.state.restore_snapshot(
            copy.deepcopy(self._history_origin["state"])
        )
        self.state.throws = [
            self._throw_from_dict(data)
            for data in self._history_origin.get("throws", [])
        ]
        self.state.last_event = None
        random.setstate(
            self._tuple_tree(self._history_origin["random_state"])
        )
        self._replaying = True
        try:
            for action in actions:
                self._replay_action = action
                kind = action["kind"]
                if kind == "throw":
                    self.handle_event(copy.deepcopy(action["event"]))
                elif kind == "continue":
                    if self.state.status == "hold":
                        self.continue_turn()
                    elif self.state.status == "running":
                        # A previously completed turn remains a turn boundary
                        # when one of its darts is later deleted.
                        self._complete_skipped_turn()
                        if self.state.status != "finished":
                            self._advance_player()
                            self.state.status = "running"
                            self.state.last_event = {"type": "continue"}
                elif kind == "next_player":
                    self.next_player()
                elif kind == "game_action":
                    self.handle_action(
                        str(action["action"]),
                        copy.deepcopy(action.get("payload", {})),
                    )
        finally:
            self._replaying = False
            self._replay_action = {}
        self.last_action = copy.deepcopy(actions[-1]) if actions else None

    def editable_turns(self, limit: int = 2) -> List[Dict[str, Any]]:
        """Return the current and immediately previous turn for operator edits."""
        players = {player.id: player for player in self.state.players}
        groups: List[Dict[str, Any]] = []
        for throw in self.state.throws:
            action_id = int(throw.raw.get("_action_id", 0) or 0)
            if not action_id:
                continue
            key = (throw.round_number, throw.player_id)
            if not groups or groups[-1]["_key"] != key:
                player = players.get(throw.player_id or "")
                groups.append({
                    "_key": key,
                    "round_number": throw.round_number,
                    "player_id": throw.player_id,
                    "player_name": player.name if player else "",
                    "current": False,
                    "darts": [],
                })
            groups[-1]["darts"].append({
                "action_id": action_id,
                "dart_in_turn": throw.dart_in_turn,
                "label": throw.label or "MISS",
                "score": throw.score,
                "source": throw.source,
            })

        current = self.state.current_player()
        current_key = (
            self.state.round_number,
            current.id if current else None,
        )
        if groups and groups[-1]["_key"] == current_key:
            groups[-1]["current"] = True
        elif current:
            groups.append({
                "_key": current_key,
                "round_number": self.state.round_number,
                "player_id": current.id,
                "player_name": current.name,
                "current": True,
                "darts": [],
            })
        selected = groups[-max(1, int(limit)):]
        for group in selected:
            group.pop("_key", None)
            group["can_add"] = bool(
                group["current"] and self.state.status == "running"
            )
        return selected

    def correct_throw(self, action_id: int, replacement: Dict[str, Any]) -> GameState:
        editable_ids = {
            int(dart["action_id"])
            for turn in self.editable_turns()
            for dart in turn["darts"]
        }
        if action_id not in editable_ids:
            raise ValueError("Only darts from the last two turns can be corrected")
        target = next(
            (
                action
                for action in self._actions
                if action["kind"] == "throw" and int(action["id"]) == action_id
            ),
            None,
        )
        if target is None:
            raise ValueError("Throw no longer exists")
        corrected = copy.deepcopy(replacement)
        corrected["seq"] = target["event"].get(
            "seq", corrected.get("seq", -1)
        )
        corrected["corrected"] = True
        corrected["_source"] = "correction"
        corrected["_action_id"] = action_id
        target["event"] = corrected
        self._replay_actions()
        self.last_action = {
            "id": action_id,
            "kind": "throw_corrected",
            "event": copy.deepcopy(corrected),
        }
        return self.state

    def delete_throw(self, action_id: int) -> GameState:
        editable_ids = {
            int(dart["action_id"])
            for turn in self.editable_turns()
            for dart in turn["darts"]
        }
        if action_id not in editable_ids:
            raise ValueError("Only darts from the last two turns can be deleted")
        before = len(self._actions)
        self._actions = [
            action
            for action in self._actions
            if not (
                action["kind"] == "throw"
                and int(action["id"]) == action_id
            )
        ]
        if len(self._actions) == before:
            raise ValueError("Throw no longer exists")
        self._replay_actions()
        self.last_action = {"id": action_id, "kind": "throw_deleted"}
        return self.state

    def continue_turn(self) -> GameState:
        if self.state.status == "hold":
            action = self._new_action("continue")
            previous_message = self.state.message
            self._advance_player()
            self.state.last_event = {"type": "continue"}
            if self.state.status != "finished":
                self.state.status = "running"
                if self.state.message == previous_message:
                    self.state.message = "Next player"
            self._commit_action(action)
        return self.state

    def next_player(self) -> GameState:
        if self.state.status in ("running", "hold"):
            action = self._new_action("next_player")
            if self.state.status == "running":
                self._complete_skipped_turn()
            if self.state.status == "finished":
                self.state.last_event = {"type": "next_player"}
                self._commit_action(action)
                return self.state
            previous_message = self.state.message
            self._advance_player()
            self.state.last_event = {"type": "next_player"}
            if self.state.status != "finished":
                self.state.status = "running"
                if self.state.message == previous_message:
                    self.state.message = "Next player"
            self._commit_action(action)
        return self.state

    def _complete_skipped_turn(self) -> None:
        """Apply turn-end rules when the operator skips the remaining darts."""
        player = self.state.current_player()
        if player is None:
            return
        mode = registry.get(self.state.game_type)
        hook = getattr(mode, "on_turn_skipped", None)
        if hook is not None:
            hook(self.state, player)
        if self.state.status != "running" or "rounds" not in self.state.options:
            return
        from .games.arcade import finish_action_round_game

        finish_action_round_game(
            self.state,
            f"{{winner}} gewinnt {mode.metadata.title}!",
        )

    def undo(self) -> GameState:
        if not self._actions:
            if not self.state.throws:
                self.last_undo_action = None
                return self.state
            last = self.state.throws.pop()
            self.state.restore_snapshot(last.snapshot_before)
            self.last_undo_action = {
                "id": 0,
                "kind": "throw",
                "event": copy.deepcopy(last.raw),
                "legacy": True,
            }
            self.state.last_event = {
                "type": "undo",
                "undone": last.label,
            }
            self.state.message = f"Undo {last.label}"
            return self.state
        undone = self._actions.pop()
        self.last_undo_action = copy.deepcopy(undone)
        self._replay_actions()
        label = (
            str(undone.get("event", {}).get("label") or "MISS")
            if undone["kind"] == "throw"
            else undone["kind"]
        )
        self.state.last_event = {
            "type": "undo",
            "undone": label,
            "action_id": undone["id"],
        }
        self.state.message = f"Undo {label}"
        return self.state

    def correct_turn_throw(
        self,
        turn_index: int,
        replacement: Dict[str, Any],
    ) -> GameState:
        """Compatibility wrapper for the original active-turn correction API."""
        current = next(
            (
                turn
                for turn in reversed(self.editable_turns())
                if turn["current"]
            ),
            None,
        )
        if current is None or turn_index < 0 or turn_index >= len(current["darts"]):
            raise ValueError("Throw index is outside the current turn")
        return self.correct_throw(
            int(current["darts"][turn_index]["action_id"]),
            replacement,
        )

    def handle_action(self, action: str, payload: Dict[str, Any] | None = None) -> GameState:
        self.state.last_event = {"type": "game_action", "action": action, "payload": payload or {}}
        mode = registry.get(self.state.game_type)
        handler = getattr(mode, "handle_action", None)
        if handler is None:
            raise ValueError(f"Game mode {self.state.game_type} does not support actions")
        timeline_action = self._new_action(
            "game_action",
            action=action,
            payload=copy.deepcopy(payload or {}),
        )
        handler(self.state, action, payload or {})
        self._commit_action(timeline_action)
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
            self._apply_timeline_throw(event)
        elif event_type == "miss":
            miss = {**event, "label": "MISS", "score": 0}
            self.state.last_event = miss
            self._apply_timeline_throw(miss)
        return self.state

    def _apply_timeline_throw(self, event: Dict[str, Any]) -> None:
        action = self._new_action("throw", event=copy.deepcopy(event))
        event["_action_id"] = action["id"]
        action["event"]["_action_id"] = action["id"]
        self._apply_throw(event)
        self._commit_action(action)

    def _advance_player(self) -> None:
        players = self.state.players
        if not players:
            return
        mode = registry.get(self.state.game_type)
        eligibility = getattr(mode, "is_player_active", None)
        start_index = self.state.current_player_index
        next_index = start_index
        for offset in range(1, len(players) + 1):
            candidate_index = (start_index + offset) % len(players)
            candidate = players[candidate_index]
            if eligibility is None or eligibility(self.state, candidate):
                next_index = candidate_index
                break
        if next_index <= start_index:
            self.state.round_number += 1
        self.state.current_player_index = next_index
        self.state.darts_in_turn = 0
        self.state.turn_score = 0
        player = self.state.current_player()
        if player:
            self.state.turn_start_values[player.id] = player.score
            mode = registry.get(self.state.game_type)
            hook = getattr(mode, "on_turn_start", None)
            if hook is not None:
                hook(self.state, player)

    def _hold_after_turn(self) -> None:
        if self.state.status == "running" and self.state.darts_in_turn >= 3:
            self.state.status = "hold"
            self.state.message = "Turn complete. Press continue."

    def _apply_throw(self, event: Dict[str, Any]) -> None:
        player = self.state.current_player()
        if player is None:
            return
        snapshot = self.state.snapshot()
        context = self.state.telemetry_context()
        round_number = self.state.round_number
        dart_in_turn = self.state.darts_in_turn + 1
        label = str(event.get("label", ""))
        score = int(event.get("score", 0))
        outcome = registry.get(self.state.game_type).apply_throw(
            self.state, player, event
        )
        # A player's stored score is not directionally comparable across
        # modes (X01 counts down, Mini Golf counts strokes, Risk It defers its
        # bank). ThrowOutcome.turn_value is the plugin's canonical value for
        # this individual dart.
        mode_points = int(outcome.turn_value)
        event_type = event.get("type")
        targets = context.get("targets", [])
        target_match = self._context_match(event, targets)
        danger_match = self._context_match(event, context.get("dangers", []))
        if event_type == "miss":
            outcome_kind = "miss"
        elif outcome.bust or mode_points < 0 or danger_match:
            outcome_kind = "danger"
        elif target_match or (mode_points > 0 and not targets):
            outcome_kind = "success"
        elif mode_points > 0:
            outcome_kind = "partial"
        else:
            outcome_kind = "neutral"
        event["mode_points"] = mode_points
        event["outcome"] = outcome_kind

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
                context_before=context,
                round_number=round_number,
                dart_in_turn=dart_in_turn,
                mode_points=mode_points,
                outcome=outcome_kind,
                source=str(event.get("_source", "unknown")),
                score_after=player.score,
            )
        )
        self.state.message = outcome.message
        if outcome.finished:
            self.state.status = "finished"
            self.state.winner_id = outcome.winner_id
            self.state.winner_ids = list(
                outcome.winner_ids
                or ([outcome.winner_id] if outcome.winner_id else [])
            )
            self.state.result_type = outcome.result_type or (
                "individual_win"
                if self.state.winner_ids
                else (
                    "draw"
                    if outcome.message.startswith("Unentschieden")
                    else "challenge_loss"
                )
            )
        elif outcome.force_hold:
            self.state.status = "hold"
        else:
            self._hold_after_turn()

    @staticmethod
    def _context_match(event: Dict[str, Any], zones: List[Dict[str, Any]]) -> bool:
        field = event.get("field")
        ring = event.get("ring")
        for zone in zones:
            if int(zone.get("field", -1) or -1) != int(field or -2):
                continue
            rings = zone.get("rings") or (
                [zone.get("ring")] if zone.get("ring") else []
            )
            if not rings or ring in rings:
                return True
        return False
