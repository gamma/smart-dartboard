from __future__ import annotations

import asyncio
import os
import secrets
from collections import deque
from pathlib import Path
from time import monotonic
from typing import Any, Callable, Deque, Dict, Iterable, Optional, Tuple

from .game import GameEngine
from .games import registry
from .storage import DartboardStore

SCREENS = {
    "attract",
    "players",
    "game_select",
    "instructions",
    "countdown",
    "playing",
    "game_result",
    "session_summary",
    "calibration",
}
REMATCH_CONFIRM_SECONDS = 3.0
CORRECTION_LOCK_SECONDS = 60.0
HIT_MISS_DEBOUNCE_SECONDS = 1.0


class SessionController:
    """Owns the durable session and screen state for every connected display."""

    def __init__(
        self,
        database_path: Path | str,
        engine: Optional[GameEngine] = None,
        clock: Optional[Callable[[], float]] = None,
    ) -> None:
        self.store = DartboardStore(database_path)
        self.engine = engine or GameEngine()
        self._clock = clock or monotonic
        self._rematch_armed_until = 0.0
        self._correction_locked_until = 0.0
        self.screen = "attract"
        self.session_id: Optional[str] = None
        self.game_id: Optional[str] = None
        self.selected_mode: Optional[str] = None
        self.selected_options: Dict[str, Any] = {}
        self.default_starter_id: Optional[str] = None
        self.selected_starter_id: Optional[str] = None
        self.starter_selection = "rotation"
        self.calibration = {
            "corners": [
                {"x": 0.247, "y": 0.05},
                {"x": 0.753, "y": 0.05},
                {"x": 0.753, "y": 0.95},
                {"x": 0.247, "y": 0.95},
            ],
            "scale": 1.0,
            "offset_x": 0.0,
            "offset_y": 0.0,
        }
        self.projector_geometry = {"width": 1600, "height": 900}
        self.sound = {"enabled": False, "status": "disabled"}
        self.art_theme = "cartoon"
        self.ui_language = "de"
        self.hardware: Dict[str, Any] = {"enabled": False, "status": "disabled"}
        self._restore()

    def close(self) -> None:
        self.store.close()

    def _restore(self) -> None:
        runtime = self.store.get_runtime_value("experience")
        checkpoint = self.store.get_runtime_value("engine")
        if runtime:
            self.screen = runtime.get("screen", "attract")
            self.session_id = runtime.get("session_id")
            self.game_id = runtime.get("game_id")
            self.selected_mode = runtime.get("selected_mode")
            self.selected_options = dict(runtime.get("selected_options", {}))
            self.default_starter_id = runtime.get("default_starter_id")
            self.selected_starter_id = runtime.get("selected_starter_id")
            stored_selection = runtime.get("starter_selection", "rotation")
            self.starter_selection = (
                stored_selection
                if stored_selection in {"rotation", "manual", "random"}
                else "rotation"
            )
        if checkpoint:
            self.engine.import_state(checkpoint)
        self.calibration = self.store.get_runtime_value("calibration", self.calibration)
        stored_sound = self.store.get_runtime_value("sound", self.sound)
        sound_enabled = bool(stored_sound.get("enabled", False))
        self.sound = {
            "enabled": sound_enabled,
            "status": "starting" if sound_enabled else "disabled",
        }
        stored_theme = self.store.get_runtime_value("art_theme", "cartoon")
        self.art_theme = stored_theme if stored_theme in {"cartoon", "neon"} else "cartoon"
        stored_language = self.store.get_runtime_value("ui_language", "de")
        self.ui_language = stored_language if stored_language in {"de", "en"} else "de"
        self.store.reconcile_running_games(self.game_id)

    def _persist(self) -> None:
        self.store.set_runtime_value(
            "experience",
            {
                "screen": self.screen,
                "session_id": self.session_id,
                "game_id": self.game_id,
                "selected_mode": self.selected_mode,
                "selected_options": self.selected_options,
                "default_starter_id": self.default_starter_id,
                "selected_starter_id": self.selected_starter_id,
                "starter_selection": self.starter_selection,
            },
        )
        self.store.set_runtime_value("engine", self.engine.export_state())

    def public_state(self) -> Dict[str, Any]:
        session = (
            self.store.get_session(self.session_id)
            if self.session_id
            else None
        )
        self._ensure_starter(session)
        player_ids = [player["id"] for player in session["players"]] if session else []
        rematch_remaining = max(0.0, self._rematch_armed_until - self._clock())
        rematch_armed = (
            rematch_remaining > 0
            and self.screen == "game_result"
            and self.engine.state.status == "finished"
        )
        return {
            "screen": self.screen,
            "session": session,
            "game_id": self.game_id,
            "selected_mode": self.selected_mode,
            "selected_options": self.selected_options,
            "starter": {
                "player_id": self.selected_starter_id,
                "default_player_id": self.default_starter_id,
                "selection": self.starter_selection,
            },
            "ui_language": self.ui_language,
            "game": self.engine.state.as_dict(),
            "editable_turns": self.engine.editable_turns(limit=1),
            "modes": registry.as_dicts(),
            "players": self.store.list_players(),
            "statistics": self.store.statistics(player_ids or None),
            "session_statistics": (
                self.store.session_statistics(self.session_id)
                if self.session_id
                else []
            ),
            "calibration": self.calibration,
            "projector_geometry": self.projector_geometry,
            "sound": self.sound,
            "art_theme": self.art_theme,
            "hardware": self.hardware,
            "rematch": {
                "armed": rematch_armed,
                "expires_in_ms": round(rematch_remaining * 1000)
                if rematch_armed
                else 0,
            },
            "correction_lock": {
                "active": self.correction_locked(),
            },
        }

    def correction_locked(self) -> bool:
        return self._clock() < self._correction_locked_until

    def set_correction_lock(self, enabled: bool) -> None:
        self._correction_locked_until = (
            self._clock() + CORRECTION_LOCK_SECONDS
            if enabled
            else 0.0
        )

    def create_player(self, name: str, avatar: str, color: str) -> Dict[str, Any]:
        return self.store.create_player(name, avatar, color)

    def show_player_selection(self) -> None:
        self.screen = "players"
        self._persist()

    def _ensure_starter(
        self,
        session: Optional[Dict[str, Any]] = None,
    ) -> None:
        if session is None and self.session_id:
            session = self.store.get_session(self.session_id)
        player_ids = [
            player["id"]
            for player in (session or {}).get("players", [])
        ]
        if not player_ids:
            self.default_starter_id = None
            self.selected_starter_id = None
            self.starter_selection = "rotation"
            return
        if self.default_starter_id not in player_ids:
            self.default_starter_id = player_ids[0]
        if self.selected_starter_id not in player_ids:
            self.selected_starter_id = self.default_starter_id
            self.starter_selection = "rotation"

    def _next_session_player_id(self, player_id: Optional[str]) -> Optional[str]:
        session = self.store.get_session(self.session_id) if self.session_id else None
        player_ids = [
            player["id"]
            for player in (session or {}).get("players", [])
        ]
        if not player_ids:
            return None
        if player_id not in player_ids:
            return player_ids[0]
        return player_ids[(player_ids.index(player_id) + 1) % len(player_ids)]

    def select_starter(
        self,
        *,
        player_id: Optional[str] = None,
        randomize: bool = False,
    ) -> str:
        if self.screen != "instructions" or not self.selected_mode:
            raise ValueError("A starter can only be selected before a game")
        session = self.store.get_session(self.session_id) if self.session_id else None
        players = list((session or {}).get("players", []))
        if not players:
            raise ValueError("An active session is required")
        player_ids = [player["id"] for player in players]
        if randomize:
            self.selected_starter_id = secrets.choice(player_ids)
            self.starter_selection = "random"
        elif player_id in player_ids:
            self.selected_starter_id = player_id
            self.starter_selection = "manual"
        else:
            raise ValueError("Selected starter is not part of the session")
        self._persist()
        return self.selected_starter_id

    def start_session(
        self,
        player_ids: Iterable[str],
    ) -> Dict[str, Any]:
        active = self.store.active_session()
        if active:
            self.store.end_session(active["id"])
        session = self.store.start_session(player_ids)
        self.session_id = session["id"]
        self.game_id = None
        self.selected_mode = None
        self.selected_options = {}
        self.default_starter_id = session["players"][0]["id"]
        self.selected_starter_id = self.default_starter_id
        self.starter_selection = "rotation"
        self.screen = "game_select"
        self._persist()
        return session

    def prepare_game(self, game_type: str, options: Dict[str, Any]) -> None:
        if not self.session_id:
            raise ValueError("Start a session before selecting a game")
        mode = registry.get(game_type)
        defaults = mode.metadata.resolve_options(options)
        session = self.store.get_session(self.session_id)
        player_count = len(session["players"]) if session else 0
        if not mode.metadata.min_players <= player_count <= mode.metadata.max_players:
            raise ValueError(
                f"{mode.metadata.title} supports "
                f"{mode.metadata.min_players}–{mode.metadata.max_players} players"
            )
        self.selected_mode = game_type
        self.selected_options = defaults
        self._ensure_starter(session)
        self.screen = "instructions"
        self._persist()

    def start_game(self) -> None:
        if not self.session_id or not self.selected_mode:
            raise ValueError("Session and game selection are required")
        session = self.store.get_session(self.session_id)
        if not session:
            raise ValueError("Active session no longer exists")
        self._ensure_starter(session)
        players = list(session["players"])
        starter_index = next(
            (
                index
                for index, player in enumerate(players)
                if player["id"] == self.selected_starter_id
            ),
            0,
        )
        ordered_players = players[starter_index:] + players[:starter_index]
        self.engine.reset(
            self.selected_mode,
            ordered_players,
            options=self.selected_options,
        )
        self.game_id = self.store.start_game(
            self.session_id,
            self.selected_mode,
            self.selected_options,
            players=[
                {
                    "id": player.id,
                    "name": player.name,
                    "avatar": player.avatar,
                    "color": player.color,
                }
                for player in self.engine.state.players
            ],
            ruleset_version=registry.get(
                self.selected_mode
            ).metadata.ruleset_version,
            app_version=os.environ.get("SDB_VERSION", "dev"),
            # A game starts as production. The event pipeline marks it as test
            # as soon as the projector simulator injects a synthetic throw.
            environment="production",
            initial_state=self.engine.state.replay_frame(),
        )
        self.store.record_game_event(
            self.game_id,
            "game_started",
            payload={
                "game_type": self.selected_mode,
                "options": self.selected_options,
                "starter_id": self.selected_starter_id,
                "starter_selection": self.starter_selection,
            },
            frame=self.engine.state.replay_frame(),
        )
        self._rematch_armed_until = 0.0
        self.screen = "countdown"
        self._persist()

    def set_screen(self, screen: str) -> None:
        if screen not in SCREENS:
            raise ValueError(f"Unknown screen: {screen}")
        if screen == "playing" and self.engine.state.status != "running":
            raise ValueError("Playing screen requires a running game")
        self.screen = screen
        self._persist()

    def process_event(self, event: Dict[str, Any]) -> None:
        if (
            event.get("type") == "button"
            and event.get("button") == "menu"
            and event.get("action") == "press"
            and self.screen == "game_result"
            and self.engine.state.status == "finished"
        ):
            self.rematch_button()
            return
        before_count = len(self.engine.state.throws)
        self.engine.handle_event(event)
        if len(self.engine.state.throws) > before_count and self.game_id:
            throw = self.engine.state.throws[-1]
            event_id = self.store.record_game_event(
                self.game_id,
                "throw",
                player_id=throw.player_id,
                source=throw.source,
                payload=throw.raw,
                task=throw.context_before,
                frame=self.engine.state.replay_frame(),
            )
            self.store.record_throw(
                self.game_id,
                throw.seq,
                throw.player_id,
                throw.raw,
                throw.score_after,
                round_number=throw.round_number,
                dart_in_turn=throw.dart_in_turn,
                mode_points=throw.mode_points,
                outcome=throw.outcome,
                source=throw.source,
                task=throw.context_before,
                event_id=event_id,
            )
        self._finish_game_if_needed()
        self._persist()

    def _record_event(
        self,
        event_type: str,
        *,
        payload: Optional[Dict[str, Any]] = None,
        source: str = "control",
        corrects_event_id: Optional[int] = None,
    ) -> Optional[int]:
        if not self.game_id:
            return None
        return self.store.record_game_event(
            self.game_id,
            event_type,
            player_id=self.engine.state.current_player().id
            if self.engine.state.players
            else None,
            source=source,
            payload=payload,
            frame=self.engine.state.replay_frame(),
            corrects_event_id=corrects_event_id,
        )

    def _finish_game_if_needed(self, *, force: bool = False) -> bool:
        if self.engine.state.status != "finished" or not self.game_id:
            return False
        game = self.store.get_game(self.game_id)
        if game and (force or game["status"] != "finished"):
            final_scores = {
                player.id: player.score for player in self.engine.state.players
            }
            self._record_event(
                "game_finished",
                payload={
                    "winner_ids": self.engine.state.winner_ids,
                    "result_type": self.engine.state.result_type,
                    "message": self.engine.state.message,
                },
                source="system",
            )
            self.store.finish_game(
                self.game_id,
                self.engine.state.winner_id,
                self.engine.state.winner_ids,
                result_type=self.engine.state.result_type,
                finish_reason=(
                    self.engine.state.last_event.get("effect", "")
                    if self.engine.state.last_event
                    else ""
                )
                or "completed",
                final_state=self.engine.state.replay_frame(),
                final_scores=final_scores,
            )
        self.screen = "game_result"
        return True

    def rematch_button(self) -> bool:
        """Arm or start a same-mode rematch from the physical board button."""
        if (
            not self.session_id
            or not self.game_id
            or not self.selected_mode
            or self.screen != "game_result"
            or self.engine.state.status != "finished"
        ):
            raise ValueError("A rematch requires a finished session game")
        previous_game = self.store.get_game(self.game_id)
        if not previous_game or previous_game["status"] != "finished":
            raise ValueError("The previous game must be stored as finished")

        now = self._clock()
        if now >= self._rematch_armed_until:
            self._rematch_armed_until = now + REMATCH_CONFIRM_SECONDS
            self._persist()
            return False

        session = self.store.get_session(self.session_id)
        if not session:
            raise ValueError("Active session no longer exists")
        players_by_id = {player["id"]: player for player in session["players"]}
        previous_order = [
            player.id
            for player in self.engine.state.players
            if player.id in players_by_id
        ]
        if len(previous_order) != len(players_by_id):
            previous_order = [player["id"] for player in session["players"]]
        rotated_order = previous_order[1:] + previous_order[:1]
        rotated_players = [players_by_id[player_id] for player_id in rotated_order]

        self.engine.reset(
            self.selected_mode,
            rotated_players,
            options=self.selected_options,
        )
        self.default_starter_id = rotated_order[0]
        self.selected_starter_id = rotated_order[0]
        self.starter_selection = "rotation"
        self.game_id = self.store.start_game(
            self.session_id,
            self.selected_mode,
            self.selected_options,
            players=[
                {
                    "id": player.id,
                    "name": player.name,
                    "avatar": player.avatar,
                    "color": player.color,
                }
                for player in self.engine.state.players
            ],
            ruleset_version=registry.get(
                self.selected_mode
            ).metadata.ruleset_version,
            app_version=os.environ.get("SDB_VERSION", "dev"),
            environment="production",
            initial_state=self.engine.state.replay_frame(),
        )
        self._record_event(
            "game_started",
            payload={
                "game_type": self.selected_mode,
                "options": self.selected_options,
                "rematch": True,
                "starter_id": rotated_order[0],
                "starter_selection": "rotation",
            },
            source="system",
        )
        self._rematch_armed_until = 0.0
        self.screen = "countdown"
        self._persist()
        return True

    def continue_turn(self) -> None:
        self.engine.continue_turn()
        self._record_event(
            "continue_turn",
            payload=self._last_action_payload(),
        )
        self._finish_game_if_needed()
        self._persist()

    def next_player(self) -> None:
        self.engine.next_player()
        self._record_event(
            "next_player",
            payload=self._last_action_payload(),
        )
        self._finish_game_if_needed()
        self._persist()

    def undo(self) -> None:
        was_finished = self.engine.state.status == "finished"
        self.engine.undo()
        undone = self.engine.last_undo_action
        if undone and self.game_id:
            action_id = int(undone["id"])
            corrected_event_id = (
                self.store.invalidate_gameplay_event(
                    self.game_id,
                    action_id=action_id,
                )
                if action_id
                else self.store.invalidate_last_throw_event(self.game_id)
            )
            if undone["kind"] == "throw":
                self._replace_persisted_throws()
            self._record_event(
                "undo",
                payload={
                    "kind": undone["kind"],
                    "action_id": undone["id"],
                },
                corrects_event_id=corrected_event_id,
            )
        if was_finished and self.game_id and self.engine.state.status != "finished":
            self.store.reopen_game(self.game_id)
            self.screen = "playing"
        self._persist()

    def correct_turn_throw(
        self,
        turn_index: int,
        replacement: Dict[str, Any],
    ) -> None:
        current = next(
            (
                turn
                for turn in reversed(self.engine.editable_turns())
                if turn["current"]
            ),
            None,
        )
        if current is None or turn_index >= len(current["darts"]):
            raise ValueError("Throw index is outside the current turn")
        self.correct_throw(
            int(current["darts"][turn_index]["action_id"]),
            replacement,
        )

    def correct_throw(
        self,
        action_id: int,
        replacement: Dict[str, Any],
    ) -> None:
        was_finished = self.engine.state.status == "finished"
        self.engine.correct_throw(action_id, replacement)
        if self.game_id:
            corrected_event_id = self.store.invalidate_gameplay_event(
                self.game_id,
                action_id=action_id,
            )
            self._replace_persisted_throws()
            self._record_event(
                "throw_corrected",
                payload={
                    "action_id": action_id,
                    "replacement": replacement,
                },
                corrects_event_id=corrected_event_id,
            )
            if self.engine.state.status == "finished":
                self._finish_game_if_needed(force=True)
            elif was_finished:
                self.store.reopen_game(self.game_id)
                self.screen = "playing"
        self._persist()

    def delete_throw(self, action_id: int) -> None:
        was_finished = self.engine.state.status == "finished"
        self.engine.delete_throw(action_id)
        if self.game_id:
            corrected_event_id = self.store.invalidate_gameplay_event(
                self.game_id,
                action_id=action_id,
            )
            self._replace_persisted_throws()
            self._record_event(
                "throw_deleted",
                payload={"action_id": action_id},
                corrects_event_id=corrected_event_id,
            )
            if self.engine.state.status == "finished":
                self._finish_game_if_needed(force=True)
            elif was_finished:
                self.store.reopen_game(self.game_id)
                self.screen = "playing"
        self._persist()

    def manual_throw(self, event: Dict[str, Any]) -> None:
        enriched = {**event, "_source": "manual"}
        self.process_event(enriched)
        event.update({
            key: value
            for key, value in enriched.items()
            if not key.startswith("_")
        })

    def _last_action_payload(self) -> Dict[str, Any]:
        action = self.engine.last_action or {}
        return {"_action_id": action.get("id")} if action.get("id") else {}

    def _replace_persisted_throws(self) -> None:
        if not self.game_id:
            return
        self.store.replace_game_throws(
            self.game_id,
            [
                {
                    "seq": throw.seq,
                    "player_id": throw.player_id,
                    "event": throw.raw,
                    "score_after": throw.score_after,
                    "round_number": throw.round_number,
                    "dart_in_turn": throw.dart_in_turn,
                    "field": throw.raw.get("field"),
                    "ring": throw.raw.get("ring"),
                    "multiplier": throw.raw.get("multiplier"),
                    "dart_score": throw.score,
                    "mode_points": throw.mode_points,
                    "outcome": throw.outcome,
                    "source": throw.source,
                    "task": throw.context_before,
                }
                for throw in self.engine.state.throws
            ],
        )

    def game_action(self, action: str, payload: Dict[str, Any] | None = None) -> None:
        self.engine.handle_action(action, payload or {})
        self._record_event(
            "game_action",
            payload={
                "action": action,
                **(payload or {}),
                **self._last_action_payload(),
            },
        )
        self._finish_game_if_needed()
        self._persist()

    def next_game(self) -> None:
        if (
            self.game_id
            and self.engine.state.status == "finished"
            and self.engine.state.players
        ):
            self.default_starter_id = self._next_session_player_id(
                self.engine.state.players[0].id
            )
        self._rematch_armed_until = 0.0
        self.engine.clear()
        self.game_id = None
        self.selected_mode = None
        self.selected_options = {}
        self.selected_starter_id = self.default_starter_id
        self.starter_selection = "rotation"
        self.screen = "game_select"
        self._persist()

    def abort_game(self) -> None:
        if not self.session_id or not self.game_id:
            raise ValueError("There is no active game to abort")
        game = self.store.get_game(self.game_id)
        if not game or game["status"] != "running":
            raise ValueError("Only a running game can be aborted")
        self._record_event(
            "game_aborted",
            payload={"reason": "user_abort"},
            source="control",
        )
        self.store.abort_game(
            self.game_id,
            reason="user_abort",
            final_state=self.engine.state.replay_frame(),
        )
        self._rematch_armed_until = 0.0
        self.engine.clear()
        self.game_id = None
        self.selected_mode = None
        self.selected_options = {}
        self.selected_starter_id = self.default_starter_id
        self.starter_selection = "rotation"
        self.screen = "game_select"
        self._persist()

    def end_session(self) -> None:
        if self.screen != "game_select":
            raise ValueError("Return to the game selection before ending the session")
        if self.session_id:
            self.store.end_session(self.session_id)
        self.screen = "session_summary"
        self._persist()

    def reset_to_attract(self) -> None:
        self._rematch_armed_until = 0.0
        self.screen = "attract"
        self.session_id = None
        self.game_id = None
        self.selected_mode = None
        self.selected_options = {}
        self.default_starter_id = None
        self.selected_starter_id = None
        self.starter_selection = "rotation"
        self._persist()

    def save_calibration(self, calibration: Dict[str, Any]) -> None:
        corners = calibration.get("corners")
        if not isinstance(corners, list) or len(corners) != 4:
            raise ValueError("Calibration needs four corners")
        self.calibration = {
            "corners": [
                {"x": float(point["x"]), "y": float(point["y"])}
                for point in corners
            ],
            "scale": float(calibration.get("scale", 1.0)),
            "offset_x": float(calibration.get("offset_x", 0.0)),
            "offset_y": float(calibration.get("offset_y", 0.0)),
        }
        self.store.set_runtime_value("calibration", self.calibration)

    def report_projector_geometry(self, width: int, height: int) -> None:
        self.projector_geometry = {
            "width": max(320, int(width)),
            "height": max(240, int(height)),
        }

    def reset_calibration(self) -> Dict[str, Any]:
        """Center the projection as the largest safe square for the viewport."""
        width = float(self.projector_geometry["width"])
        height = float(self.projector_geometry["height"])
        side = min(width, height) * 0.9
        half_x = side / width / 2
        half_y = side / height / 2
        calibration = {
            "corners": [
                {"x": 0.5 - half_x, "y": 0.5 - half_y},
                {"x": 0.5 + half_x, "y": 0.5 - half_y},
                {"x": 0.5 + half_x, "y": 0.5 + half_y},
                {"x": 0.5 - half_x, "y": 0.5 + half_y},
            ],
            "scale": 1.0,
            "offset_x": 0.0,
            "offset_y": 0.0,
        }
        self.save_calibration(calibration)
        return self.calibration

    def set_sound_enabled(self, enabled: bool) -> None:
        self.sound = {
            "enabled": bool(enabled),
            "status": "starting" if enabled else "disabled",
        }
        self.store.set_runtime_value("sound", self.sound)

    def report_sound_status(self, status: str) -> None:
        if status not in {"ready", "blocked", "unavailable"}:
            raise ValueError(f"Unknown sound status: {status}")
        self.sound = {**self.sound, "status": status}
        self.store.set_runtime_value("sound", self.sound)

    def set_art_theme(self, theme: str) -> None:
        if theme not in {"cartoon", "neon"}:
            raise ValueError(f"Unknown artwork theme: {theme}")
        self.art_theme = theme
        self.store.set_runtime_value("art_theme", theme)

    def set_ui_language(self, language: str) -> None:
        if language not in {"de", "en"}:
            raise ValueError(f"Unknown UI language: {language}")
        self.ui_language = language
        self.store.set_runtime_value("ui_language", language)


