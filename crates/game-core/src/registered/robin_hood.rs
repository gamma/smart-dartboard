use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{
        Target, parse_target, ring_name, same_field, same_target, sample_targets, target_value,
        zone_id,
    },
    finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Map, Value, json};

static ROUND_CHOICES: [GameOptionChoice; 2] =
    [choice_integer(5, "5 Runden"), choice_integer(8, "8 Runden")];

static MATCHING_CHOICES: [GameOptionChoice; 2] = [
    GameOptionChoice {
        value: GameOptionValue::Text("exact"),
        label: "Exact Ring",
        description: Some("Zahl und physischer Ring des Sheriff-Pfeils müssen stimmen."),
        description_en: Some("Both the number and physical ring of the Sheriff arrow must match."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("number"),
        label: "Same Number",
        description: Some("Jeder Ring derselben Zahl spaltet den Sheriff-Pfeil."),
        description_en: Some("Any ring of the same number splits the Sheriff arrow."),
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
        key: "matching",
        label: "Trefferregel",
        kind: "choice",
        default: GameOptionValue::Text("exact"),
        choices: &MATCHING_CHOICES,
    },
];

static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Pfeile jagen",
        body: "Triff die Sheriff-Ziele. Doppelte Ziele zählen separat.",
        icon: "target",
    },
    GameInstruction {
        title: "Split-Punkte",
        body: "Ein Split gibt 30 Punkte plus den Wert des Sheriff-Pfeils.",
        icon: "arrow",
    },
    GameInstruction {
        title: "Ziele weitergeben",
        body: "Deine gültigen Treffer werden die Ziele des nächsten Spielers.",
        icon: "shuffle",
    },
];

