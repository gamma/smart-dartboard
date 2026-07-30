from __future__ import annotations

import asyncio
import os
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
        self.screen = "attract"
        self.session_id: Optional[str] = None
        self.game_id: Optional[str] = None
        self.selected_mode: Optional[str] = None
        self.selected_options: Dict[str, Any] = {}
        self.selected_language = "de"
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
            stored_language = runtime.get("selected_language", "de")
            self.selected_language = stored_language if stored_language in {"de", "en"} else "de"
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
                "selected_language": self.selected_language,
            },
        )
        self.store.set_runtime_value("engine", self.engine.export_state())

    def public_state(self) -> Dict[str, Any]:
        session = self.store.get_session(self.session_id) if self.session_id else None
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
            "language": (
                session.get("language", self.selected_language)
                if session
                else self.selected_language
            ),
            "game": self.engine.state.as_dict(),
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
        }

    def create_player(self, name: str, avatar: str, color: str) -> Dict[str, Any]:
        return self.store.create_player(name, avatar, color)

    def show_player_selection(self) -> None:
        self.screen = "players"
        self._persist()

    def start_session(
        self,
        player_ids: Iterable[str],
        language: str = "de",
    ) -> Dict[str, Any]:
        active = self.store.active_session()
        if active:
            self.store.end_session(active["id"])
        session = self.store.start_session(player_ids, language)
        self.session_id = session["id"]
        self.game_id = None
        self.selected_mode = None
        self.selected_options = {}
        self.selected_language = language
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
        self.screen = "instructions"
        self._persist()

    def start_game(self) -> None:
        if not self.session_id or not self.selected_mode:
            raise ValueError("Session and game selection are required")
        session = self.store.get_session(self.session_id)
        if not session:
            raise ValueError("Active session no longer exists")
        self.engine.reset(
            self.selected_mode,
            session["players"],
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
            },
            source="system",
        )
        self._rematch_armed_until = 0.0
        self.screen = "countdown"
        self._persist()
        return True

    def continue_turn(self) -> None:
        self.engine.continue_turn()
        self._record_event("continue_turn")
        self._finish_game_if_needed()
        self._persist()

    def next_player(self) -> None:
        self.engine.next_player()
        self._record_event("next_player")
        self._finish_game_if_needed()
        self._persist()

    def undo(self) -> None:
        had_throw = bool(self.engine.state.throws)
        was_finished = self.engine.state.status == "finished"
        self.engine.undo()
        if had_throw and self.game_id:
            corrected_event_id = self.store.invalidate_last_throw_event(self.game_id)
            self.store.delete_last_throw(self.game_id)
            self._record_event(
                "undo",
                payload={"kind": "throw"},
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
        was_finished = self.engine.state.status == "finished"
        self.engine.correct_turn_throw(turn_index, replacement)
        if self.game_id:
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
                        "source": "correction",
                        "task": throw.context_before,
                    }
                    for throw in self.engine.state.throws
                ],
            )
            self._record_event(
                "throw_corrected",
                payload={
                    "turn_index": turn_index,
                    "replacement": replacement,
                },
            )
            if self.engine.state.status == "finished":
                self._finish_game_if_needed(force=True)
            elif was_finished:
                self.store.reopen_game(self.game_id)
                self.screen = "playing"
        self._persist()

    def game_action(self, action: str, payload: Dict[str, Any] | None = None) -> None:
        self.engine.handle_action(action, payload or {})
        self._record_event("game_action", payload={"action": action, **(payload or {})})
        self._finish_game_if_needed()
        self._persist()

    def next_game(self) -> None:
        self._rematch_armed_until = 0.0
        self.engine.clear()
        self.game_id = None
        self.selected_mode = None
        self.selected_options = {}
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


class EventPipeline:
    """Serializes events and drops repeated BLE notifications."""

    def __init__(self, controller: SessionController, history_size: int = 256) -> None:
        self.controller = controller
        self._lock = asyncio.Lock()
        self._recent: Deque[Tuple[int, str]] = deque(maxlen=history_size)
        self._recent_set: set[Tuple[int, str]] = set()

    async def process(self, event: Dict[str, Any], source: str = "ble") -> bool:
        async with self._lock:
            if source == "ble" and "seq" in event:
                identity = (int(event["seq"]), str(event.get("raw", event.get("code", ""))))
                if identity in self._recent_set:
                    return False
                if len(self._recent) == self._recent.maxlen:
                    oldest = self._recent.popleft()
                    self._recent_set.discard(oldest)
                self._recent.append(identity)
                self._recent_set.add(identity)
            enriched = {**event, "_source": source}
            if source == "test" and self.controller.game_id:
                self.controller.store.set_game_environment(
                    self.controller.game_id,
                    "test",
                )
            self.controller.process_event(enriched)
            return True
