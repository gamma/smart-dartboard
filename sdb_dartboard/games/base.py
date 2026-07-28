from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Protocol


@dataclass(frozen=True)
class GameOption:
    key: str
    label: str
    kind: str
    default: Any
    choices: List[Dict[str, Any]] = field(default_factory=list)


@dataclass(frozen=True)
class InstructionStep:
    title: str
    body: str
    icon: str


@dataclass(frozen=True)
class GameMetadata:
    slug: str
    title: str
    tagline: str
    description: str
    accent: str
    accent_secondary: str
    visual: str
    icon: str
    min_players: int = 1
    max_players: int = 8
    options: List[GameOption] = field(default_factory=list)
    instructions: List[InstructionStep] = field(default_factory=list)
    sound_theme: str = "arena"

    def as_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class ThrowOutcome:
    turn_value: int
    message: str
    finished: bool = False
    bust: bool = False
    force_hold: bool = False
    winner_id: str | None = None


class GameMode(Protocol):
    metadata: GameMetadata

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None: ...

    def apply_throw(
        self,
        state: Any,
        player: Any,
        event: Dict[str, Any],
    ) -> ThrowOutcome: ...
