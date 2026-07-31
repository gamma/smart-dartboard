from __future__ import annotations

from typing import Any, Dict

from .arcade import choose_targets, finish_round_game, overlay_item, same_target
from .base import GameMetadata, GameOption, InstructionStep, ThrowOutcome

BOARD_ORDER = [20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5]
NUMBER_RINGS = ["double", "single_outer", "triple", "single_inner"]


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
            GameOption("bomb_count", "Startbomben", "choice", 4, [{"value":2,"label":"2 Bomben"},{"value":4,"label":"4 Bomben"},{"value":6,"label":"6 Bomben"}]),
            GameOption("bomb_growth", "Bombenzuwachs", "choice", "escalating", [{"value":"steady","label":"+1 pro Runde"},{"value":"escalating","label":"+ Rundennummer"}]),
            GameOption("penalty", "Strafe", "choice", -50, [{"value":-25,"label":"-25"},{"value":-50,"label":"-50"},{"value":-100,"label":"-100"}]),
        ],
        instructions=[
            InstructionStep("Rot ist gefährlich", "Rote Felder sind Bomben und kosten Punkte.", "danger"),
            InstructionStep("Alles andere zählt", "Normale Treffer geben ihren Dartwert.", "score"),
            InstructionStep("Jede Runde schwerer", "Nachdem alle gespielt haben, wachsen die Bomben – gleichmäßig oder um die neue Rundennummer.", "growth"),
            InstructionStep(
                "Boom oder knapp",
                "Bombentreffer explodieren groß. Direkt angrenzende Felder zeigen ‚Das war knapp‘, punkten aber normal.",
                "boom",
            ),
        ],
        sound_theme="arcade",
        ruleset_version=2,
    )

    def initialize_player(self, player: Any, options: Dict[str, Any]) -> None:
        player.score = 0
        player.marks = {}

    def initialize_state(self, state: Any, options: Dict[str, Any]) -> None:
        count = int(options.get("bomb_count", 4))
        bombs = choose_targets(count, "normal")
        state.mode_state = {"bombs": bombs, "bomb_round": 1}
        state.message = "Meide Rot!"

    def on_turn_start(self, state: Any, player: Any) -> None:
        del player
        bomb_round = int(state.mode_state.get("bomb_round", 1))
        added_bomb = False
        added_count = 0
        while bomb_round < state.round_number:
            bombs = state.mode_state.setdefault("bombs", [])
            next_round = bomb_round + 1
            growth = (
                next_round
                if state.options.get("bomb_growth", "escalating") == "escalating"
                else 1
            )
            exclude = [
                str(bomb.get("label", ""))
                for bomb in bombs
                if bomb.get("label")
            ]
            additions = choose_targets(growth, "normal", exclude)
            bombs.extend(additions)
            added_count += len(additions)
            bomb_round = next_round
            added_bomb = True
        state.mode_state["bomb_round"] = bomb_round
        if added_bomb:
            state.message = (
                f"Runde {bomb_round}: Eine neue Bombe ist aktiv!"
                if added_count == 1
                else f"Runde {bomb_round}: {added_count} neue Bomben sind aktiv!"
            )

    def apply_throw(self, state: Any, player: Any, event: Dict[str, Any]) -> ThrowOutcome:
        bombs = state.mode_state.get("bombs", [])
        if event.get("type") == "miss":
            outcome = ThrowOutcome(turn_value=0, message="Miss")
        elif bomb := next((bomb for bomb in bombs if same_target(event, bomb)), None):
            penalty = int(state.options.get("penalty", -50))
            player.score += penalty
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
        penalty = int(state.options.get("penalty", -50))
        danger = []
        for bomb in bombs:
            item = overlay_item(bomb, "#e76f51", "", True)
            item.update({"icon": "mine", "variant": "mine"})
            danger.append(item)
        return {
            "prompt": f"Runde {state.round_number}: {len(bombs)} Bomben – meide Rot!",
            "danger": danger,
            "visual_legend": [{
                "icon": "mine",
                "color": "#e76f51",
                "label": "Bombe",
                "value": str(penalty),
            }],
        }


GAME_MODE = AvoidBombMode()
