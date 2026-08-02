use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
};
use crate::{GameError, GameStatus};
use sdb_contracts::DartEvent;
use serde_json::{Value, json};

#[derive(Clone, Copy)]
struct Task {
    id: &'static str,
    label: &'static str,
}

static TASKS: [Task; 16] = [
    task("double", "Any Double"),
    task("triple", "Any Triple"),
    task("bull", "Bull"),
    task("even", "Even"),
    task("odd", "Odd"),
    task("high", "16+"),
    task("low", "1-5"),
    task("field_20", "Any 20"),
    task("field_19", "Any 19"),
    task("field_18", "Any 18"),
    task("field_17", "Any 17"),
    task("field_16", "Any 16"),
    task("field_15", "Any 15"),
    task("field_10", "Any 10"),
    task("field_5", "Any 5"),
    task("field_1", "Any 1"),
];

static WIN_CHOICES: [GameOptionChoice; 2] = [
    GameOptionChoice {
        value: GameOptionValue::Text("line"),
        label: "Erste Linie",
        description: Some("Drei erledigte Aufgaben waagerecht, senkrecht oder diagonal gewinnen."),
        description_en: Some("Three completed tasks horizontally, vertically, or diagonally win."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("full"),
        label: "Volle Karte",
        description: Some("Alle neun Aufgaben müssen erfüllt werden."),
        description_en: Some("All nine tasks must be completed."),
    },
];
static OPTIONS: [GameOption; 1] = [GameOption {
    key: "points",
    label: "Sieg",
    kind: "choice",
    default: GameOptionValue::Text("line"),
    choices: &WIN_CHOICES,
}];
static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Karte füllen",
        body: "Jeder Treffer kann eine oder mehrere passende Aufgaben markieren.",
        icon: "grid",
    },
    GameInstruction {
        title: "Siegziel beachten",
        body: "Je nach Auswahl zählt die erste Linie oder die volle Karte.",
        icon: "line",
    },
    GameInstruction {
        title: "Gleiche Chancen",
        body: "Alle spielen dieselbe Karte. Nach dem ersten Bingo läuft die Teamrunde zu Ende.",
        icon: "cards",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "darts_bingo",
    ruleset_version: 2,
    format: sdb_contracts::GameFormat::Individual,
    title: "Darts Bingo",
    tagline: "Aufgaben markieren, Linie holen",
    description: "Alle spielen dieselbe 3×3-Karte aus Dartaufgaben. Eine Linie oder die volle Karte gewinnt nach einer fairen Ausgleichsrunde.",
    accent: "#ffcf33",
    accent_secondary: "#9b5cff",
    visual: "darts-bingo",
    icon: "grid",
    artwork: "/static/assets/modes/darts_bingo.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct DartsBingoMode;
pub(super) static DARTS_BINGO_MODE: DartsBingoMode = DartsBingoMode;

impl GameMode for DartsBingoMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let mut available = TASKS.to_vec();
        let mut selected = Vec::with_capacity(9);
        for _ in 0..9 {
            let index = state.random_index(available.len())?;
            selected.push(available.remove(index));
        }
        for player in &mut state.players {
            player.marks.clear();
        }
        state.mode_state = json!({
            "tasks": selected.iter().map(|task| json!({"id":task.id,"label":task.label})).collect::<Vec<_>>(),
            "bingo_candidates": [],
            "last_effect": "",
            "marked_count": 0,
            "effect_player_id": null,
        });
        state.message = "Für alle liegt dieselbe Bingo-Karte bereit!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        clear_effect(state);
        let tasks = selected_tasks(state)?;
        let player_index = state.current_player_index;
        let mut marked = Vec::new();
        if matches!(event, DartEvent::Hit { .. }) {
            let player = state
                .players
                .get_mut(player_index)
                .ok_or(GameError::NoPlayers)?;
            for (index, task) in tasks.iter().enumerate() {
                let key = index.to_string();
                if player.marks.get(&key).copied().unwrap_or_default() == 0
                    && accepts(task.id, event)
                {
                    player.marks.insert(key, 1);
                    marked.push(task.label);
                }
            }
            let points = i64::try_from(marked.len()).map_err(|_| invalid_state())?;
            player.score = player.score.saturating_add(points);
        }

        let points = i64::try_from(marked.len()).map_err(|_| invalid_state())?;
        if marked.is_empty() {
            state.message = if matches!(event, DartEvent::Miss { .. }) {
                "Miss – kein Bingo".into()
            } else {
                "Keine Bingo-Aufgabe getroffen".into()
            };
        } else {
            let player_id = state
                .players
                .get(player_index)
                .ok_or(GameError::NoPlayers)?
                .id
                .clone();
            set_mark_effect(state, points, &player_id);
            if target_reached(state, player_index)? {
                add_candidate(state, &player_id)?;
                if finish_candidates(state, true)? {
                    return Ok(points);
                }
                state.status = GameStatus::Hold;
                state.message = format!(
                    "{} hat BINGO · Ausgleichsrunde läuft",
                    player_name(state, &player_id)?
                );
                return Ok(points);
            }
            state.message = format!("Bingo markiert: {}", marked.join(" · "));
        }
        finish_candidates(state, false)?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        overlay(state).unwrap_or_else(|_| json!({"prompt":"Darts Bingo","card":[]}))
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        if !finish_candidates(state, true)? {
            state.message = "Aufnahme übersprungen".into();
        }
        Ok(())
    }
}

