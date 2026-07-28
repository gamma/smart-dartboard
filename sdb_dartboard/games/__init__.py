"""Discoverable game modes.

Adding a module that exposes ``GAME_MODE`` is enough to register a new mode.
The runtime and the web UI consume the same metadata from the registry.
"""

from .registry import GameRegistry, registry

__all__ = ["GameRegistry", "registry"]
