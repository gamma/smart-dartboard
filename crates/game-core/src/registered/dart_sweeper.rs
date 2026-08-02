use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
};
use crate::{GameError, GameStatus};
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const BOARD_ORDER: [u8; 20] = [
    20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5,
];
const ALL_RINGS: [Ring; 4] = [
    Ring::SingleInner,
    Ring::Triple,
    Ring::SingleOuter,
    Ring::Double,
];

static PRESET_CHOICES: [GameOptionChoice; 3] = [
    choice(
        "explorer",
        "Explorer · 3 Minen / 5 Leben",
        "Wenige Minen und fünf gemeinsame Fehler – gut zum Kennenlernen.",
        "Few mines and five shared mistakes—best for learning.",
    ),
    choice(
        "classic",
        "Classic · 5 Minen / 3 Leben",
        "Ausgewogenes Minenfeld mit drei gemeinsamen Leben.",
        "A balanced minefield with three shared lives.",
    ),
    choice(
        "expert",
        "Expert · 7 Minen / 2 Leben",
        "Viele Minen und nur zwei gemeinsame Fehler.",
        "Many mines and only two shared mistakes.",
    ),
];
static OPTIONS: [GameOption; 1] = [GameOption {
    key: "preset",
    label: "Schwierigkeit",
    kind: "choice",
    default: GameOptionValue::Text("classic"),
    choices: &PRESET_CHOICES,
}];
static INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Zahl aufdecken",
        body: "Singles decken genau die getroffene Zahl auf.",
        icon: "reveal",
    },
    GameInstruction {
        title: "Ring-Power",
        body: "Double deckt einen, Triple zwei sichere Nachbarn zusätzlich auf.",
        icon: "power",
    },
    GameInstruction {
        title: "Gemeinsam räumen",
        body: "Bull hilft beim Scannen. Räumt alle sicheren Zahlen vor dem letzten Leben.",
        icon: "mine",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "dart_sweeper",
    ruleset_version: 2,
    title: "DartSweeper",
    tagline: "Räumt gemeinsam das Minenfeld",
    description: "Die 20 Zahlen werden zu Minesweeper-Feldern. Double, Triple und Bull decken zusätzliche sichere Zahlen auf.",
    accent: "#5f8f71",
    accent_secondary: "#e9c46a",
    visual: "dart-sweeper",
    icon: "mine",
    artwork: "/static/assets/modes/dart_sweeper.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct DartSweeperMode;
pub(super) static DART_SWEEPER_MODE: DartSweeperMode = DartSweeperMode;

impl GameMode for DartSweeperMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let (mine_count, lives) = preset(state)?;
        state.mode_state = json!({
            "seeded": false, "direct_hit_seen": false, "mines": [], "revealed": {},
            "exploded": [], "lives": lives, "max_lives": lives, "mine_count": mine_count,
            "last_effect": "", "effect_points": 0, "effect_field": 0, "revealed_count": 0,
        });
        state.message = "Der erste Treffer ist garantiert sicher!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        set_effect(state, "", 0, 0, 0);
        match event {
            DartEvent::Miss { .. } => {
                state.message = "MISS · Das Minenfeld bleibt verdeckt".into();
                finish_if_needed(state, 0)?;
                Ok(0)
            }
            DartEvent::Hit {
                field: 25, ring, ..
            } => scan(state, *ring),
            DartEvent::Hit {
                field, multiplier, ..
            } => reveal_hit(state, *field, *multiplier),
        }
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        overlay(state).unwrap_or_else(|_| json!({}))
    }
}

