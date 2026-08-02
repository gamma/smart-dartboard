use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{parse_target, same_target, target_pool, target_value, zone_id},
};
use crate::{GameError, GameStatus};
use sdb_contracts::DartEvent;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

static WAVE_CHOICES: [GameOptionChoice; 2] = [choice(4, "4 Wellen"), choice(6, "6 Wellen")];
static OPTIONS: [GameOption; 1] = [GameOption {
    key: "waves",
    label: "Wellen",
    kind: "choice",
    default: GameOptionValue::Integer(4),
    choices: &WAVE_CHOICES,
}];
static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Schiffe treffen",
        body: "Triff das exakte Segment. Single, Double und Triple machen 1, 2 oder 3 Schaden.",
        icon: "rocket",
    },
    GameInstruction {
        title: "Bull-Laser",
        body: "Bull trifft alle aktiven Schiffe gleichzeitig.",
        icon: "laser",
    },
    GameInstruction {
        title: "Erde retten",
        body: "Nach der letzten Welle räumt ihr gemeinsam die restlichen Schiffe ab.",
        icon: "earth",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "space_defender",
    ruleset_version: 2,
    format: sdb_contracts::GameFormat::Cooperative,
    title: "Space Defender",
    tagline: "Gemeinsam die Wellen stoppen",
    description: "Ein fröhliches Koop-Weltraumabenteuer: Trefft die Raumschiffe, bevor die Invasion zehn Gegner erreicht.",
    accent: "#4f9d69",
    accent_secondary: "#f2c14e",
    visual: "space-defender",
    icon: "rocket",
    artwork: "/static/assets/modes/space_defender.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Ship {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    target: Value,
    hp: u8,
    max_hp: u8,
    points: i64,
}

pub(super) struct SpaceDefenderMode;
pub(super) static SPACE_DEFENDER_MODE: SpaceDefenderMode = SpaceDefenderMode;

impl GameMode for SpaceDefenderMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({
            "ships": [], "wave": 1, "cleanup": false, "next_ship_id": 1,
            "last_effect": "", "effect_points": 0, "effect_damage": 0, "destroyed": 0,
        });
        spawn_wave(state, 1)
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        clear_effect(state);
        let mut fleet = ships(state)?;
        let mut points = 0_i64;
        let mut damage = 0_u8;
        let mut damaged = false;
        let mut destroyed = 0_u64;

        match event {
            DartEvent::Hit {
                field: 25,
                multiplier,
                ..
            } => {
                damage = if *multiplier == 2 { 2 } else { 1 };
                damaged = !fleet.is_empty();
                for ship in &mut fleet {
                    let was_alive = ship.hp > 0;
                    points = points.saturating_add(damage_ship(ship, damage));
                    if was_alive && ship.hp == 0 {
                        destroyed = destroyed.saturating_add(1);
                    }
                }
                state.message = format!("FLÄCHENLASER! {damage} Schaden an allen");
            }
            DartEvent::Hit { multiplier, .. } => {
                let matching = fleet.iter_mut().find(|ship| {
                    parse_target(&ship.target).is_ok_and(|target| same_target(event, &target))
                });
                if let Some(ship) = matching {
                    damage = *multiplier;
                    let kind = ship.kind.to_uppercase();
                    points = damage_ship(ship, damage);
                    destroyed = u64::from(ship.hp == 0);
                    damaged = true;
                    state.message = format!("{kind} getroffen · {damage} Schaden");
                } else {
                    state.message = "Laser geht vorbei".into();
                }
            }
            DartEvent::Miss { .. } => state.message = "Laser geht vorbei".into(),
        }

        fleet.retain(|ship| ship.hp > 0);
        set_ships(state, &fleet)?;
        if points != 0 {
            for teammate in &mut state.players {
                teammate.score = teammate.score.saturating_add(points);
            }
        }
        set_effect(
            state,
            if destroyed > 0 {
                "space_destroy"
            } else if damaged && matches!(event, DartEvent::Hit { field: 25, .. }) {
                "space_laser"
            } else if damaged {
                "space_hit"
            } else {
                ""
            },
            points,
            damage,
            destroyed,
        );

        if state.darts_in_turn == 2
            && state.current_player_index.saturating_add(1) == state.players.len()
        {
            finish_team_round(state, points)?;
        }
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        overlay(state).unwrap_or_else(|_| json!({}))
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        if state.current_player_index.saturating_add(1) == state.players.len() {
            finish_team_round(state, 0)?;
        }
        Ok(())
    }
}

