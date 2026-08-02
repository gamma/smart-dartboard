use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{
        Target, parse_target, ring_name, same_field, same_target, target_pool, target_value,
        zone_id,
    },
};
use crate::{GameError, GameStatus};
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Value, json};

static HOLE_CHOICES: [GameOptionChoice; 2] =
    [choice_integer(6, "6 Löcher"), choice_integer(9, "9 Löcher")];

static DIFFICULTY_CHOICES: [GameOptionChoice; 3] = [
    choice_text(
        "easy",
        "Easy · Zahl genügt",
        "Jeder Ring der Zielzahl trifft das Loch.",
        "Any ring of the target number hits the hole.",
    ),
    choice_text(
        "normal",
        "Normal · Single/Double exakt",
        "Das angezeigte äußere Single- oder Double-Segment muss exakt getroffen werden.",
        "Hit the displayed outer Single or Double segment exactly.",
    ),
    choice_text(
        "hard",
        "Hard · Double/Triple/Bull",
        "Das angezeigte Double-, Triple- oder Bull-Segment muss exakt getroffen werden.",
        "Hit the displayed Double, Triple, or Bull segment exactly.",
    ),
];

static OPTIONS: [GameOption; 2] = [
    GameOption {
        key: "holes",
        label: "Löcher",
        kind: "choice",
        default: GameOptionValue::Integer(9),
        choices: &HOLE_CHOICES,
    },
    GameOption {
        key: "difficulty",
        label: "Platz",
        kind: "choice",
        default: GameOptionValue::Text("normal"),
        choices: &DIFFICULTY_CHOICES,
    },
];

static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Gleiches Loch",
        body: "Jeder Spieler wirft auf dasselbe Ziel.",
        icon: "flag",
    },
    GameInstruction {
        title: "Wenige Schläge",
        body: "Treffer mit Dart 1, 2 oder 3 zählt entsprechend viele Schläge.",
        icon: "golf",
    },
    GameInstruction {
        title: "Niedrig gewinnt",
        body: "Kein Treffer zählt vier Schläge. Nach dem letzten Loch gewinnt der niedrigste Score.",
        icon: "trophy",
    },
];