fn scan(state: &mut RegisteredGameState, ring: Ring) -> Result<i64, GameError> {
    if !flag(state, "seeded", false)? {
        seed(state, None)?;
    }
    let mut safe = safe_covered(state, None)?;
    let reserve = if flag(state, "direct_hit_seen", true)? {
        0
    } else {
        3
    };
    let requested = if ring == Ring::DoubleBull { 2 } else { 1 };
    let amount = requested.min(safe.len().saturating_sub(reserve));
    let mut chosen = Vec::with_capacity(amount);
    for _ in 0..amount {
        let index = state.random_index(safe.len())?;
        chosen.push(safe.remove(index));
    }
    for field in &chosen {
        reveal(state, *field)?;
    }
    let points = i64::try_from(chosen.len())
        .map_err(|_| invalid_state())?
        .saturating_mul(5);
    award_team(state, points);
    set_effect(state, "sweeper_scan", points, 25, chosen.len());
    state.message = format!("BULL-SCANNER! {} sichere Felder +{points}", chosen.len());
    finish_if_needed(state, points)?;
    Ok(points)
}

fn reveal_hit(
    state: &mut RegisteredGameState,
    field: u8,
    multiplier: u8,
) -> Result<i64, GameError> {
    if !BOARD_ORDER.contains(&field) {
        state.message = "Das Feld liegt außerhalb des Minenrasters".into();
        return Ok(0);
    }
    if !flag(state, "seeded", false)? {
        seed(state, Some(field))?;
        state.mode_state["direct_hit_seen"] = Value::from(true);
    } else if !flag(state, "direct_hit_seen", true)? {
        ensure_first_direct_safe(state, field)?;
    }

    if mines(state)?.contains(&field) {
        let mut exploded = exploded(state)?;
        if exploded.insert(field) {
            set_exploded(state, &exploded);
            let lives = counter(state, "lives")?.saturating_sub(1);
            state.mode_state["lives"] = Value::from(lives);
            set_effect(state, "mine_explosion", 0, field, 0);
            state.message = format!("BOOM auf {field}! Noch {lives} Leben");
        } else {
            state.message = format!("Mine {field} ist bereits bekannt");
        }
        finish_if_needed(state, 0)?;
        return Ok(0);
    }

    if revealed(state)?.contains_key(&field.to_string()) {
        state.message = format!("{field} ist bereits aufgedeckt");
        finish_if_needed(state, 0)?;
        return Ok(0);
    }

    let adjacent = reveal(state, field)?;
    let mut points = if adjacent == 0 { 20_i64 } else { 10_i64 };
    let wanted = if multiplier == 3 {
        2
    } else {
        usize::from(multiplier == 2)
    };
    let available = safe_covered(state, Some(&neighbors(field)?))?;
    let bonus = wanted.min(available.len());
    for neighbor in available.into_iter().take(bonus) {
        reveal(state, neighbor)?;
    }
    points = points.saturating_add(
        i64::try_from(bonus)
            .map_err(|_| invalid_state())?
            .saturating_mul(5),
    );
    award_team(state, points);
    set_effect(state, "sweeper_reveal", points, field, 1 + bonus);
    state.message = format!(
        "{field} zeigt {adjacent} · {bonus} Bonusfeld{} +{points}",
        if bonus == 1 { "" } else { "er" }
    );
    finish_if_needed(state, points)?;
    Ok(points)
}

fn seed(state: &mut RegisteredGameState, safe_field: Option<u8>) -> Result<(), GameError> {
    let mut excluded = BTreeSet::new();
    if let Some(field) = safe_field {
        excluded.insert(field);
        excluded.extend(neighbors(field)?.into_iter().take(2));
    }
    let mut available = BOARD_ORDER
        .into_iter()
        .filter(|field| !excluded.contains(field))
        .collect::<Vec<_>>();
    let amount = usize::try_from(counter(state, "mine_count")?).map_err(|_| invalid_state())?;
    let mut selected = Vec::with_capacity(amount);
    for _ in 0..amount {
        let index = state.random_index(available.len())?;
        selected.push(available.remove(index));
    }
    selected.sort_unstable();
    state.mode_state["mines"] = serde_json::to_value(selected).map_err(|_| invalid_state())?;
    state.mode_state["seeded"] = Value::from(true);
    Ok(())
}