fn spawn_wave(state: &mut RegisteredGameState, wave: u64) -> Result<(), GameError> {
    let mut fleet = ships(state)?;
    let excluded = fleet
        .iter()
        .map(|ship| parse_target(&ship.target).map(|target| zone_id(&target)))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let kinds = wave_types(state, wave)?;
    let mut available = target_pool("normal")
        .into_iter()
        .filter(|target| !excluded.contains(&zone_id(target)))
        .collect::<Vec<_>>();
    for kind in kinds {
        if available.is_empty() {
            return Err(invalid_state());
        }
        let index = state.random_index(available.len())?;
        let target = available.remove(index);
        let ship_id = counter(state, "next_ship_id")?;
        let (hp, points) = ship_stats(kind)?;
        fleet.push(Ship {
            id: format!("ship-{ship_id}"),
            kind: kind.into(),
            target: target_value(&target),
            hp,
            max_hp: hp,
            points,
        });
        state.mode_state["next_ship_id"] = Value::from(ship_id.saturating_add(1));
    }
    set_ships(state, &fleet)?;
    state.mode_state["wave"] = Value::from(wave);
    state.message = format!("Welle {wave} ist gelandet!");
    Ok(())
}

fn wave_types(state: &RegisteredGameState, wave: u64) -> Result<Vec<&'static str>, GameError> {
    let count = (2_usize.saturating_add(state.players.len() / 2)).clamp(3, 6);
    let maximum = maximum_waves(state)?;
    if wave >= maximum {
        let mut kinds = vec!["boss"];
        kinds.extend(std::iter::repeat_n("scout", count.saturating_sub(1).max(2)));
        return Ok(kinds);
    }
    if wave == 1 {
        return Ok(vec!["scout"; count]);
    }
    if wave.is_multiple_of(3) {
        let mut kinds = vec!["cruiser", "fighter", "fighter"];
        kinds.extend(std::iter::repeat_n("scout", count.saturating_sub(3)));
        return Ok(kinds);
    }
    let mut kinds = vec!["fighter", "fighter"];
    kinds.extend(std::iter::repeat_n("scout", count.saturating_sub(2).max(1)));
    Ok(kinds)
}

fn finish_team_round(state: &mut RegisteredGameState, points: i64) -> Result<(), GameError> {
    let wave = counter(state, "wave")?;
    let maximum = maximum_waves(state)?;
    if wave >= maximum {
        if ships(state)?.is_empty() {
            finish_team(state, true, "ERDE GERETTET! Das Team gewinnt!", points);
        } else if flag(state, "cleanup")? {
            finish_team(
                state,
                false,
                "Die Flotte entkommt · Team-Niederlage",
                points,
            );
        } else {
            state.mode_state["cleanup"] = Value::from(true);
            state.message = "LETZTE AUFRÄUMRUNDE!".into();
        }
        return Ok(());
    }

    spawn_wave(state, wave.saturating_add(1))?;
    set_effect(state, "space_wave", 0, 0, 0);
    if ships(state)?.len() >= 10 {
        finish_team(
            state,
            false,
            "INVASION! Zehn Schiffe haben die Erde erreicht",
            points,
        );
    }
    Ok(())
}

