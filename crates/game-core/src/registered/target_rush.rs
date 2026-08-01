use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState, finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Value, json};

static ROUND_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(3, "3 Runden"),
    choice_integer(5, "5 Runden"),
    choice_integer(8, "8 Runden"),
];

static DIFFICULTY_CHOICES: [GameOptionChoice; 3] = [
    GameOptionChoice {
        value: GameOptionValue::Text("easy"),
        label: "Easy · ganze Zahl",
        description: Some(
            "Alle vier Ringe zählen; dieselbe Zahl bleibt die komplette Runde aktiv.",
        ),
        description_en: Some(
            "All four rings score; the same number stays active for the full round.",
        ),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("normal"),
        label: "Normal · exaktes Segment",
        description: Some(
            "Drei exakte Ziele pro Runde wechseln nach jedem Dart; falscher Ring gibt +10.",
        ),
        description_en: Some(
            "Three exact targets per round change after each dart; the wrong ring scores +10.",
        ),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("hard"),
        label: "Hard · Double/Triple",
        description: Some(
            "Wie Normal, aber die Zielfolge enthält nur Doubles, Triples und Double Bull.",
        ),
        description_en: Some(
            "Like Normal, but the sequence contains only Doubles, Triples, and Double Bull.",
        ),
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
        label: "Ziele",
        kind: "choice",
        default: GameOptionValue::Text("normal"),
        choices: &DIFFICULTY_CHOICES,
    },
];

static INSTRUCTIONS: [GameInstruction; 4] = [
    GameInstruction {
        title: "Easy: ganze Zahl",
        body: "Alle vier Ringe der Zielzahl zählen voll. Das Ziel bleibt für die ganze Runde stehen.",
        icon: "target",
    },
    GameInstruction {
        title: "Normal und Hard",
        body: "Triff das exakte Segment. Die richtige Zahl im falschen Ring gibt Almost-Punkte.",
        icon: "spark",
    },
    GameInstruction {
        title: "Combo sammeln",
        body: "Exakte Treffer in Folge bringen Bonus.",
        icon: "combo",
    },
    GameInstruction {
        title: "Gleiche Chancen",
        body: "Easy gibt allen dasselbe Rundenziel. Normal und Hard geben dieselbe Folge aus drei Zielen.",
        icon: "shuffle",
    },
];

static METADATA: GameMetadata = GameMetadata {
    slug: "target_rush",
    ruleset_version: 2,
    title: "Target Rush",
    tagline: "Triff das leuchtende Ziel",
    description: "Das Board zeigt ein Ziel. Easy nimmt die ganze Zahl, Normal und Hard verlangen das genaue Segment.",
    accent: "#28e7ff",
    accent_secondary: "#3dff91",
    visual: "target-rush",
    icon: "zap",
    artwork: "/static/assets/modes/target_rush.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
};

pub(super) struct TargetRushMode;

pub(super) static TARGET_RUSH_MODE: TargetRushMode = TargetRushMode;

