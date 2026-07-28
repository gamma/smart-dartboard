from __future__ import annotations

import asyncio
from collections import deque
from pathlib import Path
from typing import Any, Deque, Dict, Iterable, Optional, Tuple

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


class SessionController:
    """Owns the durable session and screen state for every connected display."""

    def __init__(self, database_path: Path | str, engine: Optional[GameEngine] = None) -> None:
        self.store = DartboardStore(database_path)
        self.engine = engine or GameEngine()
        self.screen = "attract"
        self.session_id: Optional[str] = None
        self.game_id: Optional[str] = None
        self.selected_mode: Optional[str] = None
        self.selected_options: Dict[str, Any] = {}
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
        if checkpoint:
            self.engine.import_state(checkpoint)
        self.calibration = self.store.get_runtime_value("calibration", self.calibration)

    def _persist(self) -> None:
        self.store.set_runtime_value(
            "experience",
            {
                "screen": self.screen,
                "session_id": self.session_id,
                "game_id": self.game_id,
                "selected_mode": self.selected_mode,
                "selected_options": self.selected_options,
            },
        )
        self.store.set_runtime_value("engine", self.engine.export_state())

    def public_state(self) -> Dict[str, Any]:
        session = self.store.get_session(self.session_id) if self.session_id else None
        player_ids = [player["id"] for player in session["players"]] if session else []
        return {
            "screen": self.screen,
            "session": session,
            "game_id": self.game_id,
            "selected_mode": self.selected_mode,
            "selected_options": self.selected_options,
            "game": self.engine.state.as_dict(),
            "modes": registry.as_dicts(),
            "players": self.store.list_players(),
            "statistics": self.store.statistics(player_ids or None),
            "calibration": self.calibration,
            "hardware": self.hardware,
        }

    def create_player(self, name: str, avatar: str, color: str) -> Dict[str, Any]:
        return self.store.create_player(name, avatar, color)

    def show_player_selection(self) -> None:
        self.screen = "players"
        self._persist()

    def start_session(self, player_ids: Iterable[str]) -> Dict[str, Any]:
        active = self.store.active_session()
        if active:
            self.store.end_session(active["id"])
        session = self.store.start_session(player_ids)
        self.session_id = session["id"]
        self.game_id = None
        self.selected_mode = None
        self.selected_options = {}
        self.screen = "game_select"
        self._persist()
        return session

    def prepare_game(self, game_type: str, options: Dict[str, Any]) -> None:
        if not self.session_id:
            raise ValueError("Start a session before selecting a game")
        mode = registry.get(game_type)
        defaults = {option.key: option.default for option in mode.metadata.options}
        defaults.update(options)
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
        )
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
        before_count = len(self.engine.state.throws)
        self.engine.handle_event(event)
        if len(self.engine.state.throws) > before_count and self.game_id:
            throw = self.engine.state.throws[-1]
            player = next(
                (item for item in self.engine.state.players if item.id == throw.player_id),
                None,
            )
            self.store.record_throw(
                self.game_id,
                throw.seq,
                throw.player_id,
                throw.raw,
                player.score if player else 0,
            )
        if self.engine.state.status == "finished" and self.game_id:
            game = self.store.get_game(self.game_id)
            if game and game["status"] != "finished":
                self.store.finish_game(self.game_id, self.engine.state.winner_id)
            self.screen = "game_result"
        self._persist()

    def continue_turn(self) -> None:
        self.engine.continue_turn()
        self._persist()

    def next_player(self) -> None:
        self.engine.next_player()
        self._persist()

    def undo(self) -> None:
        had_throw = bool(self.engine.state.throws)
        was_finished = self.engine.state.status == "finished"
        self.engine.undo()
        if had_throw and self.game_id:
            self.store.delete_last_throw(self.game_id)
        if was_finished and self.game_id and self.engine.state.status != "finished":
            self.store.reopen_game(self.game_id)
            self.screen = "playing"
        self._persist()

    def next_game(self) -> None:
        self.game_id = None
        self.selected_mode = None
        self.selected_options = {}
        self.screen = "game_select"
        self._persist()

    def end_session(self) -> None:
        if self.session_id:
            self.store.end_session(self.session_id)
        self.screen = "session_summary"
        self._persist()

    def reset_to_attract(self) -> None:
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
            self.controller.process_event(event)
            return True
