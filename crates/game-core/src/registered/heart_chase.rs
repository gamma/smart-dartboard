use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
};
use crate::{GameError, GameStatus};
use sdb_contracts::DartEvent;
use serde_json::{Map, Value, json};

static HEART_CHOICES: [GameOptionChoice; 3] = [
    GameOptionChoice {
        value: GameOptionValue::Integer(2),
        label: "2 Herzen",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(3),
        label: "3 Herzen",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(5),
        label: "5 Herzen",
        description: None,
        description_en: None,
    },
];

static OPTIONS: [GameOption; 1] = [GameOption {
    key: "hearts",
    label: "Herzen",
    kind: "choice",
    default: GameOptionValue::Integer(3),
    choices: &HEART_CHOICES,
}];

static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Jagd eröffnen",
        body: "Der erste Spieler legt mit drei Darts die Jagdpunktzahl vor.",
        icon: "target",
    },
    GameInstruction {
        title: "Strikt übertreffen",
        body: "Gleichstand reicht nicht. Bei Misserfolg verlierst du ein Herz.",
        icon: "heart",
    },
    GameInstruction {
        title: "Letztes Herz gewinnt",
        body: "Ausgeschiedene Spieler werden automatisch übersprungen.",
        icon: "trophy",
    },
];

static METADATA: GameMetadata = GameMetadata {
    slug: "heart_chase",
    ruleset_version: 1,
    title: "Heart Chase",
    tagline: "Schlag die Jagdpunktzahl",
    description: "Übertriff die letzte Aufnahme. Wer scheitert, verliert ein Herz und setzt trotzdem die neue Jagd.",
    accent: "#ef476f",
    accent_secondary: "#ffd166",
    visual: "heart-chase",
    icon: "heart",
    artwork: "/static/assets/modes/heart_chase.webp",
    sound_theme: "arcade",
    min_players: 2,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct HeartChaseMode;

pub(super) static HEART_CHASE_MODE: HeartChaseMode = HeartChaseMode;

impl GameMode for HeartChaseMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let maximum = maximum_hearts(state)?;
        let hearts = state
            .players
            .iter()
            .map(|player| (player.id.clone(), Value::from(maximum)))
            .collect::<Map<_, _>>();
        state.mode_state = json!({
            "challenge_score": 0,
            "hearts": Value::Object(hearts),
            "opening_turn": true,
        });
        state.message = "Eröffnet die Jagd!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        let value = match event {
            DartEvent::Hit { score, .. } => i64::from(*score),
            DartEvent::Miss { .. } => 0,
        };
        let player_index = state.current_player_index;
        let player_id = state.players[player_index].id.clone();
        state.players[player_index].score = state.players[player_index].score.saturating_add(value);
        let turn_total = state.turn_score.saturating_add(value);
        let challenge = i64::from(challenge_score(state)?);
        if state.darts_in_turn < 2 {
            state.message = format!("{turn_total} · Jagd {challenge}");
            return Ok(value);
        }

        let opening = state
            .mode_state
            .get("opening_turn")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        state.mode_state["challenge_score"] = Value::from(turn_total);
        state.mode_state["opening_turn"] = Value::Bool(false);
        if opening {
            state.message = format!("Jagd eröffnet: {turn_total}");
            return Ok(value);
        }
        if turn_total > challenge {
            state.message = format!("CHASE BEATEN! {challenge} → {turn_total}");
            return Ok(value);
        }

        let remaining = hearts(state, &player_id)?.saturating_sub(1);
        set_hearts(state, &player_id, remaining)?;
        let active = state
            .players
            .iter()
            .filter(|player| hearts(state, &player.id).unwrap_or_default() > 0)
            .collect::<Vec<_>>();
        if active.len() == 1 {
            let winner = active[0];
            state.status = GameStatus::Finished;
            state.winner_id = Some(winner.id.clone());
            state.winner_ids = vec![winner.id.clone()];
            state.result_type = "individual_win".into();
            state.message = format!("{} gewinnt die Herzjagd!", winner.name);
        } else {
            state.message = format!("HEART LOST · Neue Jagd: {turn_total}");
        }
        Ok(value)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let maximum = maximum_hearts(state).unwrap_or(3);
        let challenge = challenge_score(state).unwrap_or_default();
        let opening = state
            .mode_state
            .get("opening_turn")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let rows = state
            .players
            .iter()
            .map(|player| {
                let remaining = hearts(state, &player.id).unwrap_or_default();
                json!({
                    "label": player.name,
                    "value": format!("{}{}", "♥".repeat(usize::from(remaining)), "♡".repeat(usize::from(maximum.saturating_sub(remaining)))),
                    "state": if remaining == 0 { "danger" } else { "" },
                })
            })
            .collect::<Vec<_>>();
        let current = state.players.get(state.current_player_index);
        json!({
            "prompt": if opening { "Jagd eröffnen!".to_string() } else { format!("Schlag {challenge}!") },
            "targets": [],
            "panel": {
                "title": "HERZJAGD",
                "headline": format!("Aktuelle Jagd: {challenge}"),
                "subline": if opening {
                    "Drei Darts legen die erste Jagd fest".to_string()
                } else if let Some(player) = current {
                    format!("{} muss strikt mehr werfen", player.name)
                } else {
                    String::new()
                },
                "rows": rows,
            },
        })
    }

    fn is_player_active(&self, state: &RegisteredGameState, player_index: usize) -> bool {
        state
            .players
            .get(player_index)
            .is_some_and(|player| hearts(state, &player.id).unwrap_or_default() > 0)
    }
}

fn maximum_hearts(state: &RegisteredGameState) -> Result<u8, GameError> {
    state
        .options
        .get("hearts")
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn challenge_score(state: &RegisteredGameState) -> Result<u32, GameError> {
    state
        .mode_state
        .get("challenge_score")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn hearts(state: &RegisteredGameState, player_id: &str) -> Result<u8, GameError> {
    state
        .mode_state
        .get("hearts")
        .and_then(Value::as_object)
        .and_then(|hearts| hearts.get(player_id))
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn set_hearts(
    state: &mut RegisteredGameState,
    player_id: &str,
    remaining: u8,
) -> Result<(), GameError> {
    let value = state
        .mode_state
        .get_mut("hearts")
        .and_then(Value::as_object_mut)
        .and_then(|hearts| hearts.get_mut(player_id))
        .ok_or_else(invalid_state)?;
    *value = Value::from(remaining);
    Ok(())
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid heart_chase mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registered::{RegisteredGame, advance_player};

    #[test]
    fn eliminated_players_are_skipped_without_advancing_the_round() {
        let mut game = RegisteredGame::new(
            "heart_chase",
            vec![
                ("ada".into(), "Ada".into()),
                ("bob".into(), "Bob".into()),
                ("cara".into(), "Cara".into()),
            ],
            &json!({"hearts": 2}),
        )
        .expect("heart chase game");
        game.state.mode_state["hearts"]["bob"] = Value::from(0);
        game.state.status = GameStatus::Hold;

        advance_player(&HEART_CHASE_MODE, &mut game.state).expect("next active player");

        assert_eq!(game.state.current_player_index, 2);
        assert_eq!(game.state.round_number, 1);
        assert_eq!(game.state.status, GameStatus::Running);
    }
}
