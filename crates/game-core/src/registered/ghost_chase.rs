use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{parse_target, ring_name, same_target, sample_targets, target_value, zone_id},
    finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Map, Value, json};

static ROUND_CHOICES: [GameOptionChoice; 2] =
    [choice_integer(5, "5 Runden"), choice_integer(8, "8 Runden")];

static DIFFICULTY_CHOICES: [GameOptionChoice; 3] = [
    GameOptionChoice {
        value: GameOptionValue::Text("easy"),
        label: "Easy · Singles",
        description: Some("Innerer und äußerer Single der Zielzahl fangen den Geist."),
        description_en: Some(
            "Both the inner and outer Single of the target number catch the ghost.",
        ),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("normal"),
        label: "Normal · Alle Ringe",
        description: Some("Das zufällig gewählte Segment muss exakt getroffen werden."),
        description_en: Some("The randomly selected segment must be hit exactly."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("hard"),
        label: "Hard · Double/Triple/Bull",
        description: Some("Ziele erscheinen nur in Double, Triple oder Double Bull."),
        description_en: Some("Targets appear only in Doubles, Triples, or Double Bull."),
    },
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
        key: "difficulty",
        label: "Geisterpfad",
        kind: "choice",
        default: GameOptionValue::Text("normal"),
        choices: &DIFFICULTY_CHOICES,
    },
];

static INSTRUCTIONS: [GameInstruction; 4] = [
    GameInstruction {
        title: "Geist treffen",
        body: "Triff das exakt markierte Segment.",
        icon: "ghost",
    },
    GameInstruction {
        title: "Combo jagen",
        body: "Treffer in einer Aufnahme zählen 40, 50 und 60.",
        icon: "combo",
    },
    GameInstruction {
        title: "Geist flieht",
        body: "Nach drei Fehlversuchen springt er auf ein neues Feld.",
        icon: "dash",
    },
    GameInstruction {
        title: "Gleicher Pfad",
        body: "Alle jagen pro Runde dieselbe Folge von Geisterzielen.",
        icon: "shuffle",
    },
];

static METADATA: GameMetadata = GameMetadata {
    slug: "ghost_chase",
    ruleset_version: 2,
    title: "Ghost Chase",
    tagline: "Fang den hüpfenden Geist",
    description: "Triff den Geist für eine wachsende Dreier-Combo. Nach drei Fehlversuchen flieht er weiter.",
    accent: "#72c9b9",
    accent_secondary: "#f7d488",
    visual: "ghost-chase",
    icon: "ghost",
    artwork: "/static/assets/modes/ghost_chase.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct GhostChaseMode;

pub(super) static GHOST_CHASE_MODE: GhostChaseMode = GhostChaseMode;

impl GameMode for GhostChaseMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let combo = player_counter(state, 0);
        let escape = player_counter(state, 0);
        let path_index = player_counter(state, 0);
        state.mode_state = json!({
            "combo": combo,
            "escape": escape,
            "path_index": path_index,
        });
        generate_round_path(state)
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        let player_index = state.current_player_index;
        let player_id = state.players[player_index].id.clone();
        let target = current_target(state, &player_id)?;
        let combo = counter(state, "combo", &player_id)?;
        let easy = difficulty(state)? == "easy";
        let easy_single = easy
            && matches!(
                event,
                DartEvent::Hit { field, ring: Ring::SingleInner | Ring::SingleOuter, .. }
                    if *field == target.field
            );
        let points = if easy_single || same_target(event, &target) {
            let points = 40_i64.saturating_add(i64::from(combo.min(2)) * 10);
            state.players[player_index].score =
                state.players[player_index].score.saturating_add(points);
            set_counter(state, "combo", &player_id, combo.saturating_add(1))?;
            set_counter(state, "escape", &player_id, 0)?;
            increment_counter(state, "path_index", &player_id)?;
            state.message = format!("GHOST CAUGHT! +{points}");
            points
        } else {
            set_counter(state, "combo", &player_id, 0)?;
            let escape = counter(state, "escape", &player_id)?.saturating_add(1);
            if escape >= 3 {
                increment_counter(state, "path_index", &player_id)?;
                set_counter(state, "escape", &player_id, 0)?;
                state.message = "WHOOSH! Der Geist ist geflohen".into();
            } else {
                set_counter(state, "escape", &player_id, escape)?;
                state.message = "Der Geist bleibt".into();
            }
            0
        };
        finish_fixed_round_game(state, "{winner} ist der beste Geisterjäger!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(player) = state.players.get(state.current_player_index) else {
            return json!({"prompt": "Fang den Geist!", "targets": []});
        };
        let Ok(target) = current_target(state, &player.id) else {
            return json!({"prompt": "Fang den Geist!", "targets": []});
        };
        let easy = difficulty(state).unwrap_or("normal") == "easy";
        let combo = counter(state, "combo", &player.id).unwrap_or_default();
        let escape = counter(state, "escape", &player.id).unwrap_or_default();
        let targets = if easy {
            [Ring::SingleInner, Ring::SingleOuter]
                .into_iter()
                .map(|ring| {
                    json!({
                        "id": format!("FIELD-{}-{}", target.field, ring_name(ring)),
                        "field": target.field,
                        "ring": ring,
                        "color": "cyan",
                        "label": if ring == Ring::SingleOuter { "👻" } else { "" },
                        "pulse": true,
                    })
                })
                .collect::<Vec<_>>()
        } else {
            vec![json!({
                "id": zone_id(&target),
                "field": target.field,
                "ring": target.ring,
                "color": "cyan",
                "label": "👻",
                "pulse": true,
            })]
        };
        json!({
            "prompt": if easy {
                format!("Fang Single {}!", target.field)
            } else {
                format!("Fang {}!", target.label)
            },
            "targets": targets,
            "combo": {"count": combo, "bonus": u32::from(combo) * 10},
            "panel": {
                "title": "GHOST CHAIN",
                "headline": format!("Combo ×{combo}"),
                "subline": format!("Fluchtladung {escape}/3"),
                "progress": {"value": escape, "max": 3},
            },
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let path_round = state
            .mode_state
            .get("path_round")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or_default();
        if path_round != state.round_number {
            generate_round_path(state)?;
        }
        let player_id = state
            .players
            .get(state.current_player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        set_counter(state, "combo", &player_id, 0)?;
        set_counter(state, "escape", &player_id, 0)?;
        set_counter(state, "path_index", &player_id, 0)
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} ist der beste Geisterjäger!")
    }
}

const fn choice_integer(value: i64, label: &'static str) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Integer(value),
        label,
        description: None,
        description_en: None,
    }
}

