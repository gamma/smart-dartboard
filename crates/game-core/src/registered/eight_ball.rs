use super::{GameInstruction, GameMetadata, GameMode, RegisteredGameState};
use crate::{GameError, GameStatus};
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Map, Value, json};

static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Eigene Kugeln",
        body: "Spieler 1 räumt 1–7, Spieler 2 räumt 9–15.",
        icon: "balls",
    },
    GameInstruction {
        title: "Foul beendet",
        body: "Falsche Kugel, neutrales Feld oder Miss beendet die Aufnahme sofort.",
        icon: "foul",
    },
    GameInstruction {
        title: "Schwarze 8",
        body: "Sind deine Kugeln weg, gewinnst du mit Double Bull. Zu früh gewinnt der Gegner.",
        icon: "eight",
    },
];

static METADATA: GameMetadata = GameMetadata {
    slug: "eight_ball",
    ruleset_version: 1,
    format: sdb_contracts::GameFormat::Individual,
    title: "8-Ball Darts",
    tagline: "Räume deine Kugeln ab",
    description: "Ein klares Duell: erst die eigenen Kugeln versenken, dann Double Bull als schwarze 8.",
    accent: "#3d8b74",
    accent_secondary: "#e9c46a",
    visual: "eight-ball",
    icon: "eight",
    artwork: "/static/assets/modes/eight_ball.webp",
    sound_theme: "club",
    min_players: 2,
    max_players: 2,
    options: &[],
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct EightBallMode;

pub(super) static EIGHT_BALL_MODE: EightBallMode = EightBallMode;

impl GameMode for EightBallMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let mut balls = Map::new();
        balls.insert(state.players[0].id.clone(), json!([1, 2, 3, 4, 5, 6, 7]));
        balls.insert(
            state.players[1].id.clone(),
            json!([9, 10, 11, 12, 13, 14, 15]),
        );
        state.mode_state = json!({"balls": Value::Object(balls)});
        state.message = "Räumt eure Kugeln ab!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        let player_index = state.current_player_index;
        let opponent_index = 1_usize.saturating_sub(player_index);
        let player_id = state.players[player_index].id.clone();
        let player_name = state.players[player_index].name.clone();
        let opponent_id = state.players[opponent_index].id.clone();
        let opponent_name = state.players[opponent_index].name.clone();
        let remaining = balls(state, &player_id)?;

        if matches!(
            event,
            DartEvent::Hit {
                field: 25,
                multiplier: 2,
                ..
            }
        ) {
            let (winner_id, message) = if remaining.is_empty() {
                (player_id, format!("BLACK 8! {player_name} gewinnt"))
            } else {
                (
                    opponent_id,
                    format!("8-Ball zu früh! {opponent_name} gewinnt"),
                )
            };
            state.status = GameStatus::Finished;
            state.winner_id = Some(winner_id.clone());
            state.winner_ids = vec![winner_id];
            state.result_type = "individual_win".into();
            state.message = message;
            return Ok(0);
        }

        let (field, is_single) = match event {
            DartEvent::Hit { field, ring, .. } => (
                *field,
                matches!(ring, Ring::SingleInner | Ring::SingleOuter),
            ),
            DartEvent::Miss { .. } => (0, false),
        };
        if is_single && remaining.contains(&field) {
            remove_ball(state, &player_id, field)?;
            state.players[player_index].score =
                state.players[player_index].score.saturating_add(20);
            state.message = format!("Kugel {field} versenkt! +20");
            return Ok(20);
        }

        state.status = GameStatus::Hold;
        state.message = "FOUL · Spielerwechsel".into();
        Ok(0)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(player) = state.players.get(state.current_player_index) else {
            return json!({"prompt": "8-BALL", "targets": []});
        };
        let remaining = balls(state, &player.id).unwrap_or_default();
        let targets = if remaining.is_empty() {
            vec![json!({
                "id": "DBULL", "field": 25, "ring": Ring::DoubleBull,
                "color": "gold", "label": "8", "pulse": true,
            })]
        } else {
            remaining
                .iter()
                .flat_map(|field| {
                    [Ring::SingleInner, Ring::SingleOuter].map(|ring| {
                        json!({
                            "id": format!("{}{}", if ring == Ring::SingleInner { "SI" } else { "SO" }, field),
                            "field": field, "ring": ring, "color": "green",
                            "label": field.to_string(), "pulse": false,
                        })
                    })
                })
                .collect()
        };
        let rows = state
            .players
            .iter()
            .map(|candidate| {
                let candidate_balls = balls(state, &candidate.id).unwrap_or_default();
                json!({
                    "label": candidate.name,
                    "value": if candidate_balls.is_empty() {
                        "8-BALL".to_string()
                    } else {
                        candidate_balls.iter().map(u8::to_string).collect::<Vec<_>>().join(" · ")
                    },
                })
            })
            .collect::<Vec<_>>();
        json!({
            "prompt": if remaining.is_empty() {
                "BLACK 8 · DOUBLE BULL!".to_string()
            } else {
                format!("Versenke: {}", remaining.iter().map(u8::to_string).collect::<Vec<_>>().join(" · "))
            },
            "targets": targets,
            "panel": {
                "title": "8-BALL",
                "headline": if remaining.is_empty() { "Schwarze 8".to_string() } else { format!("{} Kugeln übrig", remaining.len()) },
                "rows": rows,
            },
        })
    }
}

