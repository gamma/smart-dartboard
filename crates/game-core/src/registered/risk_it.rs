use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState, arcade::ring_name, finish_action_round_game, finish_fixed_round_game,
    finish_score_game,
};
use crate::{GameError, GameStatus};
use sdb_contracts::{DartEvent, Ring};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

static ROUND_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(3, "3 Runden"),
    choice_integer(5, "5 Runden"),
    choice_integer(8, "8 Runden"),
];
static MISS_CHOICES: [GameOptionChoice; 2] = [
    GameOptionChoice {
        value: GameOptionValue::Text("pot"),
        label: "Pot verlieren",
        description: Some("Ein Miss löscht den gesamten eigenen ungesicherten Pot."),
        description_en: Some("A miss removes your entire unsecured pot."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("half"),
        label: "Pot halbieren",
        description: Some(
            "Ein Miss halbiert den eigenen ungesicherten Pot; bei Dart 3 wird die Hälfte gesichert.",
        ),
        description_en: Some("A miss halves your unsecured pot; on dart 3, half is secured."),
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
        key: "miss_loses",
        label: "Miss",
        kind: "choice",
        default: GameOptionValue::Text("pot"),
        choices: &MISS_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 4] = [
    GameInstruction {
        title: "Pot füllen",
        body: "Jeder Treffer erhöht deinen ungesicherten Pot.",
        icon: "pot",
    },
    GameInstruction {
        title: "Nach Dart 1 oder 2 banken",
        body: "BANK sichert den Pot und beendet deinen Zug.",
        icon: "bank",
    },
    GameInstruction {
        title: "Dart 3 ist Risiko",
        body: "Ein Treffer macht seine Zahl zum leuchtenden Hot-Pot-Ziel.",
        icon: "risk",
    },
    GameInstruction {
        title: "Erster Dart kann stehlen",
        body: "Der nächste Spieler trifft die Zahl und stiehlt den Pot. Sonst wird er für dich gesichert.",
        icon: "target",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "risk_it",
    ruleset_version: 3,
    title: "Risk It",
    tagline: "Banken oder den Hot Pot riskieren",
    description: "Treffer füllen deinen Pot. Banke nach Dart 1 oder 2 – nach Dart 3 kann der nächste Spieler den Pot mit einem Treffer stehlen.",
    accent: "#ffb52b",
    accent_secondary: "#ff4f79",
    visual: "risk-it",
    icon: "dice",
    artwork: "/static/assets/modes/risk_it.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct HotPot {
    owner_id: String,
    amount: i64,
    field: u8,
    label: String,
}

pub(super) struct RiskItMode;
pub(super) static RISK_IT_MODE: RiskItMode = RiskItMode;

impl GameMode for RiskItMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({
            "pot": {},
            "banked_last": 0,
            "hot_pot": null,
            "final_heist": false,
            "last_effect": "",
            "effect_amount": 0,
            "effect_target_player_id": null,
        });
        state.message = "Risk It: Pot füllen, banken oder Dart 3 riskieren!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        clear_effect(state);
        let final_heist = state.mode_state["final_heist"]
            .as_bool()
            .ok_or_else(invalid_state)?;
        let player_index = state.current_player_index;
        let (stolen, hot_message) = resolve_hot_pot(state, player_index, Some(event))?;
        if final_heist {
            state.mode_state["final_heist"] = Value::from(false);
            finish_score_game(state, "{winner} gewinnt Risk It!")?;
            return Ok(stolen);
        }

        let player_id = state
            .players
            .get(player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        let mut current_pot = pot(state, &player_id)?;
        let is_last_dart = state.darts_in_turn == 2;
        let turn_value = match event {
            DartEvent::Miss { .. } => {
                if miss_rule(state)? == "half" {
                    let next_pot = current_pot / 2;
                    if is_last_dart {
                        state.players[player_index].score =
                            state.players[player_index].score.saturating_add(next_pot);
                        state.mode_state["banked_last"] = Value::from(next_pot);
                        set_pot(state, &player_id, 0)?;
                        set_effect(state, "risk_bank", next_pot, Some(&player_id));
                        state.message = join_messages(
                            &hot_message,
                            &format!("Miss · halber Pot gesichert +{next_pot}"),
                        );
                    } else {
                        set_pot(state, &player_id, next_pot)?;
                        set_effect(state, "risk_half", next_pot, Some(&player_id));
                        state.message = join_messages(
                            &hot_message,
                            &format!("Miss · Pot halbiert auf {next_pot}"),
                        );
                    }
                    stolen
                } else {
                    set_pot(state, &player_id, 0)?;
                    set_effect(state, "risk_pot_lost", current_pot, Some(&player_id));
                    state.message = join_messages(&hot_message, "Miss · eigener Pot verloren");
                    state.status = GameStatus::Hold;
                    finish_action_round_game(state, "{winner} gewinnt Risk It!")?;
                    stolen
                }
            }
            DartEvent::Hit { score, field, .. } => {
                let dart_score = i64::from(*score);
                current_pot = current_pot.saturating_add(dart_score);
                set_pot(state, &player_id, current_pot)?;
                if is_last_dart {
                    if state.players.len() == 1 {
                        state.players[player_index].score = state.players[player_index]
                            .score
                            .saturating_add(current_pot);
                        state.mode_state["banked_last"] = Value::from(current_pot);
                        set_pot(state, &player_id, 0)?;
                        set_effect(state, "risk_bank", current_pot, Some(&player_id));
                        state.message =
                            join_messages(&hot_message, &format!("Solo Auto-Bank +{current_pot}"));
                    } else {
                        make_hot_pot(state, &player_id, *field, current_pot)?;
                        let label = hot_pot(state)?
                            .map(|hot| hot.label)
                            .ok_or_else(invalid_state)?;
                        state.message = join_messages(
                            &hot_message,
                            &format!("HOT POT {current_pot} · Ziel {label}"),
                        );
                        let final_player = player_index.saturating_add(1) == state.players.len();
                        if final_player && state.round_number >= rounds(state)? {
                            state.mode_state["final_heist"] = Value::from(true);
                        }
                    }
                } else {
                    state.message = join_messages(
                        &hot_message,
                        &format!("Pot {current_pot} · BANK oder weiter?"),
                    );
                }
                dart_score
            }
        };

        if state.mode_state["final_heist"].as_bool() != Some(true)
            && state.status != GameStatus::Finished
        {
            finish_fixed_round_game(state, "{winner} gewinnt Risk It!")?;
        }
        Ok(turn_value)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(player) = state.players.get(state.current_player_index) else {
            return json!({"prompt": "Risk It", "targets": [], "actions": []});
        };
        let current_pot = pot(state, &player.id).unwrap_or_default();
        let hot = hot_pot(state).ok().flatten();
        let attack_open = hot
            .as_ref()
            .is_some_and(|hot| hot.owner_id != player.id && state.darts_in_turn == 0);
        let (prompt, panel) = if attack_open {
            let hot = hot.as_ref().expect("hot pot");
            let owner_name =
                player_by_id(state, &hot.owner_id).map_or("—", |(_, owner)| owner.name.as_str());
            (
                format!("TRIFF {} MIT DART 1 · STIEHL {}", hot.label, hot.amount),
                json!({
                    "title": "HOT POT",
                    "headline": format!("{} PUNKTE", hot.amount),
                    "subline": format!("{}: Triff {} mit Dart 1", player.name, hot.label),
                    "stats": [
                        {"label": "BESITZER", "value": owner_name},
                        {"label": "DIEBSTAHL-ZIEL", "value": hot.label},
                    ],
                }),
            )
        } else if let Some(hot) = hot.as_ref() {
            let next_name = state
                .players
                .get((state.current_player_index + 1) % state.players.len())
                .map_or("—", |player| player.name.as_str());
            (
                format!("HOT POT {} · ZIEL {}", hot.amount, hot.label),
                json!({
                    "title": "HOT POT",
                    "headline": format!("{} PUNKTE", hot.amount),
                    "subline": format!("{next_name} kann mit Dart 1 auf {} stehlen", hot.label),
                    "stats": [{"label": "DIEBSTAHL-ZIEL", "value": hot.label}],
                }),
            )
        } else {
            let decision = if state.darts_in_turn == 2 && current_pot > 0 {
                "BANKEN ODER DART 3 RISKIEREN"
            } else if current_pot > 0 {
                "BANKEN ODER WEITERWERFEN"
            } else {
                "TREFFER FÜLLEN DEINEN POT"
            };
            (
                format!("POT {current_pot} · {decision}"),
                json!({
                    "title": "UNGESICHERTER POT",
                    "headline": current_pot.to_string(),
                    "subline": decision,
                }),
            )
        };
        let can_bank = state.status == GameStatus::Running
            && matches!(state.darts_in_turn, 1 | 2)
            && current_pot > 0;
        json!({
            "prompt": prompt,
            "bonus": [],
            "targets": if attack_open { hot.as_ref().map_or_else(Vec::new, target_items) } else { Vec::new() },
            "danger": [],
            "pot": current_pot,
            "hot_pot": hot,
            "panel": panel,
            "actions": if can_bank {
                vec![json!({"id": "bank", "label": format!("BANK +{current_pot}"), "enabled": true})]
            } else {
                Vec::new()
            },
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let player_index = state.current_player_index;
        let player_id = state
            .players
            .get(player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        if hot_pot(state)?
            .as_ref()
            .is_some_and(|hot| hot.owner_id == player_id)
        {
            resolve_hot_pot(state, player_index, None)?;
        }
        Ok(())
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        clear_effect(state);
        let player_index = state.current_player_index;
        let player_id = state
            .players
            .get(player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        let player_name = state.players[player_index].name.clone();
        let (_, hot_message) = resolve_hot_pot(state, player_index, None)?;
        let own_pot = pot(state, &player_id)?;
        set_pot(state, &player_id, 0)?;
        state.message = join_messages(
            &hot_message,
            &format!("{player_name} überspringt · Pot {own_pot} verloren"),
        );
        if state.mode_state["last_effect"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
        {
            set_effect(state, "risk_skip", own_pot, Some(&player_id));
        }
        if state.mode_state["final_heist"].as_bool() == Some(true) {
            state.mode_state["final_heist"] = Value::from(false);
            finish_score_game(state, "{winner} gewinnt Risk It!")?;
        }
        Ok(())
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} gewinnt Risk It!")
    }

    fn handle_action(
        &self,
        state: &mut RegisteredGameState,
        action: &str,
        _payload: &Value,
    ) -> Result<(), GameError> {
        if action != "bank" {
            return Err(GameError::UnsupportedAction(action.into()));
        }
        if state.status != GameStatus::Running {
            return Err(GameError::NotRunning);
        }
        let player_index = state.current_player_index;
        let player_id = state
            .players
            .get(player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        let current_pot = pot(state, &player_id)?;
        if !matches!(state.darts_in_turn, 1 | 2) || current_pot <= 0 {
            return Err(GameError::InvalidOptions(
                "the pot can only be banked after dart 1 or 2".into(),
            ));
        }
        state.players[player_index].score = state.players[player_index]
            .score
            .saturating_add(current_pot);
        state.mode_state["banked_last"] = Value::from(current_pot);
        set_pot(state, &player_id, 0)?;
        set_effect(state, "risk_bank", current_pot, Some(&player_id));
        state.status = GameStatus::Hold;
        state.message = format!("{} bankt +{current_pot}", state.players[player_index].name);
        finish_action_round_game(state, "{winner} gewinnt Risk It!")
    }
}

fn resolve_hot_pot(
    state: &mut RegisteredGameState,
    attacker_index: usize,
    event: Option<&DartEvent>,
) -> Result<(i64, String), GameError> {
    let Some(hot) = hot_pot(state)? else {
        return Ok((0, String::new()));
    };
    let Some((owner_index, owner)) = player_by_id(state, &hot.owner_id) else {
        state.mode_state["hot_pot"] = Value::Null;
        return Ok((0, String::new()));
    };
    let owner_name = owner.name.clone();
    let owner_id = owner.id.clone();
    let attacker = state
        .players
        .get(attacker_index)
        .ok_or(GameError::NoPlayers)?;
    let attacker_id = attacker.id.clone();
    let attacker_name = attacker.name.clone();
    let stolen = owner_id != attacker_id
        && matches!(event, Some(DartEvent::Hit { field, .. }) if *field == hot.field);
    set_pot(state, &owner_id, 0)?;
    state.mode_state["hot_pot"] = Value::Null;
    state.mode_state["banked_last"] = Value::from(hot.amount);
    if stolen {
        state.players[attacker_index].score = state.players[attacker_index]
            .score
            .saturating_add(hot.amount);
        set_effect(state, "risk_steal", hot.amount, Some(&owner_id));
        Ok((
            hot.amount,
            format!(
                "HEIST! {attacker_name} stiehlt {} von {owner_name}",
                hot.amount
            ),
        ))
    } else {
        state.players[owner_index].score =
            state.players[owner_index].score.saturating_add(hot.amount);
        set_effect(state, "risk_secured", hot.amount, Some(&owner_id));
        Ok((0, format!("SAFE! {owner_name} bankt {}", hot.amount)))
    }
}

fn make_hot_pot(
    state: &mut RegisteredGameState,
    owner_id: &str,
    field: u8,
    amount: i64,
) -> Result<(), GameError> {
    let hot = HotPot {
        owner_id: owner_id.into(),
        amount,
        field,
        label: if field == 25 {
            "BULL".into()
        } else {
            field.to_string()
        },
    };
    state.mode_state["hot_pot"] = serde_json::to_value(hot).map_err(|_| invalid_state())?;
    set_effect(state, "risk_hot_pot", amount, Some(owner_id));
    Ok(())
}

fn hot_pot(state: &RegisteredGameState) -> Result<Option<HotPot>, GameError> {
    let value = &state.mode_state["hot_pot"];
    if value.is_null() {
        return Ok(None);
    }
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|_| invalid_state())
}

fn player_by_id<'a>(
    state: &'a RegisteredGameState,
    player_id: &str,
) -> Option<(usize, &'a super::RegisteredPlayer)> {
    state
        .players
        .iter()
        .enumerate()
        .find(|(_, player)| player.id == player_id)
}

fn pot(state: &RegisteredGameState, player_id: &str) -> Result<i64, GameError> {
    let pots = state.mode_state["pot"]
        .as_object()
        .ok_or_else(invalid_state)?;
    Ok(pots.get(player_id).and_then(Value::as_i64).unwrap_or(0))
}

fn set_pot(state: &mut RegisteredGameState, player_id: &str, value: i64) -> Result<(), GameError> {
    state.mode_state["pot"]
        .as_object_mut()
        .ok_or_else(invalid_state)?
        .insert(player_id.into(), Value::from(value.max(0)));
    Ok(())
}

fn clear_effect(state: &mut RegisteredGameState) {
    state.mode_state["last_effect"] = Value::from("");
    state.mode_state["effect_amount"] = Value::from(0);
    state.mode_state["effect_target_player_id"] = Value::Null;
}

fn set_effect(
    state: &mut RegisteredGameState,
    effect: &str,
    amount: i64,
    target_player_id: Option<&str>,
) {
    state.mode_state["last_effect"] = Value::from(effect);
    state.mode_state["effect_amount"] = Value::from(amount);
    state.mode_state["effect_target_player_id"] = target_player_id.map_or(Value::Null, Value::from);
}

fn target_items(hot: &HotPot) -> Vec<Value> {
    let rings: &[Ring] = if hot.field == 25 {
        &[Ring::SingleBull, Ring::DoubleBull]
    } else {
        &[
            Ring::SingleInner,
            Ring::Triple,
            Ring::SingleOuter,
            Ring::Double,
        ]
    };
    rings
        .iter()
        .enumerate()
        .map(|(index, ring)| {
            json!({
                "id": format!("HEIST-{}-{}", hot.field, ring_name(*ring)),
                "field": hot.field,
                "ring": ring,
                "color": "#ff4f79",
                "label": if index == 0 { "STEAL" } else { "" },
                "pulse": true,
            })
        })
        .collect()
}

fn miss_rule(state: &RegisteredGameState) -> Result<&str, GameError> {
    state.options["miss_loses"]
        .as_str()
        .ok_or_else(invalid_state)
}

fn rounds(state: &RegisteredGameState) -> Result<u16, GameError> {
    state.options["rounds"]
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(invalid_state)
}

fn join_messages(first: &str, second: &str) -> String {
    [first, second]
        .into_iter()
        .filter(|message| !message.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
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
    GameError::RulesetUnavailable("invalid risk_it mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    fn game(players: usize, miss_loses: &str) -> RegisteredGame {
        RegisteredGame::new_seeded(
            "risk_it",
            (0..players)
                .map(|index| (format!("p{index}"), format!("Player {index}")))
                .collect(),
            &json!({"rounds": 3, "miss_loses": miss_loses}),
            42,
        )
        .expect("game")
    }

    fn hit(seq: u64, field: u8, score: u16) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring: Ring::SingleOuter,
            multiplier: 1,
            label: format!("S{field}"),
            score,
        }
    }

    fn miss(seq: u64) -> DartEvent {
        DartEvent::Miss {
            seq,
            label: "MISS".into(),
            score: 0,
        }
    }

    #[test]
    fn bank_action_scores_and_is_undoable() {
        let mut game = game(1, "pot");
        game.apply_throw(&hit(1, 20, 60)).expect("hit");
        game.handle_action("bank", &json!({})).expect("bank");
        assert_eq!(game.state().players[0].score, 60);
        assert_eq!(game.state().status, GameStatus::Hold);
        assert_eq!(game.state().mode_state["last_effect"], "risk_bank");
        game.undo().expect("undo bank");
        assert_eq!(game.state().players[0].score, 0);
        assert_eq!(pot(game.state(), "p0").expect("pot"), 60);
    }

    #[test]
    fn first_dart_can_steal_hot_pot_and_still_starts_own_pot() {
        let mut game = game(2, "pot");
        for seq in 1..=3 {
            game.apply_throw(&hit(seq, 20, 20)).expect("owner hit");
        }
        game.continue_turn().expect("attacker");
        assert_eq!(
            game.state().overlay["targets"].as_array().map(Vec::len),
            Some(4)
        );
        game.apply_throw(&hit(4, 20, 5)).expect("heist");
        assert_eq!(game.state().players[1].score, 60);
        assert_eq!(pot(game.state(), "p1").expect("pot"), 5);
        assert_eq!(game.state().turn_score, 5);
        assert_eq!(game.state().mode_state["last_effect"], "risk_steal");
    }

    #[test]
    fn failed_or_skipped_heist_secures_the_owner() {
        let mut failed = game(2, "pot");
        for seq in 1..=3 {
            failed.apply_throw(&hit(seq, 20, 20)).expect("owner hit");
        }
        failed.continue_turn().expect("attacker");
        failed.apply_throw(&hit(4, 19, 19)).expect("failed heist");
        assert_eq!(failed.state().players[0].score, 60);
        assert_eq!(failed.state().mode_state["last_effect"], "risk_secured");

        let mut skipped = game(2, "pot");
        for seq in 1..=3 {
            skipped.apply_throw(&hit(seq, 20, 20)).expect("owner hit");
        }
        skipped.continue_turn().expect("attacker");
        skipped.next_player().expect("skip heist");
        assert_eq!(skipped.state().players[0].score, 60);
        assert!(hot_pot(skipped.state()).expect("hot").is_none());
    }

    #[test]
    fn miss_rules_hold_or_bank_half_on_dart_three() {
        let mut loses = game(1, "pot");
        loses.apply_throw(&hit(1, 20, 60)).expect("hit");
        loses.apply_throw(&miss(2)).expect("miss");
        assert_eq!(loses.state().status, GameStatus::Hold);
        assert_eq!(pot(loses.state(), "p0").expect("pot"), 0);

        let mut half = game(1, "half");
        half.apply_throw(&hit(1, 20, 60)).expect("hit one");
        half.apply_throw(&hit(2, 20, 40)).expect("hit two");
        half.apply_throw(&miss(3)).expect("miss three");
        assert_eq!(half.state().players[0].score, 50);
        assert_eq!(half.state().mode_state["last_effect"], "risk_bank");
    }

    #[test]
    fn final_hot_pot_gets_exactly_one_extra_heist_dart() {
        let mut game = game(2, "pot");
        game.next_player().expect("skip p0 round1");
        game.next_player().expect("skip p1 round1");
        game.next_player().expect("skip p0 round2");
        game.next_player().expect("skip p1 round2");
        game.next_player().expect("skip p0 round3");
        for seq in 1..=3 {
            game.apply_throw(&hit(seq, 20, 2)).expect("final owner hit");
        }
        assert_eq!(game.state().status, GameStatus::Hold);
        assert_eq!(game.state().mode_state["final_heist"], true);
        game.continue_turn().expect("final attacker");
        game.apply_throw(&hit(4, 20, 1)).expect("final heist");
        assert_eq!(game.state().status, GameStatus::Finished);
        assert_eq!(game.state().players[0].score, 6);
        assert_eq!(game.state().darts_in_turn, 1);
    }

    #[test]
    fn invalid_bank_and_solo_third_dart_follow_the_spec() {
        let mut invalid = game(1, "pot");
        assert!(matches!(
            invalid.handle_action("bank", &json!({})),
            Err(GameError::InvalidOptions(_))
        ));

        let mut solo = game(1, "pot");
        for seq in 1..=3 {
            solo.apply_throw(&hit(seq, 20, 20)).expect("solo hit");
        }
        assert_eq!(solo.state().players[0].score, 60);
        assert!(hot_pot(solo.state()).expect("hot").is_none());
    }
}
