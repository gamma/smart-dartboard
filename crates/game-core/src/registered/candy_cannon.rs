use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState, finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Map, Value, json};

static ROUND_CHOICES: [GameOptionChoice; 2] =
    [choice_integer(5, "5 Runden"), choice_integer(8, "8 Runden")];

static OPTIONS: [GameOption; 1] = [GameOption {
    key: "rounds",
    label: "Runden",
    kind: "choice",
    default: GameOptionValue::Integer(5),
    choices: &ROUND_CHOICES,
}];

static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Auf 8, 9 oder 10 laden",
        body: "Single zählt 1, Double 2, Triple 3 und Bull 4 Ladung.",
        icon: "candy",
    },
    GameInstruction {
        title: "BEREIT? Bull treffen",
        body: "Sobald BEREIT erscheint, feuert Single oder Double Bull: +50 für dich, −25 für den Führenden.",
        icon: "cannon",
    },
    GameInstruction {
        title: "11 ist zu viel",
        body: "Über 10 überhitzt die Kanone und deine Ladung fällt auf null.",
        icon: "danger",
    },
];

static METADATA: GameMetadata = GameMetadata {
    slug: "candy_cannon",
    ruleset_version: 1,
    title: "Candy Cannon",
    tagline: "Laden, riskieren, feuern",
    description: "Treffer laden deine Süßigkeitenkanone. Triff bei 8–10 Ladung ins Bull, bevor sie überhitzt.",
    accent: "#e76f51",
    accent_secondary: "#f4d35e",
    visual: "candy-cannon",
    icon: "cannon",
    artwork: "/static/assets/modes/candy_cannon.webp",
    sound_theme: "arcade",
    min_players: 2,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
};

pub(super) struct CandyCannonMode;

pub(super) static CANDY_CANNON_MODE: CandyCannonMode = CandyCannonMode;

impl GameMode for CandyCannonMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({
            "charge": state.players.iter().map(|player| (player.id.clone(), Value::from(0))).collect::<Map<_, _>>(),
            "last_effect": "",
            "target_player_id": null,
            "target_score_loss": 0,
        });
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        clear_effect(state);
        let player_index = state.current_player_index;
        let player_id = state.players[player_index].id.clone();
        let points = match event {
            DartEvent::Miss { .. } => {
                state.message = "MISS · Keine Ladung".into();
                0
            }
            DartEvent::Hit {
                field, multiplier, ..
            } => {
                let current_charge = charge(state, &player_id)?;
                let is_bull = *field == 25;
                if is_bull && (8..=10).contains(&current_charge) {
                    fire(state, player_index)?
                } else {
                    let addition = if is_bull { 4 } else { *multiplier };
                    let next_charge = current_charge.saturating_add(addition);
                    if next_charge > 10 {
                        set_charge(state, &player_id, 0)?;
                        state.mode_state["last_effect"] = Value::from("candy_overheat");
                        state.message = "OVERHEAT! Ladung verloren".into();
                    } else {
                        set_charge(state, &player_id, next_charge)?;
                        state.message = format!("Kanone geladen: {next_charge}/10");
                    }
                    0
                }
            }
        };
        finish_fixed_round_game(state, "{winner} gewinnt die Candy Cannon!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(player) = state.players.get(state.current_player_index) else {
            return json!({"prompt": "Candy Cannon", "targets": []});
        };
        let current_charge = charge(state, &player.id).unwrap_or_default();
        let ready = (8..=10).contains(&current_charge);
        let target = target_index(state, state.current_player_index)
            .ok()
            .and_then(|index| state.players.get(index));
        let targets = if ready {
            vec![
                json!({
                    "id": "SBULL",
                    "field": 25,
                    "ring": Ring::SingleBull,
                    "color": "#f4d35e",
                    "label": "FIRE",
                    "pulse": true,
                }),
                json!({
                    "id": "DBULL",
                    "field": 25,
                    "ring": Ring::DoubleBull,
                    "color": "#e76f51",
                    "label": "",
                    "pulse": true,
                }),
            ]
        } else {
            Vec::new()
        };
        json!({
            "prompt": if ready {
                "BEREIT · JETZT BULL TREFFEN!"
            } else {
                "LADUNG AUF 8–10 STELLEN · DANN MIT BULL FEUERN"
            },
            "targets": targets,
            "panel": {
                "title": "CANDY CANNON",
                "headline": if ready {
                    format!("BEREIT · {current_charge}/10")
                } else {
                    format!("Ladung {current_charge}/10")
                },
                "subline": if ready {
                    target.map_or_else(
                        || "Bull feuert".into(),
                        |target| format!("BULL feuert auf {}", target.name),
                    )
                } else {
                    "Über 10 überhitzt die Kanone".into()
                },
                "progress": {"value": current_charge, "max": 10},
            },
        })
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} gewinnt die Candy Cannon!")
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

