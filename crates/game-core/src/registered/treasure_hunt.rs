use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{parse_target, same_target, target_pool, target_value, zone_id},
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
static TRAP_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(3, "3 Fallen"),
    choice_integer(5, "5 Fallen"),
    choice_integer(8, "8 Fallen"),
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
        key: "traps",
        label: "Fallen",
        kind: "choice",
        default: GameOptionValue::Integer(5),
        choices: &TRAP_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Schätze versteckt",
        body: "Treffer decken geheime Inhalte auf.",
        icon: "gem",
    },
    GameInstruction {
        title: "Gold lohnt sich",
        body: "Gold und Silber bringen große Punkte.",
        icon: "coins",
    },
    GameInstruction {
        title: "Fallen meiden",
        body: "Rote Fallen kosten Punkte.",
        icon: "trap",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "treasure_hunt",
    ruleset_version: 1,
    title: "Treasure Hunt",
    tagline: "Finde Schätze, meide Fallen",
    description: "Das Board ist eine Schatzkarte. Treffer decken versteckte Münzen, Gold und Fallen auf.",
    accent: "#ffcf33",
    accent_secondary: "#3dff91",
    visual: "treasure-hunt",
    icon: "gem",
    artwork: "/static/assets/modes/treasure_hunt.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
};

pub(super) struct TreasureHuntMode;
pub(super) static TREASURE_HUNT_MODE: TreasureHuntMode = TreasureHuntMode;

impl GameMode for TreasureHuntMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let mut available = target_pool("normal");
        let mut pool = Vec::with_capacity(available.len());
        while !available.is_empty() {
            let index = state.random_index(available.len())?;
            pool.push(available.remove(index));
        }

        let trap_count = traps(state)?;
        let special_count = 4_usize.saturating_add(8).saturating_add(trap_count);
        if special_count > pool.len() {
            return Err(invalid_state());
        }
        let mut rewards = Vec::with_capacity(pool.len());
        rewards.extend(std::iter::repeat_n("gold", 4));
        rewards.extend(std::iter::repeat_n("silver", 8));
        rewards.extend(std::iter::repeat_n("trap", trap_count));
        rewards.extend(std::iter::repeat_n("coin", pool.len() - special_count));
        for index in (1..rewards.len()).rev() {
            let swap_index = state.random_index(index + 1)?;
            rewards.swap(index, swap_index);
        }

        let mut hidden = Map::new();
        for (target, reward) in pool.iter().zip(rewards) {
            hidden.insert(
                zone_id(target),
                json!({"dart": target_value(target), "reward": reward, "revealed_by": null}),
            );
        }
        state.mode_state = json!({"hidden": hidden, "revealed": {}});
        state.message = "Finde die Schätze!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        let found_key = state
            .mode_state
            .get("hidden")
            .and_then(Value::as_object)
            .ok_or_else(invalid_state)?
            .iter()
            .find_map(|(key, item)| {
                let target = item
                    .get("dart")
                    .and_then(|value| parse_target(value).ok())?;
                same_target(event, &target).then(|| key.clone())
            });

        let points = if matches!(event, DartEvent::Miss { .. }) {
            state.message = "Miss – kein Fund".into();
            0
        } else if let Some(key) = found_key {
            reveal(state, event, &key)?
        } else {
            state.message = format!("{}: leer", event.label());
            0
        };
        finish_fixed_round_game(state, "{winner} findet den größten Schatz!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(revealed) = state.mode_state.get("revealed").and_then(Value::as_object) else {
            return json!({"prompt": "Treffer decken Schätze auf!", "bonus": [], "targets": [], "danger": []});
        };
        let mut bonus = Vec::new();
        let mut targets = Vec::new();
        let mut danger = Vec::new();
        for item in revealed.values() {
            let Some(reward) = item.get("reward").and_then(Value::as_str) else {
                continue;
            };
            let Some(target) = item.get("dart").and_then(|value| parse_target(value).ok()) else {
                continue;
            };
            let entry = json!({
                "id": zone_id(&target),
                "field": target.field,
                "ring": target.ring,
                "color": reward_color(reward),
                "label": reward_label(reward),
                "pulse": false,
            });
            match reward {
                "trap" => danger.push(entry),
                "gold" => bonus.push(entry),
                _ => targets.push(entry),
            }
        }
        json!({
            "prompt": "Treffer decken Schätze auf!",
            "bonus": bonus,
            "targets": targets,
            "danger": danger,
        })
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} findet den größten Schatz!")
    }
}