static METADATA: GameMetadata = GameMetadata {
    slug: "robin_hood",
    ruleset_version: 2,
    title: "Robin Hood Hunt",
    tagline: "Spalte die Sheriff-Pfeile",
    description: "Jage die drei Ziele des Vorgängers. Jeder eigene Treffer wird danach zum Ziel für den nächsten Spieler.",
    accent: "#5aa469",
    accent_secondary: "#f4b942",
    visual: "robin-hood",
    icon: "arrow",
    artwork: "/static/assets/modes/robin_hood.webp",
    sound_theme: "arcade",
    min_players: 2,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct RobinHoodMode;

pub(super) static ROBIN_HOOD_MODE: RobinHoodMode = RobinHoodMode;

impl GameMode for RobinHoodMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let targets = sample_targets(state, 3, "normal")?;
        let target_values = targets.iter().map(target_value).collect::<Vec<_>>();
        state.mode_state = json!({
            "sheriff_targets": target_values,
            "remaining_targets": target_values,
            "current_arrows": [],
            "splits": state.players.iter().map(|player| (player.id.clone(), Value::from(0))).collect::<Map<_, _>>(),
        });
        state.message = "Die Sheriff-Pfeile liegen bereit!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        if let Some(arrow) = event_target(event) {
            state
                .mode_state
                .get_mut("current_arrows")
                .and_then(Value::as_array_mut)
                .ok_or_else(invalid_state)?
                .push(target_value(&arrow));
        }

        let matching = matching(state)?.to_string();
        let match_index = state
            .mode_state
            .get("remaining_targets")
            .and_then(Value::as_array)
            .ok_or_else(invalid_state)?
            .iter()
            .enumerate()
            .find_map(|(index, value)| {
                let target = parse_target(value).ok()?;
                let matches = if matching == "exact" {
                    same_target(event, &target)
                } else {
                    same_field(event, &target)
                };
                matches.then_some(index)
            });

        let player_index = state.current_player_index;
        let player_id = state.players[player_index].id.clone();
        let points = if let Some(index) = match_index {
            let target = state
                .mode_state
                .get_mut("remaining_targets")
                .and_then(Value::as_array_mut)
                .and_then(|targets| (index < targets.len()).then(|| targets.remove(index)))
                .ok_or_else(invalid_state)
                .and_then(|target| parse_target(&target))?;
            let points = 30_i64.saturating_add(i64::from(target.score));
            state.players[player_index].score =
                state.players[player_index].score.saturating_add(points);
            increment_split(state, &player_id)?;
            state.message = format!("SPLIT! {} +{points}", target.label);
            points
        } else {
            state.message = "Kein Sheriff-Pfeil gespalten".into();
            0
        };

        if state.darts_in_turn.saturating_add(1) >= 3 {
            pass_current_arrows(state)?;
        }
        finish_fixed_round_game(state, "{winner} ist der beste Pfeilspalter!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let remaining = target_list(state, "remaining_targets").unwrap_or_default();
        let sheriff = target_list(state, "sheriff_targets").unwrap_or_default();
        let shown = if state.status == crate::GameStatus::Hold {
            &sheriff
        } else {
            &remaining
        };
        let number_matching = matching(state).unwrap_or("exact") == "number";
        let mut overlay_targets = Vec::new();
        for target in shown {
            if number_matching && target.field != 25 {
                overlay_targets.extend(
                    [
                        Ring::SingleInner,
                        Ring::Triple,
                        Ring::SingleOuter,
                        Ring::Double,
                    ]
                    .into_iter()
                    .map(|ring| {
                        json!({
                            "id": format!("FIELD-{}-{}", target.field, ring_name(ring)),
                            "field": target.field,
                            "ring": ring,
                            "color": "green",
                            "label": "SPLIT",
                            "pulse": false,
                        })
                    }),
                );
            } else {
                overlay_targets.push(json!({
                    "id": zone_id(target),
                    "field": target.field,
                    "ring": target.ring,
                    "color": "green",
                    "label": "SPLIT",
                    "pulse": false,
                }));
            }
        }
        let prompt = if shown.is_empty() {
            "Freie Runde – lege neue Pfeile!".into()
        } else {
            format!(
                "Spalte: {}",
                shown
                    .iter()
                    .map(|target| if number_matching {
                        target.field.to_string()
                    } else {
                        target.label.clone()
                    })
                    .collect::<Vec<_>>()
                    .join(" · ")
            )
        };
        let rows = state
            .players
            .iter()
            .map(|player| {
                json!({
                    "label": player.name,
                    "value": format!("{} Splits", split_count(state, &player.id).unwrap_or_default()),
                })
            })
            .collect::<Vec<_>>();
        json!({
            "prompt": prompt,
            "targets": overlay_targets,
            "panel": {
                "title": "SHERIFF-PFEILE",
                "headline": format!("{} noch offen", remaining.len()),
                "rows": rows,
            },
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state["remaining_targets"] = state.mode_state["sheriff_targets"].clone();
        state.mode_state["current_arrows"] = Value::Array(Vec::new());
        Ok(())
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        pass_current_arrows(state)
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} ist der beste Pfeilspalter!")
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

fn matching(state: &RegisteredGameState) -> Result<&str, GameError> {
    state
        .options
        .get("matching")
        .and_then(Value::as_str)
        .ok_or_else(invalid_state)
}

fn event_target(event: &DartEvent) -> Option<Target> {
    let DartEvent::Hit {
        label,
        field,
        ring,
        multiplier,
        score,
        ..
    } = event
    else {
        return None;
    };
    Some(Target {
        label: label.clone(),
        field: *field,
        ring: *ring,
        multiplier: *multiplier,
        score: *score,
    })
}

fn target_list(state: &RegisteredGameState, name: &str) -> Result<Vec<Target>, GameError> {
    state
        .mode_state
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(invalid_state)?
        .iter()
        .map(parse_target)
        .collect()
}

fn split_count(state: &RegisteredGameState, player_id: &str) -> Result<u64, GameError> {
    state
        .mode_state
        .get("splits")
        .and_then(Value::as_object)
        .and_then(|splits| splits.get(player_id))
        .and_then(Value::as_u64)
        .ok_or_else(invalid_state)
}

fn increment_split(state: &mut RegisteredGameState, player_id: &str) -> Result<(), GameError> {
    let count = split_count(state, player_id)?.saturating_add(1);
    state
        .mode_state
        .get_mut("splits")
        .and_then(Value::as_object_mut)
        .ok_or_else(invalid_state)?
        .insert(player_id.into(), Value::from(count));
    Ok(())
}

fn pass_current_arrows(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let arrows = state
        .mode_state
        .get("current_arrows")
        .and_then(Value::as_array)
        .ok_or_else(invalid_state)?
        .clone();
    state.mode_state["sheriff_targets"] = Value::Array(arrows);
    Ok(())
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid robin_hood mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    fn t20(seq: u64) -> DartEvent {
        DartEvent::Hit {
            seq,
            field: 20,
            ring: Ring::Triple,
            multiplier: 3,
            label: "T20".into(),
            score: 60,
        }
    }

    #[test]
    fn duplicate_sheriff_arrows_are_split_once_each() {
        let mut game = RegisteredGame::new_seeded(
            "robin_hood",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({}),
            42,
        )
        .expect("game");
        let target = target_value(&event_target(&t20(1)).expect("target"));
        game.state.mode_state["sheriff_targets"] = json!([target, target]);
        game.state.mode_state["remaining_targets"] = json!([target, target]);

        game.apply_throw(&t20(1)).expect("first split");
        game.apply_throw(&t20(2)).expect("second split");

        assert_eq!(game.state.players[0].score, 180);
        assert_eq!(game.state.mode_state["remaining_targets"], json!([]));
        assert_eq!(split_count(game.state(), "ada").expect("splits"), 2);
    }

    #[test]
    fn number_matching_lights_all_four_rings() {
        let game = RegisteredGame::new_seeded(
            "robin_hood",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"rounds": 5, "matching": "number"}),
            42,
        )
        .expect("game");
        let field = game.state.mode_state["remaining_targets"][0]["field"]
            .as_u64()
            .expect("field");
        let rings = game.state.overlay["targets"]
            .as_array()
            .expect("targets")
            .iter()
            .filter(|target| target["field"].as_u64() == Some(field))
            .map(|target| target["ring"].as_str().expect("ring"))
            .collect::<Vec<_>>();

        assert_eq!(rings, ["single_inner", "triple", "single_outer", "double"]);
    }

    #[test]
    fn next_player_passes_only_the_current_partial_arrows() {
        let mut game = RegisteredGame::new_seeded(
            "robin_hood",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({}),
            42,
        )
        .expect("game");
        game.apply_throw(&t20(1)).expect("partial turn");

        game.next_player().expect("skip turn");

        assert_eq!(game.state.current_player_index, 1);
        assert_eq!(
            game.state.mode_state["remaining_targets"],
            json!([target_value(&event_target(&t20(1)).expect("target"))])
        );
        assert_eq!(game.state.mode_state["current_arrows"], json!([]));
    }
}