fn selected_tasks(state: &RegisteredGameState) -> Result<Vec<Task>, GameError> {
    state.mode_state["tasks"]
        .as_array()
        .ok_or_else(invalid_state)?
        .iter()
        .map(|value| {
            let id = value
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(invalid_state)?;
            TASKS
                .iter()
                .copied()
                .find(|task| task.id == id)
                .ok_or_else(invalid_state)
        })
        .collect()
}

fn accepts(task_id: &str, event: &DartEvent) -> bool {
    let DartEvent::Hit {
        field, multiplier, ..
    } = event
    else {
        return false;
    };
    match task_id {
        "double" => *multiplier == 2,
        "triple" => *multiplier == 3,
        "bull" => *field == 25,
        "even" => *field > 0 && *field % 2 == 0,
        "odd" => *field > 0 && *field < 25 && *field % 2 == 1,
        "high" => (16..=20).contains(field),
        "low" => (1..=5).contains(field),
        _ => task_id
            .strip_prefix("field_")
            .and_then(|value| value.parse::<u8>().ok())
            .is_some_and(|expected| expected == *field),
    }
}

fn target_reached(state: &RegisteredGameState, player_index: usize) -> Result<bool, GameError> {
    let player = state
        .players
        .get(player_index)
        .ok_or(GameError::NoPlayers)?;
    let done = (0..9)
        .map(|index| {
            player
                .marks
                .get(&index.to_string())
                .copied()
                .unwrap_or_default()
                > 0
        })
        .collect::<Vec<_>>();
    match state.options.get("points").and_then(Value::as_str) {
        Some("full") => Ok(done.iter().all(|value| *value)),
        Some("line") => Ok([
            [0, 1, 2],
            [3, 4, 5],
            [6, 7, 8],
            [0, 3, 6],
            [1, 4, 7],
            [2, 5, 8],
            [0, 4, 8],
            [2, 4, 6],
        ]
        .iter()
        .any(|line| line.iter().all(|index| done[*index]))),
        _ => Err(invalid_state()),
    }
}

fn add_candidate(state: &mut RegisteredGameState, player_id: &str) -> Result<(), GameError> {
    let candidates = state.mode_state["bingo_candidates"]
        .as_array_mut()
        .ok_or_else(invalid_state)?;
    if !candidates.iter().any(|value| value == player_id) {
        candidates.push(Value::from(player_id));
    }
    Ok(())
}

fn candidates(state: &RegisteredGameState) -> Result<Vec<String>, GameError> {
    serde_json::from_value(state.mode_state["bingo_candidates"].clone())
        .map_err(|_| invalid_state())
}

fn finish_candidates(
    state: &mut RegisteredGameState,
    force_turn_end: bool,
) -> Result<bool, GameError> {
    let candidates = candidates(state)?;
    let is_last_player = state.current_player_index.saturating_add(1) == state.players.len();
    let is_turn_end = force_turn_end || state.darts_in_turn >= 2;
    if candidates.is_empty() || !is_last_player || !is_turn_end {
        return Ok(false);
    }
    let names = candidates
        .iter()
        .map(|player_id| player_name(state, player_id).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    state.status = GameStatus::Finished;
    if candidates.len() == 1 {
        state.winner_id = Some(candidates[0].clone());
        state.winner_ids.clone_from(&candidates);
        state.result_type = "individual_win".into();
        state.message = format!("{} ruft BINGO!", names[0]);
        state.mode_state["last_effect"] = Value::from("bingo_win");
        state.mode_state["effect_player_id"] = Value::from(candidates[0].clone());
    } else {
        state.winner_id = None;
        state.winner_ids.clear();
        state.result_type = "draw".into();
        state.message = format!("Gleichzeitiges BINGO: {}", names.join(" · "));
        state.mode_state["last_effect"] = Value::from("bingo_draw");
        state.mode_state["effect_player_id"] = Value::Null;
    }
    Ok(true)
}

fn player_name<'a>(state: &'a RegisteredGameState, player_id: &str) -> Result<&'a str, GameError> {
    state
        .players
        .iter()
        .find(|player| player.id == player_id)
        .map(|player| player.name.as_str())
        .ok_or_else(invalid_state)
}