impl GameMode for TargetRushMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({"combo": {}, "last_result": ""});
        generate_round_targets(state)
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<u32, GameError> {
        let target = target(state)?;
        let player_index = state.current_player_index;
        let player_id = state.players[player_index].id.clone();
        let combo = combo(state, &player_id)?;
        let easy = difficulty(state)? == "easy";
        let points = if matches!(event, DartEvent::Miss { .. }) {
            set_combo(state, &player_id, 0)?;
            state.message = "Miss – Combo reset".into();
            0
        } else if (easy && same_field(event, &target)) || same_target(event, &target) {
            let points = 50_u32.saturating_add(u32::from(combo).saturating_mul(10));
            state.players[player_index].score =
                state.players[player_index].score.saturating_add(points);
            set_combo(state, &player_id, combo.saturating_add(1))?;
            let label = if easy {
                target.field.to_string()
            } else {
                target.label.clone()
            };
            state.message = format!("Perfect {label}! +{points}");
            points
        } else if same_field(event, &target) {
            state.players[player_index].score =
                state.players[player_index].score.saturating_add(10);
            set_combo(state, &player_id, 0)?;
            state.message = format!("Almost {} +10", event.label());
            10
        } else {
            set_combo(state, &player_id, 0)?;
            state.message = format!("Falsches Feld: {}", event.label());
            0
        };

        if !easy && state.darts_in_turn.saturating_add(1) < 3 {
            select_target(state, usize::from(state.darts_in_turn.saturating_add(1)))?;
        }
        finish_fixed_round_game(state, "{winner} gewinnt den Target Rush!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Ok(target) = target(state) else {
            return json!({"prompt": "Target Rush", "targets": []});
        };
        let easy = difficulty(state).unwrap_or("normal") == "easy";
        let player_id = state
            .players
            .get(state.current_player_index)
            .map(|player| player.id.as_str())
            .unwrap_or_default();
        let combo = combo(state, player_id).unwrap_or_default();
        let targets = if easy {
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
                    "color": "cyan",
                    "label": if ring == Ring::SingleOuter { "+50" } else { "" },
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
                "label": "+50",
                "pulse": true,
            })]
        };
        json!({
            "prompt": if easy {
                format!("Triff die {}!", target.field)
            } else {
                format!("Triff {}!", target.label)
            },
            "targets": targets,
            "combo": {"count": combo, "bonus": u32::from(combo) * 10},
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let target_round = state
            .mode_state
            .get("target_round")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or_default();
        if target_round == state.round_number {
            select_target(state, 0)
        } else {
            generate_round_targets(state)
        }
    }
}

#[derive(Clone)]
struct Target {
    label: String,
    field: u8,
    ring: Ring,
    multiplier: u8,
    score: u16,
}

const fn choice_integer(value: i64, label: &'static str) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Integer(value),
        label,
        description: None,
        description_en: None,
    }
}

fn difficulty(state: &RegisteredGameState) -> Result<&str, GameError> {
    state
        .options
        .get("difficulty")
        .and_then(Value::as_str)
        .ok_or_else(invalid_state)
}

fn generate_round_targets(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let difficulty = difficulty(state)?.to_string();
    let count = if difficulty == "easy" { 1 } else { 3 };
    let mut available = target_pool(&difficulty);
    let mut targets = Vec::with_capacity(count);
    for _ in 0..count.min(available.len()) {
        let index = state.random_index(available.len())?;
        targets.push(available.remove(index));
    }
    state.mode_state["target_round"] = Value::from(state.round_number);
    state.mode_state["round_targets"] =
        Value::Array(targets.iter().map(target_value).collect::<Vec<_>>());
    select_target(state, 0)
}

fn select_target(state: &mut RegisteredGameState, index: usize) -> Result<(), GameError> {
    let targets = state
        .mode_state
        .get("round_targets")
        .and_then(Value::as_array)
        .ok_or_else(invalid_state)?;
    let selected = index.min(targets.len().saturating_sub(1));
    let target = targets.get(selected).cloned().ok_or_else(invalid_state)?;
    state.mode_state["target_index"] = Value::from(selected);
    state.mode_state["target"] = target.clone();
    let parsed = parse_target(&target)?;
    state.message = if difficulty(state)? == "easy" {
        format!("Triff die {}!", parsed.field)
    } else {
        format!("Triff {}!", parsed.label)
    };
    Ok(())
}

fn target(state: &RegisteredGameState) -> Result<Target, GameError> {
    state
        .mode_state
        .get("target")
        .ok_or_else(invalid_state)
        .and_then(parse_target)
}

fn combo(state: &RegisteredGameState, player_id: &str) -> Result<u8, GameError> {
    Ok(state
        .mode_state
        .get("combo")
        .and_then(Value::as_object)
        .ok_or_else(invalid_state)?
        .get(player_id)
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .unwrap_or_default())
}

fn set_combo(state: &mut RegisteredGameState, player_id: &str, value: u8) -> Result<(), GameError> {
    state
        .mode_state
        .get_mut("combo")
        .and_then(Value::as_object_mut)
        .ok_or_else(invalid_state)?
        .insert(player_id.into(), Value::from(value));
    Ok(())
}