fn player_counter(state: &RegisteredGameState, value: u8) -> Value {
    Value::Object(
        state
            .players
            .iter()
            .map(|player| (player.id.clone(), Value::from(value)))
            .collect::<Map<_, _>>(),
    )
}

fn difficulty(state: &RegisteredGameState) -> Result<&str, GameError> {
    state
        .options
        .get("difficulty")
        .and_then(Value::as_str)
        .ok_or_else(invalid_state)
}

fn generate_round_path(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let difficulty = difficulty(state)?.to_string();
    let path = sample_targets(state, 4, &difficulty)?;
    state.mode_state["path"] = Value::Array(path.iter().map(target_value).collect::<Vec<_>>());
    state.mode_state["path_round"] = Value::from(state.round_number);
    Ok(())
}

fn current_target(
    state: &RegisteredGameState,
    player_id: &str,
) -> Result<super::arcade::Target, GameError> {
    let path = state
        .mode_state
        .get("path")
        .and_then(Value::as_array)
        .ok_or_else(invalid_state)?;
    let index = usize::from(counter(state, "path_index", player_id)?);
    let selected = index.min(path.len().saturating_sub(1));
    path.get(selected)
        .ok_or_else(invalid_state)
        .and_then(parse_target)
}

fn counter(state: &RegisteredGameState, name: &str, player_id: &str) -> Result<u8, GameError> {
    state
        .mode_state
        .get(name)
        .and_then(Value::as_object)
        .and_then(|values| values.get(player_id))
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn set_counter(
    state: &mut RegisteredGameState,
    name: &str,
    player_id: &str,
    value: u8,
) -> Result<(), GameError> {
    let current = state
        .mode_state
        .get_mut(name)
        .and_then(Value::as_object_mut)
        .and_then(|values| values.get_mut(player_id))
        .ok_or_else(invalid_state)?;
    *current = Value::from(value);
    Ok(())
}

fn increment_counter(
    state: &mut RegisteredGameState,
    name: &str,
    player_id: &str,
) -> Result<(), GameError> {
    let value = counter(state, name, player_id)?.saturating_add(1);
    set_counter(state, name, player_id, value)
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid ghost_chase mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameStatus, RegisteredGame, registered::advance_player};

    #[test]
    fn easy_accepts_the_inner_single_and_lights_both_single_areas() {
        let mut game = RegisteredGame::new_seeded(
            "ghost_chase",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 5, "difficulty": "easy"}),
            42,
        )
        .expect("game");
        let field = current_target(game.state(), "ada").expect("target").field;

        game.apply_throw(&DartEvent::Hit {
            seq: 1,
            field,
            ring: Ring::SingleInner,
            multiplier: 1,
            label: format!("S{field}"),
            score: u16::from(field),
        })
        .expect("ghost hit");

        assert_eq!(game.state().players[0].score, 40);
        let rings = game.state().overlay["targets"]
            .as_array()
            .expect("targets")
            .iter()
            .map(|target| target["ring"].as_str().expect("ring"))
            .collect::<Vec<_>>();
        assert_eq!(rings, ["single_inner", "single_outer"]);
    }

    #[test]
    fn wrapping_to_a_new_round_generates_one_new_shared_path() {
        let mut game = RegisteredGame::new_seeded(
            "ghost_chase",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"rounds": 5, "difficulty": "normal"}),
            42,
        )
        .expect("game");
        let first_path = game.state.mode_state["path"].clone();
        game.state.current_player_index = 1;
        game.state.status = GameStatus::Hold;

        advance_player(&GHOST_CHASE_MODE, &mut game.state).expect("new round");

        assert_eq!(game.state.round_number, 2);
        assert_eq!(game.state.random_cursor, 8);
        assert_ne!(game.state.mode_state["path"], first_path);
    }
}
