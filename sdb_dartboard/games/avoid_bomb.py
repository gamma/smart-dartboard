from __future__ import annotations

import random
from typing import Any, Dict

from .arcade import finish_round_game, overlay_item, same_target
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome
from .x01_advisor import DARTS

BOARD_ORDER = [20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5]
NUMBER_RINGS = ["double", "single_outer", "triple", "single_inner"]


def _bomb_id(target: Dict[str, Any]) -> str:
    return f"{target.get('ring', '')}:{int(target.get('field', 0) or 0)}"


def _bomb_pool() -> list[Dict[str, Any]]:
    """Return every physical scoring segment that may carry a bomb.

    The generic dart list represents a Single as ``single_outer`` because both
    Single rings score identically. Bombs are graphical board zones, however,
    so the inner and outer Single must be separate candidates.
    """
    pool = [dict(dart) for dart in DARTS if dart["label"] != "SBull"]
    pool.extend(
        {
            **dart,
            "ring": "single_inner",
        }
        for dart in DARTS
        if dart["ring"] == "single_outer"
    )
    return pool


BOMB_POOL = _bomb_pool()


def _choose_bombs(
    count: int,
    exclude: set[str] | None = None,
    *,
    rng: random.Random | None = None,
) -> list[Dict[str, Any]]:
    excluded = exclude or set()
    available = [target for target in BOMB_POOL if _bomb_id(target) not in excluded]
    picker = rng or random
    # Gameplay variety only; no security decision depends on this randomness.
    return picker.sample(available, min(count, len(available)))  # nosec B311