static METADATA: GameMetadata = GameMetadata {
    slug: "mini_golf",
    ruleset_version: 2,
    format: sdb_contracts::GameFormat::Individual,
    title: "Mini Golf Darts",
    tagline: "Neun Löcher auf der Scheibe",
    description: "Alle spielen dasselbe Loch. Je früher du das Ziel triffst, desto weniger Schläge sammelst du.",
    accent: "#74a57f",
    accent_secondary: "#f2cc8f",
    visual: "mini-golf",
    icon: "flag",
    artwork: "/static/assets/modes/mini_golf.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct MiniGolfMode;
pub(super) static MINI_GOLF_MODE: MiniGolfMode = MiniGolfMode;

impl GameMode for MiniGolfMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let holes = holes(state)?;
        state.options["rounds"] = Value::from(holes);
        state.mode_state = json!({"hole": 1, "used": []});
        new_hole(state)
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        let target = target(state)?;
        let hit = matches!(event, DartEvent::Hit { .. })
            && if difficulty(state)? == "easy" {
                same_field(event, &target)
            } else {
                same_target(event, &target)
            };
        let player_index = state.current_player_index;
        let (strokes, ends_turn) = if hit {
            let strokes = i64::from(state.darts_in_turn.saturating_add(1));
            state.message = format!(
                "{}! {strokes} Schlag",
                match strokes {
                    1 => "BIRDIE",
                    2 => "PAR",
                    _ => "BOGEY",
                }
            );
            (strokes, true)
        } else if state.darts_in_turn.saturating_add(1) >= 3 {
            state.message = "DOUBLE BOGEY · 4 Schläge".into();
            (4, true)
        } else {
            state.message = "Am Loch vorbei".into();
            (0, false)
        };
        state.players[player_index].score =
            state.players[player_index].score.saturating_add(strokes);
        if ends_turn {
            finish_if_final(state)?;
        }
        if hit && state.status == GameStatus::Running {
            state.status = GameStatus::Hold;
        }
        Ok(strokes)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Ok(target) = target(state) else {
            return json!({"prompt": "Nächstes Loch", "targets": []});
        };
        let easy = difficulty(state).unwrap_or("normal") == "easy";
        let targets = if easy && target.field != 25 {
            [Ring::SingleInner, Ring::Triple, Ring::SingleOuter, Ring::Double].into_iter().map(|ring| json!({
                "id": format!("golf-{}-{}", target.field, ring_name(ring)), "field": target.field, "ring": ring, "color": "green", "label": "⚑", "pulse": true
            })).collect::<Vec<_>>()
        } else {
            vec![
                json!({"id": zone_id(&target), "field": target.field, "ring": target.ring, "color": "green", "label": "⚑", "pulse": true}),
            ]
        };
        let mut standings = state.players.iter().collect::<Vec<_>>();
        standings.sort_by_key(|player| player.score);
        json!({
            "prompt": if easy { format!("Loch {}: Zahl {}", state.round_number, target.field) } else { format!("Loch {}: {}", state.round_number, target.label) },
            "targets": targets,
            "panel": {
                "title": format!("LOCH {}/{}", state.round_number, holes(state).unwrap_or(9)),
                "headline": target.label,
                "subline": "Birdie 1 · Par 2 · Bogey 3 · vorbei 4",
                "rows": standings.into_iter().map(|player| json!({"label": player.name, "value": format!("{} Schläge", player.score)})).collect::<Vec<_>>(),
            },
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let current_hole = state
            .mode_state
            .get("hole")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or_default();
        if current_hole != state.round_number {
            new_hole(state)?;
        }
        Ok(())
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let player = state
            .players
            .get_mut(state.current_player_index)
            .ok_or(GameError::NoPlayers)?;
        player.score = player.score.saturating_add(4);
        state.message = format!("{} überspringt · 4 Schläge", player.name);
        finish_if_final(state)
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

fn holes(state: &RegisteredGameState) -> Result<u16, GameError> {
    state
        .options
        .get("holes")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn difficulty(state: &RegisteredGameState) -> Result<&str, GameError> {
    state
        .options
        .get("difficulty")
        .and_then(Value::as_str)
        .ok_or_else(invalid_state)
}

fn target(state: &RegisteredGameState) -> Result<Target, GameError> {
    state
        .mode_state
        .get("target")
        .ok_or_else(invalid_state)
        .and_then(parse_target)
}

fn pool(state: &RegisteredGameState) -> Result<Vec<Target>, GameError> {
    Ok(match difficulty(state)? {
        "easy" => target_pool("easy"),
        "hard" => target_pool("hard"),
        _ => target_pool("normal")
            .into_iter()
            .filter(|target| matches!(target.ring, Ring::SingleOuter | Ring::Double))
            .collect(),
    })
}

fn new_hole(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let used = state
        .mode_state
        .get("used")
        .and_then(Value::as_array)
        .ok_or_else(invalid_state)?
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let full_pool = pool(state)?;
    let mut available = full_pool
        .iter()
        .filter(|target| !used.contains(&zone_id(target).as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if available.is_empty() {
        state.mode_state["used"] = Value::Array(Vec::new());
        available = full_pool;
    }
    let index = state.random_index(available.len())?;
    let selected = available.remove(index);
    state.mode_state["target"] = target_value(&selected);
    state.mode_state["used"]
        .as_array_mut()
        .ok_or_else(invalid_state)?
        .push(Value::from(zone_id(&selected)));
    state.mode_state["hole"] = Value::from(state.round_number);
    state.message = format!("Loch {}: {}", state.round_number, selected.label);
    Ok(())
}

fn finish_if_final(state: &mut RegisteredGameState) -> Result<(), GameError> {
    if state.current_player_index.saturating_add(1) != state.players.len()
        || state.round_number < holes(state)?
    {
        return Ok(());
    }
    let low = state
        .players
        .iter()
        .map(|player| player.score)
        .min()
        .ok_or(GameError::NoPlayers)?;
    let leaders = state
        .players
        .iter()
        .filter(|player| player.score == low)
        .collect::<Vec<_>>();
    state.status = GameStatus::Finished;
    if leaders.len() == 1 {
        let winner = leaders[0];
        state.winner_id = Some(winner.id.clone());
        state.winner_ids = vec![winner.id.clone()];
        state.result_type = "individual_win".into();
        state.message = format!("{} gewinnt den Platz mit {low} Schlägen!", winner.name);
    } else {
        state.winner_id = None;
        state.winner_ids.clear();
        state.result_type = "draw".into();
        state.message = format!(
            "Unentschieden: {}",
            leaders
                .iter()
                .map(|player| player.name.as_str())
                .collect::<Vec<_>>()
                .join(" · ")
        );
    }
    Ok(())
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid mini_golf mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    #[test]
    fn early_hit_scores_strokes_and_holds_immediately() {
        let mut game = RegisteredGame::new_seeded(
            "mini_golf",
            vec![("ada".into(), "Ada".into())],
            &json!({"holes": 6, "difficulty": "normal"}),
            42,
        )
        .expect("game");
        let target = target(game.state()).expect("target");
        game.apply_throw(&DartEvent::Hit {
            seq: 1,
            field: target.field,
            ring: target.ring,
            multiplier: target.multiplier,
            label: target.label,
            score: target.score,
        })
        .expect("hit");
        assert_eq!(game.state.players[0].score, 1);
        assert_eq!(game.state.status, GameStatus::Hold);
        assert_eq!(game.state.darts_in_turn, 1);
    }

    #[test]
    fn skipped_final_hole_awards_four_and_low_score_wins() {
        let mut game = RegisteredGame::new_seeded(
            "mini_golf",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"holes": 6, "difficulty": "easy"}),
            42,
        )
        .expect("game");
        for hole in 1..=6 {
            let target = target(game.state()).expect("target");
            game.apply_throw(&DartEvent::Hit {
                seq: hole,
                field: target.field,
                ring: target.ring,
                multiplier: target.multiplier,
                label: target.label,
                score: target.score,
            })
            .expect("Ada holes out");
            game.continue_turn().expect("Bob turn");
            if hole < 6 {
                game.next_player().expect("Bob skips hole");
            }
        }

        game.next_player().expect("skip final hole");
        assert_eq!(game.state.players[0].score, 6);
        assert_eq!(game.state.players[1].score, 24);
        assert_eq!(game.state.winner_id.as_deref(), Some("ada"));

        game.undo().expect("undo skip");
        assert_eq!(game.state.status, GameStatus::Running);
        assert_eq!(game.state.round_number, 6);
        assert_eq!(game.state.current_player_index, 1);
        assert_eq!(game.state.players[1].score, 20);
    }
}