fn ensure_first_direct_safe(state: &mut RegisteredGameState, field: u8) -> Result<(), GameError> {
    let protected = std::iter::once(field)
        .chain(neighbors(field)?.into_iter().take(2))
        .collect::<BTreeSet<_>>();
    let mut mine_set = mines(state)?.into_iter().collect::<BTreeSet<_>>();
    let conflicts = mine_set
        .intersection(&protected)
        .copied()
        .collect::<Vec<_>>();
    let mut replacements = safe_covered(state, None)?
        .into_iter()
        .filter(|candidate| !protected.contains(candidate))
        .collect::<Vec<_>>();
    for mine in conflicts {
        mine_set.remove(&mine);
        if replacements.is_empty() {
            return Err(invalid_state());
        }
        let index = state.random_index(replacements.len())?;
        mine_set.insert(replacements.remove(index));
    }
    state.mode_state["mines"] = Value::Array(mine_set.into_iter().map(Value::from).collect());
    state.mode_state["direct_hit_seen"] = Value::from(true);
    let fields = revealed(state)?.keys().cloned().collect::<Vec<_>>();
    for key in fields {
        let revealed_field = key.parse::<u8>().map_err(|_| invalid_state())?;
        state.mode_state["revealed"][key] = Value::from(adjacent_mines(state, revealed_field)?);
    }
    Ok(())
}

fn neighbors(field: u8) -> Result<Vec<u8>, GameError> {
    let index = BOARD_ORDER
        .iter()
        .position(|candidate| *candidate == field)
        .ok_or_else(invalid_state)?;
    Ok(vec![
        BOARD_ORDER[(index + 19) % 20],
        BOARD_ORDER[(index + 1) % 20],
        BOARD_ORDER[(index + 18) % 20],
        BOARD_ORDER[(index + 2) % 20],
    ])
}

fn adjacent_mines(state: &RegisteredGameState, field: u8) -> Result<u8, GameError> {
    let mine_set = mines(state)?.into_iter().collect::<BTreeSet<_>>();
    u8::try_from(
        neighbors(field)?
            .iter()
            .filter(|neighbor| mine_set.contains(neighbor))
            .count(),
    )
    .map_err(|_| invalid_state())
}

fn safe_covered(
    state: &RegisteredGameState,
    candidates: Option<&[u8]>,
) -> Result<Vec<u8>, GameError> {
    let mine_set = mines(state)?.into_iter().collect::<BTreeSet<_>>();
    let revealed = revealed(state)?;
    Ok(candidates
        .unwrap_or(&BOARD_ORDER)
        .iter()
        .copied()
        .filter(|field| !mine_set.contains(field) && !revealed.contains_key(&field.to_string()))
        .collect())
}

fn reveal(state: &mut RegisteredGameState, field: u8) -> Result<u8, GameError> {
    if let Some(value) = state.mode_state["revealed"].get(field.to_string()) {
        return value
            .as_u64()
            .and_then(|value| u8::try_from(value).ok())
            .ok_or_else(invalid_state);
    }
    let adjacent = adjacent_mines(state, field)?;
    state.mode_state["revealed"][field.to_string()] = Value::from(adjacent);
    Ok(adjacent)
}

fn finish_if_needed(state: &mut RegisteredGameState, points: i64) -> Result<(), GameError> {
    let safe_total = 20_usize.saturating_sub(mines(state)?.len());
    if revealed(state)?.len() >= safe_total {
        finish_team(state, true, "MINENFELD GERÄUMT! Das Team gewinnt!");
        set_effect(state, "sweeper_win", points, 0, 0);
    } else if counter(state, "lives")? == 0 {
        finish_team(state, false, "BOOM! Keine Leben mehr · Team-Niederlage");
        if state.mode_state["last_effect"] != "mine_explosion" {
            set_effect(state, "mine_explosion", points, 0, 0);
        }
    }
    Ok(())
}