fn finish_team(state: &mut RegisteredGameState, won: bool, message: &str, points: i64) {
    state.status = GameStatus::Finished;
    state.winner_id = None;
    state.winner_ids = if won {
        state
            .players
            .iter()
            .map(|player| player.id.clone())
            .collect()
    } else {
        Vec::new()
    };
    state.result_type = if won { "team_win" } else { "challenge_loss" }.into();
    state.message = message.into();
    state.mode_state["last_effect"] = Value::from(if won { "space_win" } else { "space_invasion" });
    state.mode_state["effect_points"] = Value::from(points);
}

fn damage_ship(ship: &mut Ship, damage: u8) -> i64 {
    ship.hp = ship.hp.saturating_sub(damage);
    if ship.hp == 0 { ship.points } else { 0 }
}

fn overlay(state: &RegisteredGameState) -> Result<Value, GameError> {
    let fleet = ships(state)?;
    let wave = counter(state, "wave")?;
    let targets = fleet
        .iter()
        .map(|ship| {
            let target = parse_target(&ship.target)?;
            Ok(json!({
                "id": zone_id(&target), "field": target.field, "ring": target.ring,
                "color": "green", "label": format!("{} HP", ship.hp), "pulse": true,
            }))
        })
        .collect::<Result<Vec<_>, GameError>>()?;
    let rows = fleet
        .iter()
        .take(6)
        .map(|ship| {
            Ok(json!({
                "label": format!("{} · {}", title(&ship.kind), parse_target(&ship.target)?.label),
                "value": format!("{}/{} HP", ship.hp, ship.max_hp),
            }))
        })
        .collect::<Result<Vec<_>, GameError>>()?;
    Ok(json!({
        "prompt": if flag(state, "cleanup")? { "Aufräumrunde!".into() } else { format!("Welle {wave} verteidigen!") },
        "targets": targets,
        "panel": {
            "title": "SPACE DEFENDER", "headline": format!("Welle {wave} · {} Schiffe", fleet.len()),
            "subline": "Bei 10 aktiven Schiffen ist die Erde verloren",
            "progress": {"value": fleet.len(), "max": 10}, "rows": rows,
        },
    }))
}

fn ships(state: &RegisteredGameState) -> Result<Vec<Ship>, GameError> {
    serde_json::from_value(state.mode_state["ships"].clone()).map_err(|_| invalid_state())
}

fn set_ships(state: &mut RegisteredGameState, ships: &[Ship]) -> Result<(), GameError> {
    state.mode_state["ships"] = serde_json::to_value(ships).map_err(|_| invalid_state())?;
    Ok(())
}

fn maximum_waves(state: &RegisteredGameState) -> Result<u64, GameError> {
    state
        .options
        .get("waves")
        .and_then(Value::as_u64)
        .ok_or_else(invalid_state)
}

fn counter(state: &RegisteredGameState, key: &str) -> Result<u64, GameError> {
    state.mode_state[key].as_u64().ok_or_else(invalid_state)
}

fn flag(state: &RegisteredGameState, key: &str) -> Result<bool, GameError> {
    state.mode_state[key].as_bool().ok_or_else(invalid_state)
}

fn clear_effect(state: &mut RegisteredGameState) {
    set_effect(state, "", 0, 0, 0);
}

fn set_effect(
    state: &mut RegisteredGameState,
    effect: &str,
    points: i64,
    damage: u8,
    destroyed: u64,
) {
    state.mode_state["last_effect"] = Value::from(effect);
    state.mode_state["effect_points"] = Value::from(points);
    state.mode_state["effect_damage"] = Value::from(damage);
    state.mode_state["destroyed"] = Value::from(destroyed);
}

fn ship_stats(kind: &str) -> Result<(u8, i64), GameError> {
    match kind {
        "scout" => Ok((1, 10)),
        "fighter" => Ok((2, 25)),
        "cruiser" => Ok((3, 50)),
        "boss" => Ok((5, 100)),
        _ => Err(invalid_state()),
    }
}