fn overlay(state: &RegisteredGameState) -> Result<Value, GameError> {
    let tasks = selected_tasks(state)?;
    let player = state
        .players
        .get(state.current_player_index)
        .ok_or(GameError::NoPlayers)?;
    let card = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| {
            json!({
                "index": index,
                "label": task.label,
                "done": player.marks.get(&index.to_string()).copied().unwrap_or_default() > 0,
            })
        })
        .collect::<Vec<_>>();
    let remaining = card
        .iter()
        .filter(|cell| cell["done"] == false)
        .filter_map(|cell| cell["label"].as_str())
        .take(4)
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(json!({"prompt":format!("Bingo: {remaining}"),"card":card}))
}

fn clear_effect(state: &mut RegisteredGameState) {
    state.mode_state["last_effect"] = Value::from("");
    state.mode_state["marked_count"] = Value::from(0);
    state.mode_state["effect_player_id"] = Value::Null;
}

fn set_mark_effect(state: &mut RegisteredGameState, count: i64, player_id: &str) {
    state.mode_state["last_effect"] = Value::from("bingo_mark");
    state.mode_state["marked_count"] = Value::from(count);
    state.mode_state["effect_player_id"] = Value::from(player_id);
}

const fn task(id: &'static str, label: &'static str) -> Task {
    Task { id, label }
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid darts bingo state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;
    use sdb_contracts::Ring;

    fn game_with_goal(players: &[&str], goal: &str) -> RegisteredGame {
        RegisteredGame::new_seeded(
            "darts_bingo",
            players
                .iter()
                .map(|id| ((*id).into(), id.to_uppercase()))
                .collect(),
            &json!({"points":goal}),
            42,
        )
        .expect("game")
    }

    fn game(players: &[&str]) -> RegisteredGame {
        game_with_goal(players, "line")
    }

    fn hit(field: u8, ring: Ring, multiplier: u8, seq: u64) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring,
            multiplier,
            label: format!("{field}"),
            score: u16::from(field) * u16::from(multiplier),
        }
    }

    #[test]
    fn seeded_card_is_shared_and_marks_every_matching_task() {
        let mut game = game(&["ada", "bob"]);
        assert_eq!(game.state.random_cursor, 9);
        game.state.mode_state["tasks"] = json!([
            {"id":"double","label":"Any Double"},
            {"id":"even","label":"Even"},
            {"id":"field_20","label":"Any 20"},
            {"id":"field_19","label":"Any 19"},
            {"id":"field_18","label":"Any 18"},
            {"id":"field_17","label":"Any 17"},
            {"id":"field_16","label":"Any 16"},
            {"id":"field_15","label":"Any 15"},
            {"id":"field_10","label":"Any 10"}
        ]);
        game.initial_state = Some(game.state.clone());
        game.apply_throw(&hit(20, Ring::Double, 2, 1)).expect("hit");
        assert_eq!(game.state.players[0].score, 3);
        assert_eq!(game.state.mode_state["marked_count"], 3);
        assert!(game.state.players[1].marks.is_empty());
    }

    #[test]
    fn equalization_draw_has_no_session_winners() {
        let mut game = game(&["ada", "bob"]);
        game.state.mode_state["bingo_candidates"] = json!(["ada", "bob"]);
        game.state.current_player_index = 1;
        game.state.darts_in_turn = 2;
        game.apply_throw(&DartEvent::Miss {
            seq: 1,
            label: "MISS".into(),
            score: 0,
        })
        .expect("equalizer");
        assert_eq!(game.state.status, GameStatus::Finished);
        assert_eq!(game.state.result_type, "draw");
        assert!(game.state.winner_ids.is_empty());
    }

    #[test]
    fn final_player_skip_finishes_an_existing_candidate() {
        let mut game = game(&["ada", "bob"]);
        game.state.mode_state["bingo_candidates"] = json!(["ada"]);
        game.state.current_player_index = 1;
        game.next_player().expect("skip");
        assert_eq!(game.state.status, GameStatus::Finished);
        assert_eq!(game.state.winner_ids, ["ada"]);
    }

    #[test]
    fn full_card_does_not_finish_on_a_completed_line() {
        let mut game = game_with_goal(&["ada"], "full");
        game.state.mode_state["tasks"] = json!([
            {"id":"high","label":"16+"},
            {"id":"triple","label":"Any Triple"},
            {"id":"double","label":"Any Double"},
            {"id":"low","label":"1-5"},
            {"id":"field_5","label":"Any 5"},
            {"id":"field_10","label":"Any 10"},
            {"id":"field_18","label":"Any 18"},
            {"id":"field_17","label":"Any 17"},
            {"id":"field_16","label":"Any 16"}
        ]);
        game.state.players[0].marks.insert("0".into(), 1);
        game.state.players[0].marks.insert("1".into(), 1);
        game.apply_throw(&hit(20, Ring::Double, 2, 1))
            .expect("third cell");
        assert_eq!(game.state.status, GameStatus::Running);
        assert_eq!(game.state.mode_state["bingo_candidates"], json!([]));
    }
}
