use super::arcade::{Target, parse_target, same_target, sample_targets, target_value};
use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
};
use crate::{GameError, GameStatus};
use sdb_contracts::DartEvent;
use serde_json::{Value, json};

static HP_CHOICES: [GameOptionChoice; 3] = [
    integer_choice(600, "600 HP"),
    integer_choice(1000, "1000 HP"),
    integer_choice(1500, "1500 HP"),
];
static WEAK_CHOICES: [GameOptionChoice; 3] = [
    integer_choice(2, "2"),
    integer_choice(3, "3"),
    integer_choice(5, "5"),
];
static ROUND_CHOICES: [GameOptionChoice; 3] = [
    integer_choice(5, "5 Runden"),
    integer_choice(8, "8 Runden"),
    integer_choice(12, "12 Runden"),
];
static OPTIONS: [GameOption; 3] = [
    GameOption {
        key: "boss_hp",
        label: "Boss HP",
        kind: "choice",
        default: GameOptionValue::Integer(1000),
        choices: &HP_CHOICES,
    },
    GameOption {
        key: "weak_points",
        label: "Schwachpunkte",
        kind: "choice",
        default: GameOptionValue::Integer(3),
        choices: &WEAK_CHOICES,
    },
    GameOption {
        key: "rounds",
        label: "Rundenlimit",
        kind: "choice",
        default: GameOptionValue::Integer(8),
        choices: &ROUND_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Schaden machen",
        body: "Jeder Treffer zieht seinen klassischen Dartwert von den Boss-HP ab.",
        icon: "damage",
    },
    GameInstruction {
        title: "Schwachpunkte",
        body: "Goldene exakte Segmente verursachen doppelten Schaden und verschieben danach alle Schwachpunkte.",
        icon: "weak",
    },
    GameInstruction {
        title: "Zeitlimit",
        body: "Besiegt den Boss gemeinsam innerhalb der gewählten Runden. Der meiste Schaden ist nur eine MVP-Ehrung.",
        icon: "coop",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "boss_fight",
    ruleset_version: 1,
    title: "Boss Fight",
    tagline: "Alle gegen den Boss",
    description: "Der bestehende kooperative V1-Bosskampf: Treffer verursachen Schaden, exakte Schwachpunkte doppelten Schaden.",
    accent: "#ff4f79",
    accent_secondary: "#9b5cff",
    visual: "boss-fight",
    icon: "monster",
    artwork: "/static/assets/modes/boss_fight.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct BossFightMode;
pub(super) static BOSS_FIGHT_MODE: BossFightMode = BossFightMode;

impl GameMode for BossFightMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let maximum = boss_hp_option(state)?;
        let targets = sample_targets(state, weak_count(state)?, "normal")?;
        state.mode_state = json!({
            "boss_hp": maximum,
            "max_hp": maximum,
            "weak": targets.iter().map(target_value).collect::<Vec<_>>(),
            "last_effect": "",
            "effect_damage": 0,
            "effect_weak": false,
            "effect_player_id": null,
        });
        state.message = "Boss Fight!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        clear_effect(state);
        let targets = weak_targets(state)?;
        let is_weak = targets.iter().any(|target| same_target(event, target));
        let base = match event {
            DartEvent::Hit { score, .. } => i64::from(*score),
            DartEvent::Miss { .. } => 0,
        };
        let damage = base.saturating_mul(if is_weak { 2 } else { 1 });
        if damage > 0 {
            let player = state
                .players
                .get_mut(state.current_player_index)
                .ok_or(GameError::NoPlayers)?;
            player.score = player.score.saturating_add(damage);
            let player_id = player.id.clone();
            let remaining = boss_hp(state)?.saturating_sub(damage).max(0);
            state.mode_state["boss_hp"] = Value::from(remaining);
            if is_weak {
                refresh_weak(state)?;
            }
            set_effect(
                state,
                if is_weak { "boss_weak" } else { "boss_hit" },
                damage,
                is_weak,
                Some(player_id),
            );
        }

        if boss_hp(state)? == 0 {
            finish_team_win(state)?;
            set_effect(state, "boss_defeated", damage, is_weak, None);
            return Ok(damage);
        }

        let final_dart = state.darts_in_turn >= 2;
        let final_player = state.current_player_index.saturating_add(1) == state.players.len();
        let final_round = state.round_number >= rounds(state)?;
        if final_dart && final_player && final_round {
            finish_loss(state)?;
            return Ok(damage);
        }

        state.message = match event {
            DartEvent::Hit { label, .. } => format!("{label} macht {damage} Schaden"),
            DartEvent::Miss { .. } => "Miss – kein Schaden".into(),
        };
        Ok(damage)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        overlay(state).unwrap_or_else(|_| json!({"prompt":"Boss Fight","bonus":[]}))
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let final_player = state.current_player_index.saturating_add(1) == state.players.len();
        let final_round = state.round_number >= rounds(state)?;
        if final_player && final_round {
            finish_loss(state)?;
        }
        Ok(())
    }
}

fn refresh_weak(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let targets = sample_targets(state, weak_count(state)?, "normal")?;
    state.mode_state["weak"] = Value::Array(targets.iter().map(target_value).collect::<Vec<_>>());
    Ok(())
}