fn title(kind: &str) -> &str {
    match kind {
        "scout" => "Scout",
        "fighter" => "Fighter",
        "cruiser" => "Cruiser",
        "boss" => "Boss",
        _ => kind,
    }
}

const fn choice(value: i64, label: &'static str) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Integer(value),
        label,
        description: None,
        description_en: None,
    }
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid space defender state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;
    use sdb_contracts::{PlayerRef, Ring};

    fn players() -> Vec<PlayerRef> {
        vec![
            PlayerRef {
                id: "ada".into(),
                name: "Ada".into(),
                avatar: "fox".into(),
                color: "#ff00aa".into(),
                team_id: None,
            },
            PlayerRef {
                id: "bob".into(),
                name: "Bob".into(),
                avatar: "comet".into(),
                color: "#00ffaa".into(),
                team_id: None,
            },
        ]
    }

    fn game() -> RegisteredGame {
        RegisteredGame::new_seeded_with_players(
            "space_defender",
            players(),
            &json!({"waves":4}),
            42,
        )
        .expect("game")
    }

    fn event_for(target: &Value, seq: u64) -> DartEvent {
        serde_json::from_value(json!({
            "type":"hit", "seq":seq, "field":target["field"], "ring":target["ring"],
            "multiplier":target["multiplier"], "label":target["label"], "score":target["score"],
        }))
        .expect("event")
    }

    #[test]
    fn seeded_scout_kill_awards_the_entire_team() {
        let mut game = game();
        let target = game.state().mode_state["ships"][0]["target"].clone();
        game.apply_throw(&event_for(&target, 1)).expect("throw");
        assert_eq!(
            game.state()
                .players
                .iter()
                .map(|player| player.score)
                .collect::<Vec<_>>(),
            [10, 10]
        );
        assert_eq!(
            game.state().mode_state["ships"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(game.state().mode_state["last_effect"], "space_destroy");
    }

    #[test]
    fn bull_laser_damages_every_ship() {
        let mut game = game();
        game.apply_throw(&DartEvent::Hit {
            seq: 1,
            field: 25,
            ring: Ring::SingleBull,
            multiplier: 1,
            label: "SBull".into(),
            score: 25,
        })
        .expect("laser");
        assert!(
            game.state().mode_state["ships"]
                .as_array()
                .is_some_and(Vec::is_empty)
        );
        assert_eq!(game.state().players[0].score, 30);
        assert_eq!(game.state().players[1].score, 30);
        assert_eq!(game.state().mode_state["destroyed"], 3);
    }

    #[test]
    fn skipped_team_rounds_advance_and_trigger_invasion() {
        let mut game = game();
        for expected_wave in [2, 3] {
            game.next_player().expect("skip Ada");
            game.next_player().expect("skip Bob");
            assert_eq!(game.state().mode_state["wave"], expected_wave);
        }
        game.next_player().expect("skip Ada");
        game.next_player().expect("skip Bob");
        assert_eq!(game.state().status, GameStatus::Finished);
        assert_eq!(game.state().result_type, "challenge_loss");
        assert!(game.state().winner_ids.is_empty());
    }

    #[test]
    fn final_wave_gets_exactly_one_cleanup_round() {
        let mut game = game();
        game.state.mode_state["wave"] = Value::from(4);
        game.state.mode_state["ships"] =
            Value::Array(vec![game.state.mode_state["ships"][0].clone()]);
        game.next_player().expect("skip Ada");
        game.next_player().expect("skip Bob");
        assert_eq!(game.state().mode_state["cleanup"], true);
        assert_eq!(game.state().status, GameStatus::Running);
        game.next_player().expect("cleanup Ada");
        game.next_player().expect("cleanup Bob");
        assert_eq!(game.state().status, GameStatus::Finished);
        assert_eq!(game.state().result_type, "challenge_loss");
    }
}