class AvoidBombMode:
    metadata = GameMetadata(
        slug="avoid_bomb",
        title="Avoid the Bomb",
        tagline="Sammle Punkte – meide Rot",
        description="Normale Treffer zählen, aber rote Bomben ziehen Punkte ab und sorgen für Party-Chaos.",
        accent="#ff4f79",
        accent_secondary="#ffb52b",
        visual="avoid-bomb",
        icon="bomb",
        options=[
            GameOption("rounds", "Runden", "choice", 5, [{"value":3,"label":"3 Runden"},{"value":5,"label":"5 Runden"},{"value":8,"label":"8 Runden"}]),
            GameOption("bomb_count", "Startbomben", "choice", 6, [{"value":4,"label":"4 Bomben"},{"value":6,"label":"6 Bomben"},{"value":8,"label":"8 Bomben"}]),
            GameOption("bomb_growth", "Bombenzuwachs", "choice", "escalating", [{"value":"steady","label":"+1 pro Runde"},{"value":"escalating","label":"+ Rundennummer"}]),
            GameOption("hidden_bombs", "Bombensicht", "choice", "memory", [{"value":"visible","label":"Immer sichtbar"},{"value":"memory","label":"Memory · zeitweise versteckt"}]),
            GameOption("penalty", "Strafe", "choice", -50, [{"value":-25,"label":"-25"},{"value":-50,"label":"-50"},{"value":-100,"label":"-100"}]),
        ],
        instructions=[
            InstructionStep("Rot ist gefährlich", "Rote Felder sind Bomben und kosten Punkte.", "danger"),
            InstructionStep("Alles andere zählt", "Normale Treffer geben ihren Dartwert.", "score"),
            InstructionStep("Jede Runde schwerer", "Nachdem alle gespielt haben, wachsen die Bomben – gleichmäßig oder um die neue Rundennummer.", "growth"),
            InstructionStep("Memory-Bomben", "Im Memory-Modus taucht nach einer sichtbaren Runde die Hälfte der Bomben für zwei Runden ab und erscheint danach wieder.", "memory"),
            InstructionStep(
                "Boom oder knapp",
                "Bombentreffer explodieren groß. Direkt angrenzende Felder zeigen ‚Das war knapp‘, punkten aber normal.",
                "boom",
            ),
        ],
        sound_theme="arcade",
        ruleset_version=3,
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        count = int(options.get("bomb_count", 6))
        seed = random.randrange(2**63)  # nosec B311 - reproducible gameplay seed
        bombs = _choose_bombs(count, rng=random.Random(f"{seed}:initial"))
        state.mode_state = {
            "bombs": bombs,
            "bomb_round": 1,
            "seed": seed,
            "hidden_bomb_ids": [],
            "hidden_until_round": 0,
            "next_hide_round": 2,
            "visibility_round": 1,
        }
        state.message = "Meide Rot!"

    def on_turn_start(self, state: Any, player: Any) -> None:
        del player
        bomb_round = int(state.mode_state.get("bomb_round", 1))
        added_count = 0
        while bomb_round < state.round_number:
            bombs = state.mode_state.setdefault("bombs", [])
            next_round = bomb_round + 1
            growth = (
                next_round
                if state.options.get("bomb_growth", "escalating") == "escalating"
                else 1
            )
            excluded = {_bomb_id(bomb) for bomb in bombs}
            seed = int(state.mode_state.get("seed", 0))
            additions = _choose_bombs(
                growth,
                excluded,
                rng=random.Random(f"{seed}:growth:{next_round}"),
            )
            bombs.extend(additions)
            added_count += len(additions)
            bomb_round = next_round
        state.mode_state["bomb_round"] = bomb_round

        visibility_messages = self._advance_visibility(state)
        messages = []
        if added_count:
            messages.append(
                "Eine neue Bombe ist aktiv!"
                if added_count == 1
                else f"{added_count} neue Bomben sind aktiv!"
            )
        messages.extend(visibility_messages)
        if messages:
            state.message = f"Runde {state.round_number}: {' '.join(messages)}"

    @staticmethod
    def _advance_visibility(state: Any) -> list[str]:
        mode_state = state.mode_state
        current = int(mode_state.get("visibility_round", 1))
        messages: list[str] = []
        while current < state.round_number:
            current += 1
            hidden_ids = list(mode_state.get("hidden_bomb_ids", []))
            hidden_until = int(mode_state.get("hidden_until_round", 0))
            if hidden_ids and current >= hidden_until:
                mode_state["hidden_bomb_ids"] = []
                messages.append("Die versteckten Bomben sind wieder sichtbar!")

            memory_enabled = state.options.get("hidden_bombs", "memory") == "memory"
            next_hide = int(mode_state.get("next_hide_round", 2))
            if (
                memory_enabled
                and not mode_state.get("hidden_bomb_ids")
                and current >= next_hide
            ):
                bombs = mode_state.get("bombs", [])
                hide_count = max(1, len(bombs) // 2) if bombs else 0
                seed = int(mode_state.get("seed", 0))
                rng = random.Random(f"{seed}:hide:{current}")
                hidden = rng.sample(bombs, hide_count) if hide_count else []
                mode_state["hidden_bomb_ids"] = [_bomb_id(bomb) for bomb in hidden]
                mode_state["hidden_until_round"] = current + 2
                mode_state["next_hide_round"] = current + 3
                messages.append(
                    f"{len(hidden)} Bomben sind für zwei Runden versteckt!"
                )
        mode_state["visibility_round"] = max(current, state.round_number)
        return messages

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        bombs = state.mode_state.get("bombs", [])
        if event.get("type") == "miss":
            outcome = ThrowOutcome(turn_value=0, message="Miss")
        elif bomb := next((bomb for bomb in bombs if same_target(event, bomb)), None):
            penalty = int(state.options.get("penalty", -50))
            player.score += penalty
            hidden_ids = state.mode_state.setdefault("hidden_bomb_ids", [])
            bomb_id = _bomb_id(bomb)
            if bomb_id in hidden_ids:
                hidden_ids.remove(bomb_id)
            event.update({"effect": "bomb_explosion", "bomb": dict(bomb)})
            outcome = ThrowOutcome(turn_value=penalty, message=f"BOMB! {penalty}")
        else:
            score = int(event.get("score", 0))
            player.score += score
            near_bomb = next(
                (bomb for bomb in bombs if self._is_adjacent(event, bomb)),
                None,
            )
            if near_bomb:
                event.update({
                    "effect": "bomb_near_miss",
                    "near_bomb": dict(near_bomb),
                })
                message = f"DAS WAR KNAPP! {event.get('label', '')} +{score}"
            else:
                message = f"Safe {event.get('label', '')} +{score}"
            outcome = ThrowOutcome(turn_value=score, message=message)
        return finish_round_game(
            state, outcome, "{winner} überlebt Avoid the Bomb!"
        )

    @staticmethod
    def _is_adjacent(event: Dict[str, Any], bomb: Dict[str, Any]) -> bool:
        event_field = int(event.get("field", 0) or 0)
        bomb_field = int(bomb.get("field", 0) or 0)
        event_ring = str(event.get("ring", ""))
        bomb_ring = str(bomb.get("ring", ""))

        if event_field == bomb_field == 25:
            return {event_ring, bomb_ring} == {"single_bull", "double_bull"}
        if event_field == 25 and event_ring == "single_bull":
            return bomb_ring == "single_inner" and bomb_field in BOARD_ORDER
        if bomb_field == 25 and bomb_ring == "single_bull":
            return event_ring == "single_inner" and event_field in BOARD_ORDER
        if event_field not in BOARD_ORDER or bomb_field not in BOARD_ORDER:
            return False
        if event_ring == bomb_ring:
            index = BOARD_ORDER.index(event_field)
            return bomb_field in {
                BOARD_ORDER[(index - 1) % len(BOARD_ORDER)],
                BOARD_ORDER[(index + 1) % len(BOARD_ORDER)],
            }
        if event_field != bomb_field:
            return False
        if event_ring not in NUMBER_RINGS or bomb_ring not in NUMBER_RINGS:
            return False
        return abs(NUMBER_RINGS.index(event_ring) - NUMBER_RINGS.index(bomb_ring)) == 1

    def get_overlay(self, state: Any) -> Dict[str, Any]:
        bombs = state.mode_state.get("bombs", [])
        hidden_ids = set(state.mode_state.get("hidden_bomb_ids", []))
        visible_bombs = [bomb for bomb in bombs if _bomb_id(bomb) not in hidden_ids]
        penalty = int(state.options.get("penalty", -50))
        danger = []
        for bomb in visible_bombs:
            item = overlay_item(bomb, "#e76f51", "", True)
            item.update({"id": _bomb_id(bomb), "icon": "mine", "variant": "mine"})
            danger.append(item)
        legend = [{
            "icon": "mine",
            "color": "#e76f51",
            "label": "Bombe",
            "value": str(penalty),
        }]
        if hidden_ids:
            legend.append({
                "icon": "mine",
                "color": "#72506f",
                "label": "Versteckt",
                "value": str(len(hidden_ids)),
            })
        return {
            "prompt": (
                f"Runde {state.round_number}: {len(visible_bombs)} sichtbar · "
                f"{len(hidden_ids)} versteckt – meide alle Bomben!"
            ),
            "danger": danger,
            "visual_legend": legend,
        }


GAME_MODE = AvoidBombMode()