fn weak_targets(state: &RegisteredGameState) -> Result<Vec<Target>, GameError> {
    state.mode_state["weak"]
        .as_array()
        .ok_or_else(invalid_state)?
        .iter()
        .map(parse_target)
        .collect()
}

fn finish_team_win(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let highest = state
        .players
        .iter()
        .map(|player| player.score)
        .max()
        .ok_or(GameError::NoPlayers)?;
    let leaders = state
        .players
        .iter()
        .filter(|player| player.score == highest)
        .collect::<Vec<_>>();
    state.status = GameStatus::Finished;
    state.winner_id = None;
    state.winner_ids = state
        .players
        .iter()
        .map(|player| player.id.clone())
        .collect();
    state.result_type = "team_win".into();
    state.message = if leaders.len() == 1 {
        format!("Boss besiegt! MVP: {}", leaders[0].name)
    } else {
        format!(
            "Unentschieden: {}",
            leaders
                .iter()
                .map(|player| player.name.as_str())
                .collect::<Vec<_>>()
                .join(" · ")
        )
    };
    Ok(())
}

fn finish_loss(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let remaining = boss_hp(state)?;
    state.status = GameStatus::Finished;
    state.winner_id = None;
    state.winner_ids.clear();
    state.result_type = "challenge_loss".into();
    state.message = format!("Boss gewinnt mit {remaining} HP!");
    set_effect(state, "boss_victory", 0, false, None);
    Ok(())
}

fn overlay(state: &RegisteredGameState) -> Result<Value, GameError> {
    let hp = boss_hp(state)?;
    let maximum = state
        .mode_state
        .get("max_hp")
        .and_then(Value::as_i64)
        .unwrap_or(boss_hp_option(state)?);
    let bonus = weak_targets(state)?
        .iter()
        .map(|target| {
            json!({
                "id": target.label,
                "field": target.field,
                "ring": target.ring,
                "color": "gold",
                "label": "x2",
                "pulse": true,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "prompt": format!("BOSS HP {hp} – Gold = Schwachpunkt"),
        "bonus": bonus,
        "boss": {"hp":hp,"max_hp":maximum},
    }))
}

fn boss_hp(state: &RegisteredGameState) -> Result<i64, GameError> {
    state.mode_state["boss_hp"]
        .as_i64()
        .ok_or_else(invalid_state)
}

fn boss_hp_option(state: &RegisteredGameState) -> Result<i64, GameError> {
    state
        .options
        .get("boss_hp")
        .and_then(Value::as_i64)
        .ok_or_else(invalid_state)
}

fn weak_count(state: &RegisteredGameState) -> Result<usize, GameError> {
    state
        .options
        .get("weak_points")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn rounds(state: &RegisteredGameState) -> Result<u16, GameError> {
    state
        .options
        .get("rounds")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn clear_effect(state: &mut RegisteredGameState) {
    set_effect(state, "", 0, false, None);
}

fn set_effect(
    state: &mut RegisteredGameState,
    effect: &str,
    damage: i64,
    weak: bool,
    player_id: Option<String>,
) {
    state.mode_state["last_effect"] = Value::from(effect);
    state.mode_state["effect_damage"] = Value::from(damage);
    state.mode_state["effect_weak"] = Value::from(weak);
    state.mode_state["effect_player_id"] = player_id.map_or(Value::Null, Value::from);
}

const fn integer_choice(value: i64, label: &'static str) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Integer(value),
        label,
        description: None,
        description_en: None,
    }
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid boss fight state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;
    use sdb_contracts::Ring;

    fn game() -> RegisteredGame {
        RegisteredGame::new_seeded(
            "boss_fight",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"boss_hp":600,"weak_points":3,"rounds":5}),
            42,
        )
        .expect("game")
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
    fn weak_hit_doubles_damage_and_refreshes_with_shared_randomness() {
        let mut game = game();
        assert_eq!(game.state.random_cursor, 3);
        let weak = weak_targets(&game.state).expect("weak");
        let initial_targets = game.state.mode_state["weak"].clone();
        game.apply_throw(&hit(&weak[0], 1)).expect("weak hit");
        assert_eq!(game.state.players[0].score, i64::from(weak[0].score) * 2);
        assert_eq!(game.state.mode_state["effect_weak"], true);
        assert_eq!(game.state.random_cursor, 6);
        assert_ne!(game.state.mode_state["weak"], initial_targets);
    }

    #[test]
    fn final_player_skip_loses_the_challenge() {
        let mut game = game();
        game.state.current_player_index = 1;
        game.state.round_number = 5;
        game.next_player().expect("skip");
        assert_eq!(game.state.status, GameStatus::Finished);
        assert_eq!(game.state.result_type, "challenge_loss");
    }

    #[test]
    fn defeating_the_boss_awards_every_team_member_the_win() {
        let mut game = game();
        game.state.mode_state["boss_hp"] = Value::from(1);
        let event = DartEvent::Hit {
            seq: 1,
            field: 1,
            ring: Ring::SingleOuter,
            multiplier: 1,
            label: "S1".into(),
            score: 1,
        };
        game.apply_throw(&event).expect("winning hit");
        assert_eq!(game.state.status, GameStatus::Finished);
        assert_eq!(game.state.result_type, "team_win");
        assert_eq!(game.state.winner_ids, ["ada", "bob"]);
        assert_eq!(game.state.mode_state["boss_hp"], 0);
    }
}