fn balls(state: &RegisteredGameState, player_id: &str) -> Result<Vec<u8>, GameError> {
    state
        .mode_state
        .get("balls")
        .and_then(Value::as_object)
        .and_then(|balls| balls.get(player_id))
        .and_then(Value::as_array)
        .ok_or_else(invalid_state)?
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .ok_or_else(invalid_state)
        })
        .collect()
}

fn remove_ball(
    state: &mut RegisteredGameState,
    player_id: &str,
    field: u8,
) -> Result<(), GameError> {
    let values = state
        .mode_state
        .get_mut("balls")
        .and_then(Value::as_object_mut)
        .and_then(|balls| balls.get_mut(player_id))
        .and_then(Value::as_array_mut)
        .ok_or_else(invalid_state)?;
    let position = values
        .iter()
        .position(|value| value.as_u64() == Some(u64::from(field)))
        .ok_or_else(invalid_state)?;
    values.remove(position);
    Ok(())
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid eight_ball mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    fn single(seq: u64, field: u8) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring: Ring::SingleInner,
            multiplier: 1,
            label: format!("S{field}"),
            score: u16::from(field),
        }
    }

    #[test]
    fn clearing_all_balls_then_double_bull_wins() {
        let mut game = RegisteredGame::new(
            "eight_ball",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({}),
        )
        .expect("game");
        let mut seq = 1;
        for group in [[1, 2, 3], [4, 5, 6]] {
            for field in group {
                game.apply_throw(&single(seq, field)).expect("Ada ball");
                seq += 1;
            }
            game.continue_turn().expect("Bob turn");
            game.apply_throw(&DartEvent::Miss {
                seq,
                label: "MISS".into(),
                score: 0,
            })
            .expect("Bob foul");
            seq += 1;
            game.continue_turn().expect("Ada turn");
        }
        game.apply_throw(&single(seq, 7)).expect("last ball");
        game.apply_throw(&DartEvent::Hit {
            seq: seq + 1,
            field: 25,
            ring: Ring::DoubleBull,
            multiplier: 2,
            label: "DBULL".into(),
            score: 50,
        })
        .expect("black eight");
        assert_eq!(game.state().status, GameStatus::Finished);
        assert_eq!(game.state().winner_id.as_deref(), Some("ada"));
        assert_eq!(game.state().players[0].score, 140);
    }

    #[test]
    fn exactly_two_players_are_required() {
        let result =
            RegisteredGame::new("eight_ball", vec![("ada".into(), "Ada".into())], &json!({}));
        assert!(matches!(result, Err(GameError::InvalidOptions(_))));
    }
}
