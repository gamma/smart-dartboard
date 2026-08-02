use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{Target, parse_target, same_target, target_pool, target_value, zone_id},
    finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::DartEvent;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

static ROUND_CHOICES: [GameOptionChoice; 2] = [choice(5, "5 Runden"), choice(8, "8 Runden")];
static EGG_CHOICES: [GameOptionChoice; 3] = [
    choice(3, "3 Eier"),
    choice(4, "4 Eier"),
    choice(6, "6 Eier"),
];
static OPTIONS: [GameOption; 2] = [
    GameOption {
        key: "rounds",
        label: "Runden",
        kind: "choice",
        default: GameOptionValue::Integer(5),
        choices: &ROUND_CHOICES,
    },
    GameOption {
        key: "eggs",
        label: "Dracheneier",
        kind: "choice",
        default: GameOptionValue::Integer(4),
        choices: &EGG_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Goldenes Ei",
        body: "Ein sichtbares Ei bringt einmal pro Runde +30 Punkte.",
        icon: "egg",
    },
    GameInstruction {
        title: "Rote Schuppe",
        body: "Eine Schuppe kostet 15 Punkte und füllt eine Flamme.",
        icon: "danger",
    },
    GameInstruction {
        title: "Drachenfeuer",
        body: "Die dritte Flamme verbrennt zusätzlich die Hälfte deiner positiven Punkte dieses Zugs.",
        icon: "dragon",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "dragon_eggs",
    ruleset_version: 2,
    format: sdb_contracts::GameFormat::Individual,
    title: "Dragon Eggs",
    tagline: "Eier bergen, Drachenfeuer vermeiden",
    description: "Sammle goldene Eier. Jede rote Schuppe heizt den Drachen auf – die dritte entfacht sein Feuer.",
    accent: "#f4a261",
    accent_secondary: "#6ab04c",
    visual: "dragon-eggs",
    icon: "egg",
    artwork: "/static/assets/modes/dragon_eggs.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct DragonEggsMode;
pub(super) static DRAGON_EGGS_MODE: DragonEggsMode = DragonEggsMode;

impl GameMode for DragonEggsMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({
            "heat": player_values(state, 0),
            "turn_positive": player_values(state, 0),
            "collected": player_collections(state),
            "layout_round": state.round_number,
            "last_effect": "",
            "effect_points": 0,
            "dragon_heat": 0,
            "dragon_fire_penalty": 0,
        });
        shuffle(state)
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        let player_id = state
            .players
            .get(state.current_player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        let old_heat = counter(state, "heat", &player_id)?;
        set_effect(state, "", 0, old_heat, 0);
        let egg = matching_target(state, "eggs", event)?;
        let scale = matching_target(state, "scales", event)?.is_some();

        let points = if let Some(egg) = egg {
            collect_egg(state, &player_id, &egg)?
        } else if scale {
            hit_scale(state, &player_id)?
        } else {
            state.message = "Kein Ei gefunden".into();
            0
        };
        state.players[state.current_player_index].score = state.players[state.current_player_index]
            .score
            .saturating_add(points);
        finish_fixed_round_game(state, "{winner} hütet den Drachenschatz!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(player) = state.players.get(state.current_player_index) else {
            return json!({"prompt": "GOLDENE EIER SAMMELN", "bonus": [], "danger": []});
        };
        let heat = counter(state, "heat", &player.id).unwrap_or_default();
        let collected = collection(state, &player.id).unwrap_or_default();
        let bonus = targets(state, "eggs")
            .unwrap_or_default()
            .into_iter()
            .filter(|target| !collected.contains(&zone_id(target)))
            .map(|target| overlay_target(&target, "gold", "+30", false, "egg", "dragon-egg"))
            .collect::<Vec<_>>();
        let danger = targets(state, "scales")
            .unwrap_or_default()
            .into_iter()
            .map(|target| {
                overlay_target(
                    &target,
                    "red",
                    "HITZE",
                    true,
                    "dragon_scale",
                    "dragon-scale",
                )
            })
            .collect::<Vec<_>>();
        json!({
            "prompt": "GOLDENE EIER SAMMELN · ROTE SCHUPPEN MEIDEN",
            "bonus": bonus,
            "danger": danger,
            "visual_legend": [
                {"icon":"egg","label":"GOLDENES EI","value":"+30","color":"#f4c95d"},
                {"icon":"dragon_scale","label":"ROTE SCHUPPE","value":"-15 · +1 HITZE","color":"#f05d5e"},
            ],
            "panel": {
                "kind": "dragon_heat",
                "title": "DRACHEN-HITZE",
                "heat": heat,
                "headline": format!("{heat}/3 FLAMMEN"),
                "subline": if heat == 2 { "Noch eine Schuppe: Feuer!" } else { "Die dritte Schuppe entfacht das Feuer" },
                "progress": {"value": heat, "max": 3},
            },
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let player_id = state
            .players
            .get(state.current_player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        set_counter(state, "turn_positive", &player_id, 0)?;
        let layout_round = state.mode_state["layout_round"]
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(invalid_state)?;
        if layout_round != state.round_number {
            shuffle(state)?;
            state.mode_state["layout_round"] = Value::from(state.round_number);
        }
        Ok(())
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} hütet den Drachenschatz!")
    }
}

fn collect_egg(
    state: &mut RegisteredGameState,
    player_id: &str,
    egg: &Target,
) -> Result<i64, GameError> {
    let id = zone_id(egg);
    let mut collected = collection(state, player_id)?;
    if collected.contains(&id) {
        state.message = "Dieses Ei ist schon leer".into();
        return Ok(0);
    }
    collected.insert(id);
    state.mode_state["collected"][player_id] =
        Value::Array(collected.into_iter().map(Value::from).collect());
    let positive = counter(state, "turn_positive", player_id)?.saturating_add(30);
    set_counter(state, "turn_positive", player_id, positive)?;
    set_effect(
        state,
        "dragon_egg",
        30,
        counter(state, "heat", player_id)?,
        0,
    );
    state.message = "Ei geknackt! +30".into();
    Ok(30)
}

fn hit_scale(state: &mut RegisteredGameState, player_id: &str) -> Result<i64, GameError> {
    let mut heat = counter(state, "heat", player_id)?.saturating_add(1);
    let mut points = -15_i64;
    if heat >= 3 {
        let penalty = i64::from(counter(state, "turn_positive", player_id)? / 2);
        points = points.saturating_sub(penalty);
        heat = 0;
        state.message = format!("DRACHENFEUER! -{}", 15_i64.saturating_add(penalty));
        set_effect(
            state,
            "dragon_fire",
            points,
            heat,
            15_i64.saturating_add(penalty),
        );
    } else {
        state.message = format!("Schuppe! -15 · Hitze {heat}/3");
        set_effect(state, "dragon_scale", points, heat, 0);
    }
    set_counter(state, "heat", player_id, heat)?;
    Ok(points)
}

fn shuffle(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let eggs = choose(state, egg_count(state)?, &BTreeSet::new())?;
    let excluded = eggs.iter().map(zone_id).collect::<BTreeSet<_>>();
    let scales = choose(state, 8, &excluded)?;
    state.mode_state["eggs"] = Value::Array(eggs.iter().map(target_value).collect());
    state.mode_state["scales"] = Value::Array(scales.iter().map(target_value).collect());
    state.mode_state["collected"] = player_collections(state);
    state.message = "Goldene Eier sammeln · rote Schuppen meiden!".into();
    Ok(())
}

fn choose(
    state: &mut RegisteredGameState,
    count: usize,
    excluded: &BTreeSet<String>,
) -> Result<Vec<Target>, GameError> {
    let mut available = target_pool("normal")
        .into_iter()
        .filter(|target| !excluded.contains(&zone_id(target)))
        .collect::<Vec<_>>();
    let mut selected = Vec::with_capacity(count);
    for _ in 0..count.min(available.len()) {
        let index = state.random_index(available.len())?;
        selected.push(available.remove(index));
    }
    Ok(selected)
}

fn matching_target(
    state: &RegisteredGameState,
    key: &str,
    event: &DartEvent,
) -> Result<Option<Target>, GameError> {
    Ok(targets(state, key)?
        .into_iter()
        .find(|target| same_target(event, target)))
}

fn targets(state: &RegisteredGameState, key: &str) -> Result<Vec<Target>, GameError> {
    state.mode_state[key]
        .as_array()
        .ok_or_else(invalid_state)?
        .iter()
        .map(parse_target)
        .collect()
}

fn overlay_target(
    target: &Target,
    color: &str,
    label: &str,
    pulse: bool,
    icon: &str,
    variant: &str,
) -> Value {
    json!({"id":zone_id(target),"field":target.field,"ring":target.ring,"color":color,"label":label,"pulse":pulse,"icon":icon,"variant":variant})
}

fn player_values(state: &RegisteredGameState, value: u8) -> Value {
    Value::Object(
        state
            .players
            .iter()
            .map(|player| (player.id.clone(), Value::from(value)))
            .collect::<Map<_, _>>(),
    )
}

fn player_collections(state: &RegisteredGameState) -> Value {
    Value::Object(
        state
            .players
            .iter()
            .map(|player| (player.id.clone(), Value::Array(Vec::new())))
            .collect::<Map<_, _>>(),
    )
}

fn counter(state: &RegisteredGameState, key: &str, player_id: &str) -> Result<u8, GameError> {
    state.mode_state[key][player_id]
        .as_u64()
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn set_counter(
    state: &mut RegisteredGameState,
    key: &str,
    player_id: &str,
    value: u8,
) -> Result<(), GameError> {
    if !state.mode_state[key]
        .as_object()
        .is_some_and(|map| map.contains_key(player_id))
    {
        return Err(invalid_state());
    }
    state.mode_state[key][player_id] = Value::from(value);
    Ok(())
}

fn collection(state: &RegisteredGameState, player_id: &str) -> Result<BTreeSet<String>, GameError> {
    state.mode_state["collected"][player_id]
        .as_array()
        .ok_or_else(invalid_state)?
        .iter()
        .map(|value| value.as_str().map(str::to_owned).ok_or_else(invalid_state))
        .collect()
}

fn set_effect(state: &mut RegisteredGameState, effect: &str, points: i64, heat: u8, penalty: i64) {
    state.mode_state["last_effect"] = Value::from(effect);
    state.mode_state["effect_points"] = Value::from(points);
    state.mode_state["dragon_heat"] = Value::from(heat);
    state.mode_state["dragon_fire_penalty"] = Value::from(penalty);
}

fn egg_count(state: &RegisteredGameState) -> Result<usize, GameError> {
    state.options["eggs"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invalid_state)
}

const fn choice(value: i64, label: &'static str) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Integer(value),
        label,
        description: None,
        description_en: None,
    }
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid dragon_eggs mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;
    use sdb_contracts::Ring;

    fn game() -> RegisteredGame {
        RegisteredGame::new_seeded(
            "dragon_eggs",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"rounds":5,"eggs":4}),
            42,
        )
        .expect("game")
    }

    fn hit(seq: u64, field: u8, ring: Ring) -> DartEvent {
        let multiplier = match ring {
            Ring::Triple => 3,
            Ring::Double | Ring::DoubleBull => 2,
            _ => 1,
        };
        DartEvent::Hit {
            seq,
            field,
            ring,
            multiplier,
            label: if field == 25 {
                if ring == Ring::DoubleBull {
                    "DBull"
                } else {
                    "SBull"
                }
                .into()
            } else {
                format!(
                    "{}{field}",
                    if ring == Ring::Triple {
                        "T"
                    } else if ring == Ring::Double {
                        "D"
                    } else {
                        "S"
                    }
                )
            },
            score: u16::from(field) * u16::from(multiplier),
        }
    }

    #[test]
    fn eggs_are_personal_and_fire_penalizes_positive_turn_points() {
        let mut game = game();
        game.state.mode_state["heat"]["ada"] = Value::from(2);
        game.apply_throw(&hit(1, 4, Ring::Double)).expect("egg");
        game.apply_throw(&hit(2, 17, Ring::Double)).expect("fire");

        assert_eq!(game.state().players[0].score, 0);
        assert_eq!(game.state().mode_state["heat"]["ada"], 0);
        assert_eq!(game.state().mode_state["last_effect"], "dragon_fire");
        assert_eq!(game.state().mode_state["dragon_fire_penalty"], 30);
        assert_eq!(game.state().mode_state["collected"]["ada"], json!(["D4"]));
        assert_eq!(game.state().mode_state["collected"]["bob"], json!([]));
    }

    #[test]
    fn shared_layout_changes_only_after_every_player_finishes_the_round() {
        let mut game = game();
        let first = game.state().mode_state["eggs"].clone();
        assert_eq!(game.state().random_cursor, 12);
        game.next_player().expect("Bob");
        assert_eq!(game.state().mode_state["eggs"], first);
        game.next_player().expect("round two");
        assert_ne!(game.state().mode_state["eggs"], first);
        assert_eq!(game.state().mode_state["layout_round"], 2);
        assert_eq!(game.state().random_cursor, 24);
    }

    #[test]
    fn correction_replays_heat_and_personal_collection() {
        let mut game = game();
        game.next_player().expect("Bob");
        game.apply_throw(&hit(1, 17, Ring::Double)).expect("scale");
        game.apply_throw(&hit(2, 17, Ring::Double)).expect("scale");
        game.apply_throw(&hit(3, 17, Ring::Double)).expect("fire");
        game.correct_throw(2, hit(99, 13, Ring::Double))
            .expect("correct first Bob dart");

        assert_eq!(game.state().players[1].score, 0);
        assert_eq!(game.state().mode_state["heat"]["bob"], 2);
        assert_eq!(game.state().mode_state["collected"]["bob"], json!(["D13"]));
        assert_eq!(game.state().mode_state["last_effect"], "dragon_scale");
    }
}