fn reveal(state: &mut RegisteredGameState, event: &DartEvent, key: &str) -> Result<i64, GameError> {
    let item = state
        .mode_state
        .get("hidden")
        .and_then(Value::as_object)
        .and_then(|hidden| hidden.get(key))
        .cloned()
        .ok_or_else(invalid_state)?;
    if item.get("revealed_by").is_some_and(Value::is_string) {
        state.message = format!("{}: bereits gefunden", event.label());
        return Ok(0);
    }
    let reward = item
        .get("reward")
        .and_then(Value::as_str)
        .ok_or_else(invalid_state)?;
    let points = reward_points(reward)?;
    let player = state
        .players
        .get_mut(state.current_player_index)
        .ok_or(GameError::NoPlayers)?;
    player.score = player.score.saturating_add(points);
    let player_id = player.id.clone();
    let hidden_item = state
        .mode_state
        .get_mut("hidden")
        .and_then(Value::as_object_mut)
        .and_then(|hidden| hidden.get_mut(key))
        .ok_or_else(invalid_state)?;
    hidden_item["revealed_by"] = Value::from(player_id);
    let revealed_item = hidden_item.clone();
    state
        .mode_state
        .get_mut("revealed")
        .and_then(Value::as_object_mut)
        .ok_or_else(invalid_state)?
        .insert(key.into(), revealed_item);
    state.message = format!("{}: {}", event.label(), reward_label(reward));
    Ok(points)
}

fn traps(state: &RegisteredGameState) -> Result<usize, GameError> {
    state
        .options
        .get("traps")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn reward_points(reward: &str) -> Result<i64, GameError> {
    match reward {
        "gold" => Ok(75),
        "silver" => Ok(35),
        "coin" => Ok(10),
        "trap" => Ok(-40),
        _ => Err(invalid_state()),
    }
}

fn reward_color(reward: &str) -> &'static str {
    match reward {
        "gold" => "gold",
        "silver" => "cyan",
        "trap" => "red",
        _ => "green",
    }
}

fn reward_label(reward: &str) -> &'static str {
    match reward {
        "gold" => "+75",
        "silver" => "+35",
        "trap" => "TRAP",
        _ => "+10",
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
    GameError::RulesetUnavailable("invalid treasure_hunt mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameStatus, RegisteredGame};
    use sdb_contracts::Ring;

    fn reward_target(state: &RegisteredGameState, reward: &str) -> super::super::arcade::Target {
        state.mode_state["hidden"]
            .as_object()
            .expect("hidden")
            .values()
            .find(|item| item["reward"] == reward)
            .and_then(|item| parse_target(&item["dart"]).ok())
            .expect("reward target")
    }

    fn hit(target: &super::super::arcade::Target, seq: u64) -> DartEvent {
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
    fn a_trap_can_make_the_score_negative_and_undo_restores_the_map() {
        let mut game = RegisteredGame::new_seeded(
            "treasure_hunt",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 3, "traps": 3}),
            42,
        )
        .expect("game");
        let trap = reward_target(game.state(), "trap");

        game.apply_throw(&hit(&trap, 1)).expect("trap");

        assert_eq!(game.state().players[0].score, -40);
        assert_eq!(game.state().turn_score, -40);
        assert_eq!(
            game.state().overlay["danger"].as_array().map(Vec::len),
            Some(1)
        );
        game.undo().expect("undo trap");
        assert_eq!(game.state().players[0].score, 0);
        assert_eq!(game.state().mode_state["revealed"], json!({}));
    }

    #[test]
    fn a_revealed_treasure_can_only_score_once_globally() {
        let mut game = RegisteredGame::new_seeded(
            "treasure_hunt",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"rounds": 3, "traps": 3}),
            42,
        )
        .expect("game");
        let gold = reward_target(game.state(), "gold");

        game.apply_throw(&hit(&gold, 1)).expect("Ada finds gold");
        game.next_player().expect("Bob starts");
        game.apply_throw(&hit(&gold, 2)).expect("Bob repeats gold");

        assert_eq!(game.state().players[0].score, 75);
        assert_eq!(game.state().players[1].score, 0);
        assert_eq!(game.state().turn_score, 0);
    }

    #[test]
    fn skipping_the_last_round_finishes_the_game() {
        let mut game = RegisteredGame::new_seeded(
            "treasure_hunt",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 3, "traps": 3}),
            42,
        )
        .expect("game");

        game.next_player().expect("skip round one");
        game.next_player().expect("skip round two");
        game.next_player().expect("skip round three");

        assert_eq!(game.state().status, GameStatus::Finished);
        assert_eq!(game.state().winner_id.as_deref(), Some("ada"));
    }

    #[test]
    fn the_normal_pool_contains_only_one_bull_target() {
        assert_eq!(METADATA.ruleset_version, 1);
        assert_eq!(METADATA.options[0].default, GameOptionValue::Integer(5));
        assert_eq!(METADATA.options[1].default, GameOptionValue::Integer(5));
        let game = RegisteredGame::new_seeded(
            "treasure_hunt",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 3, "traps": 3}),
            42,
        )
        .expect("game");
        let bull_rings = game.state().mode_state["hidden"]
            .as_object()
            .expect("hidden")
            .values()
            .filter_map(|item| parse_target(&item["dart"]).ok())
            .filter(|target| target.field == 25)
            .map(|target| target.ring)
            .collect::<Vec<_>>();
        assert_eq!(bull_rings, [Ring::DoubleBull]);
    }
}
