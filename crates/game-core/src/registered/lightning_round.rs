use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState, finish_action_round_game,
};
use crate::{GameError, GameStatus};
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Value, json};

struct Task {
    id: &'static str,
    prompt: &'static str,
}

static TASKS: [Task; 6] = [
    Task {
        id: "any_double",
        prompt: "Triff ein Double",
    },
    Task {
        id: "any_triple",
        prompt: "Triff ein Triple",
    },
    Task {
        id: "over_15",
        prompt: "Triff eine Zahl über 15",
    },
    Task {
        id: "under_10",
        prompt: "Triff eine Zahl unter 10",
    },
    Task {
        id: "bull",
        prompt: "Triff Bull",
    },
    Task {
        id: "even",
        prompt: "Triff eine gerade Zahl",
    },
];

static ROUND_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(5, "5 Runden"),
    choice_integer(8, "8 Runden"),
    choice_integer(12, "12 Runden"),
];

static OPTIONS: [GameOption; 1] = [GameOption {
    key: "rounds",
    label: "Runden",
    kind: "choice",
    default: GameOptionValue::Integer(8),
    choices: &ROUND_CHOICES,
}];

static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Aufgabe lesen",
        body: "Der Projector zeigt die Challenge.",
        icon: "task",
    },
    GameInstruction {
        title: "Ein Dart",
        body: "Jeder Spieler hat genau einen Dart pro Aufgabe.",
        icon: "dart",
    },
    GameInstruction {
        title: "Erfolg punktet",
        body: "Erfolg gibt +25, Fehler gibt 0.",
        icon: "success",
    },
];

static METADATA: GameMetadata = GameMetadata {
    slug: "lightning_round",
    ruleset_version: 2,
    title: "Lightning Round",
    tagline: "Eine Aufgabe, ein Dart",
    description: "Schnelle Mini-Challenges: Löse die angezeigte Aufgabe mit deinem nächsten Dart.",
    accent: "#28e7ff",
    accent_secondary: "#ffcf33",
    visual: "lightning-round",
    icon: "bolt",
    artwork: "/static/assets/modes/lightning_round.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct LightningRoundMode;
pub(super) static LIGHTNING_ROUND_MODE: LightningRoundMode = LightningRoundMode;

impl GameMode for LightningRoundMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        select_initial_task(state)
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        let task = task(state)?;
        let success = matches!(event, DartEvent::Hit { .. }) && accepts(task.id, event);
        let points = if success { 25 } else { 0 };
        let player = state
            .players
            .get_mut(state.current_player_index)
            .ok_or(GameError::NoPlayers)?;
        player.score = player.score.saturating_add(points);
        state.message = if success {
            "SUCCESS +25".into()
        } else {
            "FAIL".into()
        };
        if state.current_player_index.saturating_add(1) == state.players.len() {
            select_next_task(state)?;
        }
        finish_action_round_game(state, "{winner} gewinnt Lightning!")?;
        if state.status == GameStatus::Running {
            state.status = GameStatus::Hold;
        }
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Ok(task) = task(state) else {
            return json!({"prompt": "Lightning Round", "targets": []});
        };
        json!({"prompt": task.prompt, "targets": overlay_targets(task.id)})
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let last_player = state.current_player_index.saturating_add(1) == state.players.len();
        let final_round = state.round_number >= rounds(state)?;
        if last_player && !final_round {
            select_next_task(state)?;
        }
        Ok(())
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} gewinnt Lightning!")
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

fn rounds(state: &RegisteredGameState) -> Result<u16, GameError> {
    state
        .options
        .get("rounds")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn task(state: &RegisteredGameState) -> Result<&'static Task, GameError> {
    let id = state
        .mode_state
        .get("task_id")
        .and_then(Value::as_str)
        .ok_or_else(invalid_state)?;
    TASKS
        .iter()
        .find(|task| task.id == id)
        .ok_or_else(invalid_state)
}