fn target_pool(difficulty: &str) -> Vec<Target> {
    let mut targets = Vec::new();
    if difficulty == "easy" || difficulty == "normal" {
        targets.extend((1..=20).map(|field| target_for(field, Ring::SingleOuter)));
    }
    if difficulty != "easy" {
        targets.extend((1..=20).map(|field| target_for(field, Ring::Double)));
        targets.extend((1..=20).map(|field| target_for(field, Ring::Triple)));
        targets.push(target_for(25, Ring::DoubleBull));
    }
    targets
}

fn target_for(field: u8, ring: Ring) -> Target {
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

fn target_value(target: &Target) -> Value {
    json!({
        "label": target.label,
        "field": target.field,
        "ring": target.ring,
        "multiplier": target.multiplier,
        "score": target.score,
    })
}

fn parse_target(value: &Value) -> Result<Target, GameError> {
    serde_json::from_value::<DartEvent>(json!({
        "type": "hit",
        "seq": 0,
        "field": value.get("field"),
        "ring": value.get("ring"),
        "multiplier": value.get("multiplier"),
        "label": value.get("label"),
        "score": value.get("score"),
    }))
    .map_err(|_| invalid_state())
    .and_then(|event| match event {
        DartEvent::Hit {
            label,
            field,
            ring,
            multiplier,
            score,
            ..
        } => Ok(Target {
            label,
            field,
            ring,
            multiplier,
            score,
        }),
        DartEvent::Miss { .. } => Err(invalid_state()),
    })
}

fn same_field(event: &DartEvent, target: &Target) -> bool {
    matches!(event, DartEvent::Hit { field, .. } if *field == target.field)
}

fn same_target(event: &DartEvent, target: &Target) -> bool {
    matches!(event, DartEvent::Hit { field, ring, .. } if *field == target.field && *ring == target.ring)
}

fn zone_id(target: &Target) -> String {
    match target.ring {
        Ring::SingleBull => "SBULL".into(),
        Ring::DoubleBull => "DBULL".into(),
        _ => target.label.clone(),
    }
}

const fn ring_name(ring: Ring) -> &'static str {
    match ring {
        Ring::SingleInner => "single_inner",
        Ring::SingleOuter => "single_outer",
        Ring::Triple => "triple",
        Ring::Double => "double",
        Ring::SingleBull => "single_bull",
        Ring::DoubleBull => "double_bull",
    }
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid target_rush mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameStatus, RegisteredGame};

    #[test]
    fn easy_accepts_any_ring_and_lights_the_whole_number() {
        let mut game = RegisteredGame::new_seeded(
            "target_rush",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 3, "difficulty": "easy"}),
            42,
        )
        .expect("game");
        let field = target(game.state()).expect("target").field;

        game.apply_throw(&DartEvent::Hit {
            seq: 1,
            field,
            ring: Ring::Triple,
            multiplier: 3,
            label: format!("T{field}"),
            score: u16::from(field) * 3,
        })
        .expect("easy hit");

        assert_eq!(game.state().players[0].score, 50);
        let rings = game.state().overlay["targets"]
            .as_array()
            .expect("targets")
            .iter()
            .map(|target| target["ring"].as_str().expect("ring"))
            .collect::<Vec<_>>();
        assert_eq!(rings, ["single_inner", "triple", "single_outer", "double"]);
    }

    #[test]
    fn tied_final_visit_finishes_without_an_arbitrary_winner() {
        let mut game = RegisteredGame::new_seeded(
            "target_rush",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"rounds": 3, "difficulty": "normal"}),
            42,
        )
        .expect("game");
        game.state.round_number = 3;
        game.state.current_player_index = 1;
        game.state.darts_in_turn = 2;

        game.apply_throw(&DartEvent::Miss {
            seq: 1,
            label: "MISS".into(),
            score: 0,
        })
        .expect("final miss");

        assert_eq!(game.state().status, GameStatus::Finished);
        assert_eq!(game.state().result_type, "draw");
        assert_eq!(game.state().winner_id, None);
        assert!(game.state().winner_ids.is_empty());
    }
}