fn finish_team(state: &mut RegisteredGameState, won: bool, message: &str) {
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
}

fn award_team(state: &mut RegisteredGameState, points: i64) {
    for player in &mut state.players {
        player.score = player.score.saturating_add(points);
    }
}

fn overlay(state: &RegisteredGameState) -> Result<Value, GameError> {
    let revealed = revealed(state)?;
    let exploded = exploded(state)?;
    let zones = BOARD_ORDER
        .iter()
        .map(|field| {
            if exploded.contains(field) {
                json!({
                    "field": field, "rings": ALL_RINGS, "role": "mine", "label": "",
                    "icon": "mine", "variant": "mine", "color": "#e76f51",
                })
            } else if let Some(count) = revealed.get(&field.to_string()) {
                json!({
                    "field": field, "rings": ALL_RINGS, "role": "revealed",
                    "color": if *count <= 1 { "#70b77e" } else if *count <= 2 { "#e9c46a" } else { "#e76f51" },
                    "label": count.to_string(),
                })
            } else {
                json!({"field":field,"rings":ALL_RINGS,"role":"covered","label":"?"})
            }
        })
        .collect::<Vec<_>>();
    let safe_total = 20_usize.saturating_sub(
        usize::try_from(counter(state, "mine_count")?).map_err(|_| invalid_state())?,
    );
    let remaining = safe_total.saturating_sub(revealed.len());
    let lives = usize::try_from(counter(state, "lives")?).map_err(|_| invalid_state())?;
    let maximum = usize::try_from(counter(state, "max_lives")?).map_err(|_| invalid_state())?;
    Ok(json!({
        "prompt": "Single 1 Feld · Double +1 · Triple +2",
        "zones": zones,
        "panel": {
            "title": "DARTSWEEPER", "headline": format!("{remaining} sichere Felder übrig"),
            "subline": format!("{}{}", "♥".repeat(lives), "♡".repeat(maximum.saturating_sub(lives))),
            "progress": {"value": revealed.len(), "max": safe_total},
            "stats": [
                {"label":"MINEN","value":counter(state,"mine_count")?},
                {"label":"GEFUNDEN","value":exploded.len()},
            ],
        },
    }))
}

fn preset(state: &RegisteredGameState) -> Result<(u64, u64), GameError> {
    match state.options.get("preset").and_then(Value::as_str) {
        Some("explorer") => Ok((3, 5)),
        Some("classic") => Ok((5, 3)),
        Some("expert") => Ok((7, 2)),
        _ => Err(invalid_state()),
    }
}

fn mines(state: &RegisteredGameState) -> Result<Vec<u8>, GameError> {
    serde_json::from_value(state.mode_state["mines"].clone()).map_err(|_| invalid_state())
}

fn revealed(state: &RegisteredGameState) -> Result<BTreeMap<String, u8>, GameError> {
    serde_json::from_value(state.mode_state["revealed"].clone()).map_err(|_| invalid_state())
}

fn exploded(state: &RegisteredGameState) -> Result<BTreeSet<u8>, GameError> {
    serde_json::from_value(state.mode_state["exploded"].clone()).map_err(|_| invalid_state())
}

fn set_exploded(state: &mut RegisteredGameState, exploded: &BTreeSet<u8>) {
    state.mode_state["exploded"] =
        Value::Array(exploded.iter().copied().map(Value::from).collect());
}

fn counter(state: &RegisteredGameState, key: &str) -> Result<u64, GameError> {
    state.mode_state[key].as_u64().ok_or_else(invalid_state)
}

fn flag(state: &RegisteredGameState, key: &str, missing: bool) -> Result<bool, GameError> {
    match state.mode_state.get(key) {
        Some(value) => value.as_bool().ok_or_else(invalid_state),
        None => Ok(missing),
    }
}

