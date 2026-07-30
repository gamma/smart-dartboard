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
    control_legend: List[Dict[str, Any]] = field(default_factory=list)
    sound_theme: str = "arena"
    ruleset_version: int = 1

    def as_dict(self) -> Dict[str, Any]:
        return asdict(self)

    def resolve_options(self, provided: Dict[str, Any] | None = None) -> Dict[str, Any]:
        """Merge defaults and reject unknown, malformed or unsafe values."""
        supplied = dict(provided or {})
        known = {option.key: option for option in self.options}
        unknown = sorted(set(supplied) - set(known))
        if unknown:
            raise ValueError(f"Unknown options for {self.slug}: {', '.join(unknown)}")

        resolved = {option.key: option.default for option in self.options}
        for key, value in supplied.items():
            option = known[key]
            expected = type(option.default)
            if expected is int and (isinstance(value, bool) or not isinstance(value, int)):
                raise ValueError(f"Invalid value for {key}: expected an integer")
            if expected is str and not isinstance(value, str):
                raise ValueError(f"Invalid value for {key}: expected text")
            if isinstance(value, int) and not -100_000 <= value <= 100_000:
                raise ValueError(f"Invalid value for {key}: number is outside the safe range")
            if isinstance(value, str) and len(value) > 64:
                raise ValueError(f"Invalid value for {key}: text is too long")
            allowed = [choice["value"] for choice in option.choices]
            if allowed and value not in allowed:
                raise ValueError(
                    f"Invalid value for {key}: choose one of "
                    + ", ".join(str(item) for item in allowed)
                )
            resolved[key] = value
        return resolved


@dataclass
class ThrowOutcome:
    turn_value: int
    message: str
    finished: bool = False
    bust: bool = False
    force_hold: bool = False
    winner_id: str | None = None
    winner_ids: List[str] = field(default_factory=list)
    result_type: str = ""


class GameMode(Protocol):
    metadata: GameMetadata

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None: ...

    def apply_throw(
        self,
        state: Any,
        player: Any,
        event: Dict[str, Any],
    ) -> ThrowOutcome: ...
