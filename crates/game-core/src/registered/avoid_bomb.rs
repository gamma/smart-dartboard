use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{Target, parse_target, ring_name, same_target, target_value},
    finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const BOARD_ORDER: [u8; 20] = [
    20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5,
];
const NUMBER_RINGS: [Ring; 4] = [
    Ring::Double,
    Ring::SingleOuter,
    Ring::Triple,
    Ring::SingleInner,
];

static ROUND_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(3, "3 Runden"),
    choice_integer(5, "5 Runden"),
    choice_integer(8, "8 Runden"),
];
static BOMB_COUNT_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(4, "4 Bomben"),
    choice_integer(6, "6 Bomben"),
    choice_integer(8, "8 Bomben"),
];
static GROWTH_CHOICES: [GameOptionChoice; 2] = [
    GameOptionChoice {
        value: GameOptionValue::Text("steady"),
        label: "+1 pro Runde",
        description: Some("Nach jeder vollständigen Runde kommt genau eine Bombe dazu."),
        description_en: Some("Exactly one bomb is added after every full round."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("escalating"),
        label: "+ Rundennummer",
        description: Some("Vor Runde 2 kommen zwei, vor Runde 3 drei neue Bomben dazu usw."),
        description_en: Some(
            "Two bombs are added before round 2, three before round 3, and so on.",
        ),
    },
];
static VISIBILITY_CHOICES: [GameOptionChoice; 2] = [
    GameOptionChoice {
        value: GameOptionValue::Text("visible"),
        label: "Immer sichtbar",
        description: Some("Alle Bomben bleiben auf der Scheibe sichtbar."),
        description_en: Some("Every bomb remains visible on the board."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("memory"),
        label: "Memory · zeitweise versteckt",
        description: Some(
            "Nach einer sichtbaren Runde wird die Hälfte für zwei Runden ausgeblendet.",
        ),
        description_en: Some("After one visible round, half the bombs are hidden for two rounds."),
    },
];
static PENALTY_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(-25, "-25"),
    choice_integer(-50, "-50"),
    choice_integer(-100, "-100"),
];
static OPTIONS: [GameOption; 5] = [
    GameOption {
        key: "rounds",
        label: "Runden",
        kind: "choice",
        default: GameOptionValue::Integer(5),
        choices: &ROUND_CHOICES,
    },
    GameOption {
        key: "bomb_count",
        label: "Startbomben",
        kind: "choice",
        default: GameOptionValue::Integer(6),
        choices: &BOMB_COUNT_CHOICES,
    },
    GameOption {
        key: "bomb_growth",
        label: "Bombenzuwachs",
        kind: "choice",
        default: GameOptionValue::Text("escalating"),
        choices: &GROWTH_CHOICES,
    },
    GameOption {
        key: "hidden_bombs",
        label: "Bombensicht",
        kind: "choice",
        default: GameOptionValue::Text("memory"),
        choices: &VISIBILITY_CHOICES,
    },
    GameOption {
        key: "penalty",
        label: "Strafe",
        kind: "choice",
        default: GameOptionValue::Integer(-50),
        choices: &PENALTY_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 5] = [
    GameInstruction {
        title: "Rot ist gefährlich",
        body: "Rote Felder sind Bomben und kosten Punkte.",
        icon: "danger",
    },
    GameInstruction {
        title: "Alles andere zählt",
        body: "Normale Treffer geben ihren Dartwert.",
        icon: "score",
    },
    GameInstruction {
        title: "Jede Runde schwerer",
        body: "Nachdem alle gespielt haben, wachsen die Bomben – gleichmäßig oder um die neue Rundennummer.",
        icon: "growth",
    },
    GameInstruction {
        title: "Memory-Bomben",
        body: "Im Memory-Modus taucht nach einer sichtbaren Runde die Hälfte der Bomben für zwei Runden ab und erscheint danach wieder.",
        icon: "memory",
    },
    GameInstruction {
        title: "Boom oder knapp",
        body: "Bombentreffer explodieren groß. Direkt angrenzende Felder zeigen ‚Das war knapp‘, punkten aber normal.",
        icon: "boom",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "avoid_bomb",
    ruleset_version: 4,
    format: sdb_contracts::GameFormat::Individual,
    title: "Avoid the Bomb",
    tagline: "Sammle Punkte – meide Rot",
    description: "Normale Treffer zählen, aber rote Bomben ziehen Punkte ab und sorgen für Party-Chaos.",
    accent: "#ff4f79",
    accent_secondary: "#ffb52b",
    visual: "avoid-bomb",
    icon: "bomb",
    artwork: "/static/assets/modes/avoid_bomb.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct AvoidBombMode;
pub(super) static AVOID_BOMB_MODE: AvoidBombMode = AvoidBombMode;

impl GameMode for AvoidBombMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let bombs = choose_bombs(state, bomb_count(state)?, &BTreeSet::new())?;
        state.mode_state = json!({
            "bombs": bombs.iter().map(target_value).collect::<Vec<_>>(),
            "bomb_round": 1,
            "hidden_bomb_ids": [],
            "hidden_until_round": 0,
            "next_hide_round": 2,
            "visibility_round": 1,
            "last_effect": "",
            "effect_target": null,
        });
        state.message = "Meide Rot!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        state.mode_state["last_effect"] = Value::from("");
        state.mode_state["effect_target"] = Value::Null;
        let bombs = bombs(state)?;
        let exact = bombs.iter().find(|bomb| same_target(event, bomb)).cloned();
        let points = if matches!(event, DartEvent::Miss { .. }) {
            state.message = "Miss".into();
            0
        } else if let Some(bomb) = exact {
            let penalty = penalty(state)?;
            let player = state
                .players
                .get_mut(state.current_player_index)
                .ok_or(GameError::NoPlayers)?;
            player.score = player.score.saturating_add(penalty);
            reveal_hidden_bomb(state, &bomb)?;
            state.mode_state["last_effect"] = Value::from("bomb_explosion");
            state.mode_state["effect_target"] = target_value(&bomb);
            state.message = format!("BOMB! {penalty}");
            penalty
        } else {
            let score = i64::from(event.score());
            let player = state
                .players
                .get_mut(state.current_player_index)
                .ok_or(GameError::NoPlayers)?;
            player.score = player.score.saturating_add(score);
            if let Some(near) = bombs.iter().find(|bomb| is_adjacent(event, bomb)) {
                state.mode_state["last_effect"] = Value::from("bomb_near_miss");
                state.mode_state["effect_target"] = target_value(near);
                state.message = format!("DAS WAR KNAPP! {} +{score}", event.label());
            } else {
                state.message = format!("Safe {} +{score}", event.label());
            }
            score
        };
        finish_fixed_round_game(state, "{winner} überlebt Avoid the Bomb!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let hidden = hidden_ids(state).unwrap_or_default();
        let visible = bombs(state)
            .unwrap_or_default()
            .into_iter()
            .filter(|bomb| !hidden.contains(&bomb_id(bomb)))
            .map(|bomb| {
                json!({
                    "id": bomb_id(&bomb),
                    "field": bomb.field,
                    "ring": bomb.ring,
                    "color": "#e76f51",
                    "label": "",
                    "pulse": true,
                    "icon": "mine",
                    "variant": "mine",
                })
            })
            .collect::<Vec<_>>();
        let mut legend = vec![json!({
            "icon": "mine",
            "color": "#e76f51",
            "label": "Bombe",
            "value": penalty(state).unwrap_or(-50).to_string(),
        })];
        if !hidden.is_empty() {
            legend.push(json!({
                "icon": "mine",
                "color": "#72506f",
                "label": "Versteckt",
                "value": hidden.len().to_string(),
            }));
        }
        json!({
            "prompt": format!(
                "Runde {}: {} sichtbar · {} versteckt – meide alle Bomben!",
                state.round_number,
                visible.len(),
                hidden.len(),
            ),
            "danger": visible,
            "visual_legend": legend,
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let added = grow_bombs(state)?;
        let visibility_messages = advance_visibility(state)?;
        let mut messages = Vec::new();
        if added == 1 {
            messages.push("Eine neue Bombe ist aktiv!".into());
        } else if added > 1 {
            messages.push(format!("{added} neue Bomben sind aktiv!"));
        }
        messages.extend(visibility_messages);
        if !messages.is_empty() {
            state.message = format!("Runde {}: {}", state.round_number, messages.join(" "));
        }
        Ok(())
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} überlebt Avoid the Bomb!")
    }
}

fn bomb_pool() -> Vec<Target> {
    let mut pool = Vec::with_capacity(81);
    for ring in [Ring::SingleOuter, Ring::Double, Ring::Triple] {
        pool.extend((1..=20).map(|field| target(field, ring)));
    }
    pool.push(target(25, Ring::DoubleBull));
    pool.extend((1..=20).map(|field| target(field, Ring::SingleInner)));
    pool
}

fn target(field: u8, ring: Ring) -> Target {
    let (prefix, multiplier) = match ring {
        Ring::Double => ("D", 2),
        Ring::Triple => ("T", 3),
        Ring::DoubleBull => ("DBull", 2),
        _ => ("S", 1),
    };
    Target {
        label: if field == 25 {
            prefix.into()
        } else {
            format!("{prefix}{field}")
        },
        field,
        ring,
        multiplier,
        score: u16::from(field) * u16::from(multiplier),
    }
}

fn choose_bombs(
    state: &mut RegisteredGameState,
    count: usize,
    excluded: &BTreeSet<String>,
) -> Result<Vec<Target>, GameError> {
    let mut available = bomb_pool()
        .into_iter()
        .filter(|target| !excluded.contains(&bomb_id(target)))
        .collect::<Vec<_>>();
    choose_existing(state, &mut available, count)
}

fn choose_existing(
    state: &mut RegisteredGameState,
    available: &mut Vec<Target>,
    count: usize,
) -> Result<Vec<Target>, GameError> {
    let mut selected = Vec::with_capacity(count.min(available.len()));
    for _ in 0..count.min(available.len()) {
        let index = state.random_index(available.len())?;
        selected.push(available.remove(index));
    }
    Ok(selected)
}

fn grow_bombs(state: &mut RegisteredGameState) -> Result<usize, GameError> {
    let mut bomb_round = state.mode_state["bomb_round"]
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(invalid_state)?;
    let mut added = 0;
    while bomb_round < state.round_number {
        let next_round = bomb_round.saturating_add(1);
        let growth = if state.options["bomb_growth"] == "escalating" {
            usize::from(next_round)
        } else {
            1
        };
        let excluded = bombs(state)?.iter().map(bomb_id).collect::<BTreeSet<_>>();
        let additions = choose_bombs(state, growth, &excluded)?;
        let list = state.mode_state["bombs"]
            .as_array_mut()
            .ok_or_else(invalid_state)?;
        list.extend(additions.iter().map(target_value));
        added += additions.len();
        bomb_round = next_round;
    }
    state.mode_state["bomb_round"] = Value::from(bomb_round);
    Ok(added)
}

fn advance_visibility(state: &mut RegisteredGameState) -> Result<Vec<String>, GameError> {
    let mut current = state.mode_state["visibility_round"]
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(invalid_state)?;
    let mut messages = Vec::new();
    while current < state.round_number {
        current = current.saturating_add(1);
        let hidden = hidden_ids(state)?;
        let hidden_until = state.mode_state["hidden_until_round"]
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(invalid_state)?;
        if !hidden.is_empty() && current >= hidden_until {
            state.mode_state["hidden_bomb_ids"] = json!([]);
            messages.push("Die versteckten Bomben sind wieder sichtbar!".into());
        }

        let next_hide = state.mode_state["next_hide_round"]
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(invalid_state)?;
        if state.options["hidden_bombs"] == "memory"
            && hidden_ids(state)?.is_empty()
            && current >= next_hide
        {
            let mut available = bombs(state)?;
            let hide_count = if available.is_empty() {
                0
            } else {
                1.max(available.len() / 2)
            };
            let selected = choose_existing(state, &mut available, hide_count)?;
            let ids = selected.iter().map(bomb_id).collect::<Vec<_>>();
            state.mode_state["hidden_bomb_ids"] = json!(ids);
            state.mode_state["hidden_until_round"] = Value::from(current.saturating_add(2));
            state.mode_state["next_hide_round"] = Value::from(current.saturating_add(3));
            messages.push(format!(
                "{} Bomben sind für zwei Runden versteckt!",
                selected.len()
            ));
        }
    }
    state.mode_state["visibility_round"] = Value::from(current.max(state.round_number));
    Ok(messages)
}

fn bombs(state: &RegisteredGameState) -> Result<Vec<Target>, GameError> {
    state.mode_state["bombs"]
        .as_array()
        .ok_or_else(invalid_state)?
        .iter()
        .map(parse_target)
        .collect()
}

fn hidden_ids(state: &RegisteredGameState) -> Result<BTreeSet<String>, GameError> {
    state.mode_state["hidden_bomb_ids"]
        .as_array()
        .ok_or_else(invalid_state)?
        .iter()
        .map(|value| value.as_str().map(str::to_owned).ok_or_else(invalid_state))
        .collect()
}

fn reveal_hidden_bomb(state: &mut RegisteredGameState, bomb: &Target) -> Result<(), GameError> {
    let id = bomb_id(bomb);
    state.mode_state["hidden_bomb_ids"]
        .as_array_mut()
        .ok_or_else(invalid_state)?
        .retain(|value| value.as_str() != Some(&id));
    Ok(())
}

fn bomb_id(target: &Target) -> String {
    format!("{}:{}", ring_name(target.ring), target.field)
}

fn bomb_count(state: &RegisteredGameState) -> Result<usize, GameError> {
    state.options["bomb_count"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn penalty(state: &RegisteredGameState) -> Result<i64, GameError> {
    state.options["penalty"].as_i64().ok_or_else(invalid_state)
}

fn is_adjacent(event: &DartEvent, bomb: &Target) -> bool {
    let DartEvent::Hit { field, ring, .. } = event else {
        return false;
    };
    if *field == 25 && bomb.field == 25 {
        return matches!(
            (*ring, bomb.ring),
            (Ring::SingleBull, Ring::DoubleBull) | (Ring::DoubleBull, Ring::SingleBull)
        );
    }
    if *field == 25 && *ring == Ring::SingleBull {
        return bomb.ring == Ring::SingleInner && BOARD_ORDER.contains(&bomb.field);
    }
    if bomb.field == 25 && bomb.ring == Ring::SingleBull {
        return *ring == Ring::SingleInner && BOARD_ORDER.contains(field);
    }
    let Some(event_index) = BOARD_ORDER.iter().position(|candidate| candidate == field) else {
        return false;
    };
    if !BOARD_ORDER.contains(&bomb.field) {
        return false;
    }
    if *ring == bomb.ring {
        return bomb.field
            == BOARD_ORDER[(event_index + BOARD_ORDER.len() - 1) % BOARD_ORDER.len()]
            || bomb.field == BOARD_ORDER[(event_index + 1) % BOARD_ORDER.len()];
    }
    if *field != bomb.field {
        return false;
    }
    let Some(event_ring) = NUMBER_RINGS.iter().position(|candidate| candidate == ring) else {
        return false;
    };
    let Some(bomb_ring) = NUMBER_RINGS
        .iter()
        .position(|candidate| *candidate == bomb.ring)
    else {
        return false;
    };
    event_ring.abs_diff(bomb_ring) == 1
}

const fn choice_integer(value: i64, label: &'static str) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Integer(value),
        label,
        description: None,
        description_en: None,
    }
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid avoid_bomb mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameStatus, RegisteredGame};

    fn game(options: &Value) -> RegisteredGame {
        RegisteredGame::new_seeded(
            "avoid_bomb",
            vec![("ada".into(), "Ada".into())],
            options,
            42,
        )
        .expect("game")
    }

    fn hit(target: &Target, seq: u64) -> DartEvent {
        DartEvent::Hit {
            seq,
            field: target.field,
            ring: target.ring,
            multiplier: target.multiplier,
            label: target.label.clone(),
            score: target.score,
        }
    }

    #[test]
    fn pool_contains_both_single_rings_and_only_double_bull() {
        assert_eq!(METADATA.ruleset_version, 4);
        assert_eq!(METADATA.options.len(), 5);
        assert_eq!(METADATA.options[1].default, GameOptionValue::Integer(6));
        assert_eq!(METADATA.options[4].default, GameOptionValue::Integer(-50));
        let pool = bomb_pool();
        assert_eq!(pool.len(), 81);
        assert_eq!(
            pool.iter()
                .filter(|target| target.ring == Ring::SingleOuter)
                .count(),
            20
        );
        assert_eq!(
            pool.iter()
                .filter(|target| target.ring == Ring::SingleInner)
                .count(),
            20
        );
        assert!(pool.iter().any(|target| target.ring == Ring::DoubleBull));
        assert!(!pool.iter().any(|target| target.ring == Ring::SingleBull));
    }

    #[test]
    fn bomb_hit_is_signed_reveals_hidden_target_and_undoes() {
        let mut game = game(&json!({
            "rounds": 3,
            "bomb_count": 4,
            "bomb_growth": "steady",
            "hidden_bombs": "memory",
            "penalty": -50,
        }));
        for seq in 1..=3 {
            game.apply_throw(&DartEvent::Miss {
                seq,
                label: "MISS".into(),
                score: 0,
            })
            .expect("miss");
        }
        game.continue_turn().expect("memory round");
        let hidden = hidden_ids(game.state())
            .expect("hidden")
            .into_iter()
            .next()
            .expect("hidden bomb");
        let bomb = bombs(game.state())
            .expect("bombs")
            .into_iter()
            .find(|bomb| bomb_id(bomb) == hidden)
            .expect("bomb");

        game.apply_throw(&hit(&bomb, 4)).expect("bomb");

        assert_eq!(game.state.players[0].score, -50);
        assert_eq!(game.state.turn_score, -50);
        assert_eq!(game.state.mode_state["last_effect"], "bomb_explosion");
        assert!(
            !hidden_ids(game.state())
                .expect("hidden")
                .contains(&bomb_id(&bomb))
        );
        game.undo().expect("undo");
        assert_eq!(game.state.players[0].score, 0);
        assert!(
            hidden_ids(game.state())
                .expect("hidden")
                .contains(&bomb_id(&bomb))
        );
    }

    #[test]
    fn adjacency_handles_circumferential_radial_and_bull_neighbors() {
        assert!(is_adjacent(
            &hit(&target(1, Ring::Triple), 1),
            &target(20, Ring::Triple)
        ));
        assert!(is_adjacent(
            &hit(&target(20, Ring::SingleOuter), 1),
            &target(20, Ring::Triple)
        ));
        assert!(is_adjacent(
            &hit(&target(25, Ring::SingleBull), 1),
            &target(25, Ring::DoubleBull)
        ));
    }

    #[test]
    fn escalating_growth_and_memory_are_round_scoped() {
        let mut game = game(&json!({
            "rounds": 3,
            "bomb_count": 4,
            "bomb_growth": "escalating",
            "hidden_bombs": "memory",
            "penalty": -50,
        }));
        for seq in 1..=3 {
            game.apply_throw(&DartEvent::Miss {
                seq,
                label: "MISS".into(),
                score: 0,
            })
            .expect("miss");
        }
        game.continue_turn().expect("round two");

        assert_eq!(game.state.round_number, 2);
        assert_eq!(bombs(game.state()).expect("bombs").len(), 6);
        assert_eq!(hidden_ids(game.state()).expect("hidden").len(), 3);
        assert_eq!(game.state.random_cursor, 9);
    }

    #[test]
    fn skipping_the_final_round_finishes_the_game() {
        let mut game = game(&json!({
            "rounds": 3,
            "bomb_count": 4,
            "bomb_growth": "steady",
            "hidden_bombs": "visible",
            "penalty": -50,
        }));
        game.next_player().expect("round one");
        game.next_player().expect("round two");
        game.next_player().expect("round three");
        assert_eq!(game.state.status, GameStatus::Finished);
        assert_eq!(game.state.winner_id.as_deref(), Some("ada"));
    }
}