fn set_effect(state: &mut RegisteredGameState, effect: &str, points: i64, field: u8, count: usize) {
    state.mode_state["last_effect"] = Value::from(effect);
    state.mode_state["effect_points"] = Value::from(points);
    state.mode_state["effect_field"] = Value::from(field);
    state.mode_state["revealed_count"] = Value::from(u64::try_from(count).unwrap_or(u64::MAX));
}

const fn choice(
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

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid dart sweeper state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;
    use sdb_contracts::PlayerRef;

    fn players() -> Vec<PlayerRef> {
        ["ada", "bob"]
            .into_iter()
            .map(|id| PlayerRef {
                id: id.into(),
                name: id.to_uppercase(),
                avatar: "comet".into(),
                color: "#28e7ff".into(),
            })
            .collect()
    }

    fn game(preset: &str) -> RegisteredGame {
        RegisteredGame::new_seeded_with_players(
            "dart_sweeper",
            players(),
            &json!({"preset":preset}),
            42,
        )
        .expect("game")
    }

    fn hit(field: u8, ring: Ring, multiplier: u8, seq: u64) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring,
            multiplier,
            label: if field == 25 {
                if multiplier == 2 { "DBull" } else { "SBull" }.into()
            } else {
                format!(
                    "{}{field}",
                    if multiplier == 3 {
                        "T"
                    } else if multiplier == 2 {
                        "D"
                    } else {
                        "S"
                    }
                )
            },
            score: u16::from(field) * u16::from(multiplier),
        }
    }

    #[test]
    fn first_triple_is_safe_and_reveals_two_neighbors_for_every_teammate() {
        let mut game = game("classic");
        game.apply_throw(&hit(20, Ring::Triple, 3, 1))
            .expect("throw");
        let mine_set = mines(&game.state).expect("mines");
        assert!(!mine_set.iter().any(|field| [20, 5, 1].contains(field)));
        assert_eq!(revealed(&game.state).expect("revealed").len(), 3);
        assert_eq!(game.state.players[0].score, game.state.players[1].score);
        assert!(game.state.players[0].score > 0);
    }

    #[test]
    fn miss_does_not_seed_or_consume_first_hit_safety() {
        let mut game = game("classic");
        game.apply_throw(&DartEvent::Miss {
            seq: 1,
            label: "MISS".into(),
            score: 0,
        })
        .expect("miss");
        assert_eq!(game.state.mode_state["seeded"], false);
        game.apply_throw(&hit(20, Ring::SingleOuter, 1, 2))
            .expect("hit");
        assert!(
            mines(&game.state)
                .expect("mines")
                .iter()
                .all(|field| ![20, 5, 1].contains(field))
        );
    }

    #[test]
    fn bull_before_direct_hit_preserves_the_safe_halo() {
        let mut game = game("expert");
        game.apply_throw(&hit(25, Ring::DoubleBull, 2, 1))
            .expect("scan");
        game.apply_throw(&hit(20, Ring::SingleOuter, 1, 2))
            .expect("direct");
        assert!(
            mines(&game.state)
                .expect("mines")
                .iter()
                .all(|field| ![20, 5, 1].contains(field))
        );
    }

    #[test]
    fn direct_mine_explodes_once_even_with_a_multiplier() {
        let mut game = game("classic");
        game.state.mode_state["seeded"] = Value::from(true);
        game.state.mode_state["direct_hit_seen"] = Value::from(true);
        game.state.mode_state["mines"] = json!([1, 2, 3, 4, 20]);
        game.apply_throw(&hit(20, Ring::Triple, 3, 1))
            .expect("mine");
        game.apply_throw(&hit(20, Ring::SingleOuter, 1, 2))
            .expect("known mine");
        assert_eq!(game.state.mode_state["lives"], 2);
        assert_eq!(game.state.mode_state["exploded"], json!([20]));
        assert!(revealed(&game.state).expect("revealed").is_empty());
    }
}