fn select_initial_task(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let index = state.random_index(TASKS.len())?;
    let selected = &TASKS[index];
    state.mode_state = json!({"task_id": selected.id});
    state.message = selected.prompt.into();
    Ok(())
}

fn select_next_task(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let current = task(state)?.id;
    let candidates = TASKS
        .iter()
        .filter(|task| task.id != current)
        .collect::<Vec<_>>();
    let index = state.random_index(candidates.len())?;
    state.mode_state["task_id"] = Value::from(candidates[index].id);
    state.message = candidates[index].prompt.into();
    Ok(())
}

fn accepts(task_id: &str, event: &DartEvent) -> bool {
    let DartEvent::Hit {
        field, multiplier, ..
    } = event
    else {
        return false;
    };
    match task_id {
        "any_double" => *multiplier == 2,
        "any_triple" => *multiplier == 3,
        "over_15" => (16..=20).contains(field),
        "under_10" => (1..10).contains(field),
        "bull" => *field == 25,
        "even" => (2..=20).contains(field) && field % 2 == 0,
        _ => false,
    }
}

fn overlay_targets(task_id: &str) -> Vec<Value> {
    match task_id {
        "any_double" => (1..=20)
            .map(|field| overlay(field, Ring::Double))
            .chain(std::iter::once(overlay(25, Ring::DoubleBull)))
            .collect(),
        "any_triple" => (1..=20).map(|field| overlay(field, Ring::Triple)).collect(),
        "over_15" => number_targets(16..=20),
        "under_10" => number_targets(1..=9),
        "bull" => vec![overlay(25, Ring::SingleBull), overlay(25, Ring::DoubleBull)],
        "even" => number_targets((2..=20).step_by(2)),
        _ => Vec::new(),
    }
}

fn number_targets(fields: impl IntoIterator<Item = u8>) -> Vec<Value> {
    fields
        .into_iter()
        .flat_map(|field| {
            [
                Ring::SingleInner,
                Ring::Triple,
                Ring::SingleOuter,
                Ring::Double,
            ]
            .map(|ring| overlay(field, ring))
        })
        .collect()
}

fn overlay(field: u8, ring: Ring) -> Value {
    json!({"id": format!("lightning-{field}-{}", super::arcade::ring_name(ring)), "field": field, "ring": ring, "color": "cyan", "label": "OK", "pulse": false})
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid lightning_round mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    #[test]
    fn one_dart_holds_and_the_task_stays_shared_for_the_round() {
        let mut game = RegisteredGame::new_seeded(
            "lightning_round",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"rounds": 5}),
            42,
        )
        .expect("game");
        let task = game.state.mode_state["task_id"].clone();
        game.apply_throw(&DartEvent::Miss {
            seq: 1,
            label: "MISS".into(),
            score: 0,
        })
        .expect("dart");
        assert_eq!(game.state.status, GameStatus::Hold);
        assert_eq!(game.state.darts_in_turn, 1);
        game.continue_turn().expect("Bob turn");
        assert_eq!(game.state.mode_state["task_id"], task);
        game.apply_throw(&DartEvent::Miss {
            seq: 2,
            label: "MISS".into(),
            score: 0,
        })
        .expect("dart");
        assert_ne!(game.state.mode_state["task_id"], task);
        assert_eq!(game.state.random_cursor, 2);
    }

    #[test]
    fn skipping_the_final_one_dart_round_finishes_and_is_undoable() {
        let mut game = RegisteredGame::new_seeded(
            "lightning_round",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 5}),
            42,
        )
        .expect("game");
        for _ in 1..5 {
            game.next_player().expect("skip round");
        }
        assert_eq!(game.state.round_number, 5);

        game.next_player().expect("finish final round");
        assert_eq!(game.state.status, GameStatus::Finished);
        assert_eq!(game.state.winner_id.as_deref(), Some("ada"));

        game.undo().expect("undo final skip");
        assert_eq!(game.state.status, GameStatus::Running);
        assert_eq!(game.state.round_number, 5);
    }
}
