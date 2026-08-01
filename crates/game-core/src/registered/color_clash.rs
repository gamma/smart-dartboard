use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{Target, physical_target_pool, ring_name, zone_id},
    finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::DartEvent;
use serde_json::{Map, Value, json};

static ROUND_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(3, "3 Runden"),
    choice_integer(5, "5 Runden"),
    choice_integer(8, "8 Runden"),
];
static SHUFFLE_CHOICES: [GameOptionChoice; 2] = [
    GameOptionChoice {
        value: GameOptionValue::Text("turn"),
        label: "Nach jeder Runde",
        description: Some("Die Farbverteilung bleibt für alle Spieler der Runde identisch."),
        description_en: Some("The color layout stays identical for every player in the round."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("dart"),
        label: "Nach jedem Dart · gleich für alle",
        description: Some("Alle erhalten dieselbe vorbereitete Folge aus drei Farbverteilungen."),
        description_en: Some(
            "Everyone receives the same prepared sequence of three color layouts.",
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
        key: "shuffle",
        label: "Farbwechsel",
        kind: "choice",
        default: GameOptionValue::Text("turn"),
        choices: &SHUFFLE_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Farben zählen",
        body: "Gold +50, Cyan +25, Grün +10, Rot -25.",
        icon: "palette",
    },
    GameInstruction {
        title: "Klassische Punkte egal",
        body: "Die Farbe des getroffenen Segments entscheidet.",
        icon: "rules",
    },
    GameInstruction {
        title: "Gleiche Chancen",
        body: "Alle spielen pro Runde dieselben Farben – fest oder als gleiche Drei-Dart-Folge.",
        icon: "shuffle",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "color_clash",
    ruleset_version: 3,
    title: "Color Clash",
    tagline: "Gold zählt, Rot tut weh",
    description: "Das Board wird zur Arcade-Fläche: Farben bestimmen die Punkte, nicht der klassische Dartwert.",
    accent: "#ffcf33",
    accent_secondary: "#28e7ff",
    visual: "color-clash",
    icon: "palette",
    artwork: "/static/assets/modes/color_clash.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct ColorClashMode;
pub(super) static COLOR_CLASH_MODE: ColorClashMode = ColorClashMode;

impl GameMode for ColorClashMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({});
        generate_round_layouts(state)?;
        state.message = "Gold zählt am meisten!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        ensure_round_layouts(state)?;
        if shuffle(state)? == "dart" {
            select_layout(state, usize::from(state.darts_in_turn))?;
        }
        let points = if matches!(event, DartEvent::Miss { .. }) {
            state.message = "Miss".into();
            0
        } else {
            let color = color_for_event(state, event)?.to_owned();
            let points = color_score(&color);
            let player = state
                .players
                .get_mut(state.current_player_index)
                .ok_or(GameError::NoPlayers)?;
            player.score = player.score.saturating_add(points);
            state.message = if color.is_empty() {
                format!("{}: neutral", event.label())
            } else {
                format!("{}: {color} {points:+}", event.label())
            };
            points
        };
        if shuffle(state)? == "dart" && state.darts_in_turn.saturating_add(1) < 3 {
            select_layout(state, usize::from(state.darts_in_turn.saturating_add(1)))?;
        }
        finish_fixed_round_game(state, "{winner} gewinnt den Color Clash!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(colors) = state.mode_state["colors"].as_object() else {
            return json!({"prompt": "Color Clash", "bonus": [], "targets": [], "danger": []});
        };
        let mut bonus = Vec::new();
        let mut targets = Vec::new();
        let mut danger = Vec::new();
        for target in physical_target_pool() {
            let color = colors
                .get(&physical_id(&target))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let entry = json!({
                "id": zone_id(&target),
                "field": target.field,
                "ring": target.ring,
                "color": color,
                "label": color_label(color),
                "pulse": color == "red",
            });
            match color {
                "gold" => bonus.push(entry),
                "cyan" | "green" => targets.push(entry),
                "red" => danger.push(entry),
                _ => {}
            }
        }
        json!({
            "prompt": "Gold +50 · Cyan +25 · Grün +10 · Rot -25",
            "bonus": bonus,
            "targets": targets,
            "danger": danger,
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        ensure_round_layouts(state)?;
        select_layout(state, 0)
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} gewinnt den Color Clash!")
    }
}

fn generate_round_layouts(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let count = if shuffle(state)? == "dart" { 3 } else { 1 };
    let mut layouts = Vec::with_capacity(count);
    for _ in 0..count {
        layouts.push(Value::Object(generate_colors(state)?));
    }
    let colors = layouts.first().cloned().ok_or_else(invalid_state)?;
    state.mode_state = json!({
        "layout_round": state.round_number,
        "layouts": layouts,
        "layout_index": 0,
        "colors": colors,
    });
    Ok(())
}

fn generate_colors(state: &mut RegisteredGameState) -> Result<Map<String, Value>, GameError> {
    let mut distribution = Vec::with_capacity(82);
    distribution.extend(std::iter::repeat_n("gold", 8));
    distribution.extend(std::iter::repeat_n("cyan", 22));
    distribution.extend(std::iter::repeat_n("green", 36));
    distribution.extend(std::iter::repeat_n("red", 16));
    for index in (1..distribution.len()).rev() {
        let swap_index = state.random_index(index + 1)?;
        distribution.swap(index, swap_index);
    }
    Ok(physical_target_pool()
        .into_iter()
        .zip(distribution)
        .map(|(target, color)| (physical_id(&target), Value::from(color)))
        .collect())
}

fn ensure_round_layouts(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let expected = if shuffle(state)? == "dart" { 3 } else { 1 };
    let current_round = state.mode_state["layout_round"]
        .as_u64()
        .and_then(|value| u16::try_from(value).ok());
    let layout_count = state.mode_state["layouts"].as_array().map(Vec::len);
    if current_round != Some(state.round_number) || layout_count != Some(expected) {
        generate_round_layouts(state)?;
    }
    Ok(())
}

fn select_layout(state: &mut RegisteredGameState, index: usize) -> Result<(), GameError> {
    let layouts = state.mode_state["layouts"]
        .as_array()
        .ok_or_else(invalid_state)?;
    let selected = index.min(layouts.len().saturating_sub(1));
    let colors = layouts.get(selected).cloned().ok_or_else(invalid_state)?;
    state.mode_state["layout_index"] = Value::from(selected);
    state.mode_state["colors"] = colors;
    Ok(())
}

fn color_for_event<'a>(
    state: &'a RegisteredGameState,
    event: &DartEvent,
) -> Result<&'a str, GameError> {
    let DartEvent::Hit { field, ring, .. } = event else {
        return Ok("");
    };
    let id = format!("{}:{field}", ring_name(*ring));
    state.mode_state["colors"]
        .as_object()
        .ok_or_else(invalid_state)?
        .get(&id)
        .and_then(Value::as_str)
        .ok_or_else(invalid_state)
}

fn shuffle(state: &RegisteredGameState) -> Result<&str, GameError> {
    state.options["shuffle"].as_str().ok_or_else(invalid_state)
}

fn physical_id(target: &Target) -> String {
    format!("{}:{}", ring_name(target.ring), target.field)
}

fn color_score(color: &str) -> i64 {
    match color {
        "gold" => 50,
        "cyan" => 25,
        "green" => 10,
        "red" => -25,
        _ => 0,
    }
}

fn color_label(color: &str) -> &'static str {
    match color {
        "gold" => "+50",
        "cyan" => "+25",
        "green" => "+10",
        "red" => "-25",
        _ => "",
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

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid color_clash mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameStatus, RegisteredGame};
    use sdb_contracts::Ring;

    fn game(players: usize, shuffle: &str) -> RegisteredGame {
        RegisteredGame::new_seeded(
            "color_clash",
            (0..players)
                .map(|index| (format!("p{index}"), format!("Player {index}")))
                .collect(),
            &json!({"rounds": 3, "shuffle": shuffle}),
            42,
        )
        .expect("game")
    }

    fn miss(seq: u64) -> DartEvent {
        DartEvent::Miss {
            seq,
            label: "MISS".into(),
            score: 0,
        }
    }

    fn target_with_color(state: &RegisteredGameState, color: &str) -> Target {
        physical_target_pool()
            .into_iter()
            .find(|target| state.mode_state["colors"][physical_id(target)] == color)
            .expect("colored target")
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
    fn physical_pool_and_distribution_cover_every_segment() {
        assert_eq!(METADATA.ruleset_version, 3);
        let pool = physical_target_pool();
        assert_eq!(pool.len(), 82);
        assert_eq!(
            pool.last().map(|target| target.label.as_str()),
            Some("DBull")
        );
        assert!(pool.iter().any(|target| target.ring == Ring::SingleBull));
        let game = game(1, "turn");
        let colors = game.state().mode_state["colors"]
            .as_object()
            .expect("colors");
        assert_eq!(colors.len(), 82);
        for (color, expected) in [("gold", 8), ("cyan", 22), ("green", 36), ("red", 16)] {
            assert_eq!(
                colors.values().filter(|value| **value == color).count(),
                expected
            );
        }
        assert_eq!(game.state().random_cursor, 81);
    }

    #[test]
    fn red_scores_negative_and_undo_restores_the_score() {
        let mut game = game(1, "turn");
        let red = target_with_color(game.state(), "red");
        game.apply_throw(&hit(&red, 1)).expect("red");
        assert_eq!(game.state().players[0].score, -25);
        assert_eq!(game.state().turn_score, -25);
        game.undo().expect("undo");
        assert_eq!(game.state().players[0].score, 0);
    }

    #[test]
    fn dart_layout_sequence_repeats_for_each_player() {
        let mut game = game(2, "dart");
        let layouts = game.state().mode_state["layouts"].clone();
        assert_eq!(game.state().random_cursor, 243);
        game.apply_throw(&miss(1)).expect("dart one");
        assert_eq!(game.state().mode_state["layout_index"], 1);
        game.apply_throw(&miss(2)).expect("dart two");
        assert_eq!(game.state().mode_state["layout_index"], 2);
        game.apply_throw(&miss(3)).expect("dart three");
        game.continue_turn().expect("player two");
        assert_eq!(game.state().mode_state["layout_index"], 0);
        assert_eq!(game.state().mode_state["layouts"], layouts);
    }

    #[test]
    fn next_round_generates_one_new_shared_layout() {
        let mut game = game(2, "turn");
        let first = game.state().mode_state["colors"].clone();
        for seq in 1..=3 {
            game.apply_throw(&miss(seq)).expect("player one");
        }
        game.continue_turn().expect("player two");
        for seq in 4..=6 {
            game.apply_throw(&miss(seq)).expect("player two");
        }
        game.continue_turn().expect("round two");
        assert_eq!(game.state().round_number, 2);
        assert_ne!(game.state().mode_state["colors"], first);
        assert_eq!(game.state().random_cursor, 162);
    }

    #[test]
    fn skipping_the_final_round_finishes_the_game() {
        let mut game = game(1, "turn");
        game.next_player().expect("round one");
        game.next_player().expect("round two");
        game.next_player().expect("round three");
        assert_eq!(game.state().status, GameStatus::Finished);
    }
}