class EventPipeline:
    """Serializes events and drops repeated BLE notifications."""

    def __init__(
        self,
        controller: SessionController,
        history_size: int = 256,
        clock: Callable[[], float] = monotonic,
        hit_miss_debounce_seconds: float = HIT_MISS_DEBOUNCE_SECONDS,
    ) -> None:
        self.controller = controller
        self._lock = asyncio.Lock()
        self._recent: Deque[Tuple[int, str]] = deque(maxlen=history_size)
        self._recent_set: set[Tuple[int, str]] = set()
        self._clock = clock
        self._hit_miss_debounce_seconds = hit_miss_debounce_seconds
        self._last_ble_throw: Optional[Tuple[str, float]] = None

    async def process(self, event: Dict[str, Any], source: str = "ble") -> bool:
        async with self._lock:
            if (
                source in {"ble", "test"}
                and event.get("type") in {"hit", "miss"}
                and self.controller.correction_locked()
            ):
                return False
            if source == "ble" and "seq" in event:
                identity = (int(event["seq"]), str(event.get("raw", event.get("code", ""))))
                if identity in self._recent_set:
                    return False
                if len(self._recent) == self._recent.maxlen:
                    oldest = self._recent.popleft()
                    self._recent_set.discard(oldest)
                self._recent.append(identity)
                self._recent_set.add(identity)
            now = self._clock()
            if source == "ble" and event.get("type") == "miss":
                if self._last_ble_throw is not None:
                    last_type, last_time = self._last_ble_throw
                    if (
                        last_type == "hit"
                        and now - last_time <= self._hit_miss_debounce_seconds
                    ):
                        return False
            enriched = {**event, "_source": source}
            if source == "test" and self.controller.game_id:
                self.controller.store.set_game_environment(
                    self.controller.game_id,
                    "test",
                )
            throw_count = len(self.controller.engine.state.throws)
            self.controller.process_event(enriched)
            if (
                source == "ble"
                and event.get("type") in {"hit", "miss"}
                and len(self.controller.engine.state.throws) > throw_count
            ):
                self._last_ble_throw = (str(event["type"]), now)
            event.update(
                {
                    key: value
                    for key, value in enriched.items()
                    if not key.startswith("_")
                }
            )
            return True
