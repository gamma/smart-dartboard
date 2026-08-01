use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState, finish_action_round_game,
};
use crate::{GameError, GameStatus};
use sdb_contracts::{DartEvent, Ring};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const BOARD_ORDER: [u8; 20] = [
    20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5,
];
const NUMBER_RINGS: [Ring; 4] = [
    Ring::SingleInner,
    Ring::Triple,
    Ring::SingleOuter,
    Ring::Double,
];

static ROUND_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(3, "3 Runden"),
    choice_integer(5, "5 Runden"),
    choice_integer(8, "8 Runden"),
];
static DIFFICULTY_CHOICES: [GameOptionChoice; 4] = [
    choice_text(
        "very_easy",
        "Sehr leicht · 4 Zonen",
        "Fünf benachbarte Zahlen bilden jeweils ein großes Zielgebiet.",
        "Five neighboring numbers form each large target zone.",
    ),
    choice_text(
        "easy",
        "Leicht · 5 Zonen",
        "Vier benachbarte Zahlen bilden jeweils ein Zielgebiet.",
        "Four neighboring numbers form each target zone.",
    ),
    choice_text(
        "normal",
        "Mittel · 10 Zonen",
        "Je zwei benachbarte Zahlen bilden jeweils ein Zielgebiet.",
        "Each pair of neighboring numbers forms one target zone.",
    ),
    choice_text(
        "hard",
        "Schwer · 20 Zahlen",
        "Jede Zahl ist ein eigenes Ziel; der Ring bleibt egal.",
        "Every number is its own target; the ring still does not matter.",
    ),
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
        label: "Zielgröße",
        kind: "choice",
        default: GameOptionValue::Text("easy"),
        choices: &DIFFICULTY_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 5] = [
    GameInstruction {
        title: "Sequenz merken",
        body: "Die leuchtenden Zahlengruppen sind deine Reihenfolge.",
        icon: "memory",
    },
    GameInstruction {
        title: "Jeder Ring zählt",
        body: "Triff eine Zahl der aktuellen Gruppe. Single, Double und Triple sind gleich richtig.",
        icon: "target",
    },
    GameInstruction {
        title: "Bull ist Joker",
        body: "Single Bull und Double Bull erfüllen immer das nächste Ziel.",
        icon: "joker",
    },
    GameInstruction {
        title: "Sequenz wächst",
        body: "Die gemeinsame Aufgabe wächst über die ersten drei Runden.",
        icon: "grow",
    },
    GameInstruction {
        title: "Gleiche Chancen",
        body: "Alle spielen in einer Runde exakt dieselbe Sequenz.",
        icon: "shuffle",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "simon_says",
    ruleset_version: 2,
    title: "Simon Says",
    tagline: "Merken, treffen, erweitern",
    description: "Der Projector zeigt eine Sequenz. Triff die Ziele in der richtigen Reihenfolge.",
    accent: "#3dff91",
    accent_secondary: "#9b5cff",
    visual: "simon-says",
    icon: "memory",
    artwork: "/static/assets/modes/simon_says.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Zone {
    zone: usize,
    fields: Vec<u8>,
}

pub(super) struct SimonSaysMode;
pub(super) static SIMON_SAYS_MODE: SimonSaysMode = SimonSaysMode;

impl GameMode for SimonSaysMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({});
        generate_round_sequence(state)?;
        state.message = "Merke die Sequenz!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<u32, GameError> {
        let sequence = sequence(state)?;
        let position = position(state)?;
        let Some(target) = sequence.get(position) else {
            return wrong_target(state);
        };
        if !matches_target(event, target) {
            return wrong_target(state);
        }

        let next_position = position.saturating_add(1);
        state.mode_state["position"] = Value::from(next_position);
        if next_position >= sequence.len() {
            let points = u32::try_from(25_usize.saturating_mul(sequence.len()))
                .map_err(|_| invalid_state())?;
            let player = state
                .players
                .get_mut(state.current_player_index)
                .ok_or(GameError::NoPlayers)?;
            player.score = player.score.saturating_add(points);
            state.mode_state["position"] = Value::from(0);
            state.message = format!("Sequenz geschafft +{points}");
            finish_action_round_game(state, "{winner} gewinnt Simon Says!")?;
            if state.status == GameStatus::Running {
                state.status = GameStatus::Hold;
            }
            return Ok(points);
        }

        let zone_count = zone_count(state)?;
        state.message = format!(
            "Weiter: {}",
            target_label(&sequence[next_position], zone_count)
        );
        Ok(0)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Ok(sequence) = sequence(state) else {
            return json!({"prompt": "Simon Says", "targets": []});
        };
        let position = position(state).unwrap_or_default();
        let zone_count = zone_count(state).unwrap_or(5);
        let mut targets = Vec::new();
        for (step, target) in sequence.iter().enumerate() {
            let label_field = target.fields.get(target.fields.len() / 2).copied();
            for field in &target.fields {
                for ring in NUMBER_RINGS {
                    let ring_name = ring_name(ring);
                    targets.push(json!({
                        "id": format!("simon-{}-{ring_name}-{field}", step + 1),
                        "field": field,
                        "ring": ring,
                        "color": if step == position { "cyan" } else { "green" },
                        "label": if Some(*field) == label_field && ring == Ring::SingleOuter { (step + 1).to_string() } else { String::new() },
                        "pulse": step == position,
                    }));
                }
            }
        }
        json!({
            "prompt": sequence.iter().map(|target| target_label(target, zone_count)).collect::<Vec<_>>().join(" → "),
            "targets": targets,
            "bonus": [
                {"id":"simon-joker-sbull","field":25,"ring":"single_bull","color":"gold","label":"JOKER","pulse":true},
                {"id":"simon-joker-dbull","field":25,"ring":"double_bull","color":"gold","label":"","pulse":true}
            ],
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let sequence_round = state
            .mode_state
            .get("sequence_round")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or_default();
        if sequence_round == state.round_number {
            state.mode_state["position"] = Value::from(0);
            Ok(())
        } else {
            generate_round_sequence(state)
        }
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state["position"] = Value::from(0);
        state.message = "Sequenz übersprungen".into();
        finish_action_round_game(state, "{winner} gewinnt Simon Says!")
    }
}

fn wrong_target(state: &mut RegisteredGameState) -> Result<u32, GameError> {
    state.mode_state["position"] = Value::from(0);
    state.message = "Falsches Feld – Sequenz reset".into();
    finish_action_round_game(state, "{winner} gewinnt Simon Says!")?;
    if state.status == GameStatus::Running {
        state.status = GameStatus::Hold;
    }
    Ok(0)
}

fn generate_round_sequence(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let length = usize::from(state.round_number.min(3));
    let count = difficulty_zone_count(state)?;
    let fields_per_zone = BOARD_ORDER.len() / count;
    let mut available = (0..count)
        .map(|index| Zone {
            zone: index + 1,
            fields: BOARD_ORDER[index * fields_per_zone..(index + 1) * fields_per_zone].to_vec(),
        })
        .collect::<Vec<_>>();
    let mut selected = Vec::with_capacity(length);
    for _ in 0..length {
        let index = state.random_index(available.len())?;
        selected.push(available.remove(index));
    }
    state.mode_state["sequence"] = serde_json::to_value(selected)
        .map_err(|error| GameError::RulesetUnavailable(error.to_string()))?;
    state.mode_state["zone_count"] = Value::from(count);
    state.mode_state["sequence_round"] = Value::from(state.round_number);
    state.mode_state["position"] = Value::from(0);
    Ok(())
}

fn difficulty_zone_count(state: &RegisteredGameState) -> Result<usize, GameError> {
    match state.options.get("difficulty").and_then(Value::as_str) {
        Some("very_easy") => Ok(4),
        Some("easy") => Ok(5),
        Some("normal") => Ok(10),
        Some("hard") => Ok(20),
        _ => Err(invalid_state()),
    }
}

fn sequence(state: &RegisteredGameState) -> Result<Vec<Zone>, GameError> {
    serde_json::from_value(
        state
            .mode_state
            .get("sequence")
            .cloned()
            .ok_or_else(invalid_state)?,
    )
    .map_err(|_| invalid_state())
}

fn position(state: &RegisteredGameState) -> Result<usize, GameError> {
    state
        .mode_state
        .get("position")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn zone_count(state: &RegisteredGameState) -> Result<usize, GameError> {
    state
        .mode_state
        .get("zone_count")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn matches_target(event: &DartEvent, target: &Zone) -> bool {
    match event {
        DartEvent::Hit {
            field: 25, ring, ..
        } => {
            matches!(ring, Ring::SingleBull | Ring::DoubleBull)
        }
        DartEvent::Hit { field, .. } => target.fields.contains(field),
        DartEvent::Miss { .. } => false,
    }
}

fn target_label(target: &Zone, zone_count: usize) -> String {
    if zone_count == 20 {
        target
            .fields
            .first()
            .map_or_else(|| "Z0".into(), ToString::to_string)
    } else {
        format!("Z{}", target.zone)
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

const fn choice_text(
    value: &'static str,
    label: &'static str,
    description: &'static str,
    description_en: &'static str,
) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Text(value),
        label,
        description: Some(description),
        description_en: Some(description_en),
    }
}

fn ring_name(ring: Ring) -> &'static str {
    match ring {
        Ring::SingleInner => "single_inner",
        Ring::Triple => "triple",
        Ring::SingleOuter => "single_outer",
        Ring::Double => "double",
        Ring::SingleBull => "single_bull",
        Ring::DoubleBull => "double_bull",
    }
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid simon_says mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    fn hit(field: u8, seq: u64) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring: Ring::SingleOuter,
            multiplier: 1,
            label: format!("S{field}"),
            score: u16::from(field),
        }
    }

    #[test]
    fn every_player_gets_the_same_sequence_and_the_next_round_grows() {
        let mut game = RegisteredGame::new_seeded(
            "simon_says",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"rounds": 3, "difficulty": "easy"}),
            42,
        )
        .expect("game");
        let round_one = sequence(game.state()).expect("round-one sequence");
        let field = round_one[0].fields[0];

        game.apply_throw(&hit(field, 1)).expect("Ada succeeds");
        assert_eq!(game.state().status, GameStatus::Hold);
        game.continue_turn().expect("Bob starts");
        assert_eq!(
            sequence(game.state()).expect("same sequence")[0].fields,
            round_one[0].fields
        );

        game.apply_throw(&hit(field, 2)).expect("Bob succeeds");
        game.continue_turn().expect("round two starts");
        assert_eq!(game.state().round_number, 2);
        assert_eq!(sequence(game.state()).expect("round-two sequence").len(), 2);
        assert_eq!(game.state().random_cursor, 3);
    }

    #[test]
    fn wrong_field_ends_the_visit_after_one_dart() {
        let mut game = RegisteredGame::new_seeded(
            "simon_says",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 3, "difficulty": "hard"}),
            42,
        )
        .expect("game");
        let target = sequence(game.state()).expect("sequence")[0].fields[0];
        let wrong = if target == 20 { 1 } else { 20 };

        game.apply_throw(&hit(wrong, 1)).expect("wrong field");

        assert_eq!(game.state().status, GameStatus::Hold);
        assert_eq!(game.state().darts_in_turn, 1);
        assert_eq!(game.state().mode_state["position"], 0);
        assert_eq!(game.state().players[0].score, 0);
    }

    #[test]
    fn bull_completes_the_current_step_as_a_joker() {
        let mut game = RegisteredGame::new_seeded(
            "simon_says",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 3, "difficulty": "easy"}),
            42,
        )
        .expect("game");

        game.apply_throw(&DartEvent::Hit {
            seq: 1,
            field: 25,
            ring: Ring::DoubleBull,
            multiplier: 2,
            label: "DBULL".into(),
            score: 50,
        })
        .expect("joker");

        assert_eq!(game.state().players[0].score, 25);
        assert_eq!(game.state().status, GameStatus::Hold);
    }
}