fn charge(state: &RegisteredGameState, player_id: &str) -> Result<u8, GameError> {
    state
        .mode_state
        .get("charge")
        .and_then(Value::as_object)
        .and_then(|charge| charge.get(player_id))
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn set_charge(
    state: &mut RegisteredGameState,
    player_id: &str,
    value: u8,
) -> Result<(), GameError> {
    state
        .mode_state
        .get_mut("charge")
        .and_then(Value::as_object_mut)
        .ok_or_else(invalid_state)?
        .insert(player_id.into(), Value::from(value));
    Ok(())
}

fn target_index(state: &RegisteredGameState, player_index: usize) -> Result<usize, GameError> {
    let high_score = state
        .players
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != player_index)
        .map(|(_, player)| player.score)
        .max()
        .ok_or(GameError::NoPlayers)?;
    (1..state.players.len())
        .map(|offset| (player_index + offset) % state.players.len())
        .find(|index| state.players[*index].score == high_score)
        .ok_or_else(invalid_state)
}

fn fire(state: &mut RegisteredGameState, player_index: usize) -> Result<i64, GameError> {
    let target_index = target_index(state, player_index)?;
    let player_id = state.players[player_index].id.clone();
    let player_name = state.players[player_index].name.clone();
    let target_id = state.players[target_index].id.clone();
    let target_name = state.players[target_index].name.clone();
    let previous_score = state.players[target_index].score;
    state.players[player_index].score = state.players[player_index].score.saturating_add(50);
    state.players[target_index].score = previous_score.saturating_sub(25).max(0);
    let score_loss = previous_score.saturating_sub(state.players[target_index].score);
    set_charge(state, &player_id, 0)?;
    state.mode_state["last_effect"] = Value::from("candy_fire");
    state.mode_state["target_player_id"] = Value::from(target_id);
    state.mode_state["target_score_loss"] = Value::from(score_loss);
    state.message = format!("FIRE! {player_name} +50 · {target_name} -25");
    Ok(50)
}

fn clear_effect(state: &mut RegisteredGameState) {
    state.mode_state["last_effect"] = Value::from("");
    state.mode_state["target_player_id"] = Value::Null;
    state.mode_state["target_score_loss"] = Value::from(0);
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid candy_cannon mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    fn hit(seq: u64, field: u8, ring: Ring, multiplier: u8) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring,
            multiplier,
            label: if field == 25 {
                if ring == Ring::DoubleBull {
                    "DBull"
                } else {
                    "SBull"
                }
                .into()
            } else {
                format!("S{field}")
            },
            score: u16::from(field) * u16::from(multiplier),
        }
    }

    #[test]
    fn ready_bull_fires_at_the_next_leading_opponent() {
        let mut game = RegisteredGame::new(
            "candy_cannon",
            vec![
                ("ada".into(), "Ada".into()),
                ("bob".into(), "Bob".into()),
                ("cid".into(), "Cid".into()),
            ],
            &json!({}),
        )
        .expect("game");
        game.state.players[1].score = 60;
        game.state.players[2].score = 60;
        set_charge(&mut game.state, "ada", 8).expect("charge");

        game.apply_throw(&hit(1, 25, Ring::SingleBull, 1))
            .expect("fire");

        assert_eq!(game.state.players[0].score, 50);
        assert_eq!(game.state.players[1].score, 35);
        assert_eq!(game.state.players[2].score, 60);
        assert_eq!(charge(game.state(), "ada").expect("charge"), 0);
        assert_eq!(game.state.mode_state["target_player_id"], "bob");
    }

    #[test]
    fn charge_over_ten_overheats_and_is_undoable() {
        let mut game = RegisteredGame::new(
            "candy_cannon",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({}),
        )
        .expect("game");
        for seq in 1..=3 {
            game.apply_throw(&hit(seq, 20, Ring::Triple, 3))
                .expect("Ada charge");
        }
        game.continue_turn().expect("Bob turn");
        for seq in 4..=6 {
            game.apply_throw(&DartEvent::Miss {
                seq,
                label: "MISS".into(),
                score: 0,
            })
            .expect("Bob miss");
        }
        game.continue_turn().expect("Ada turn");

        game.apply_throw(&hit(7, 20, Ring::Double, 2))
            .expect("overheat");
        assert_eq!(charge(game.state(), "ada").expect("charge"), 0);
        assert_eq!(game.state.mode_state["last_effect"], "candy_overheat");

        game.undo().expect("undo");
        assert_eq!(charge(game.state(), "ada").expect("charge"), 9);
    }
}
