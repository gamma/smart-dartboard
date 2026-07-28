from __future__ import annotations

import importlib
import pkgutil
from typing import Dict, Iterable

from .base import GameMode


class GameRegistry:
    def __init__(self) -> None:
        self._modes: Dict[str, GameMode] = {}

    def discover(self) -> None:
        self._modes.clear()
        package = importlib.import_module(__package__)
        for module_info in pkgutil.iter_modules(package.__path__):
            if module_info.name in {"base", "registry"} or module_info.name.startswith("_"):
                continue
            module = importlib.import_module(f"{__package__}.{module_info.name}")
            mode = getattr(module, "GAME_MODE", None)
            if mode is not None:
                self.register(mode)

    def register(self, mode: GameMode) -> None:
        slug = mode.metadata.slug
        if not slug:
            raise ValueError("Game mode slug must not be empty")
        self._modes[slug] = mode

    def get(self, slug: str) -> GameMode:
        try:
            return self._modes[slug]
        except KeyError as exc:
            raise ValueError(f"Unknown game mode: {slug}") from exc

    def all(self) -> Iterable[GameMode]:
        return tuple(self._modes[key] for key in sorted(self._modes))

    def as_dicts(self):
        return [mode.metadata.as_dict() for mode in self.all()]


registry = GameRegistry()
registry.discover()
