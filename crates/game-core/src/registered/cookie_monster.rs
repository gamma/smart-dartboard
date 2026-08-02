use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState,
    arcade::{Target, parse_target, same_target, target_pool, target_value, zone_id},
    finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Map, Value, json};

static DIFFICULTY_CHOICES: [GameOptionChoice; 3] = [
    option(
        "easy",
        "Einfach · Snack Time",
        "15 große Zahlenfelder; jeder Ring isst den Cookie. Bull gibt +30.",
    ),
    option(
        "normal",
        "Mittel · Cookie Hunt",
        "12 exakte Cookies mit Gold und Schimmel. Bull verdoppelt oder rettet den Zug.",
    ),
    option(
        "hard",
        "Schwer · Sugar Rush",
        "12 exakte, farbige Cookies; Serien aktivieren Sugar Rush. Bull verdoppelt oder rettet.",
    ),
];
static ROUND_CHOICES: [GameOptionChoice; 2] = [integer(5, "5 Runden"), integer(8, "8 Runden")];
static OPTIONS: [GameOption; 2] = [
    GameOption {
        key: "difficulty",
        label: "Spielstufe",
        kind: "choice",
        default: GameOptionValue::Text("easy"),
        choices: &DIFFICULTY_CHOICES,
    },
    GameOption {
        key: "rounds",
        label: "Runden",
        kind: "choice",
        default: GameOptionValue::Integer(5),
        choices: &ROUND_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 4] = [
    GameInstruction {
        title: "Board leer essen",
        body: "Getroffene Cookies verschwinden für dich. Erst wenn alle weg sind, kommt ein neues Board.",
        icon: "cookie",
    },
    GameInstruction {
        title: "Schimmel meiden",
        body: "Schimmel kostet Punkte, muss aber nicht abgeräumt werden.",
        icon: "danger",
    },
    GameInstruction {
        title: "Bull ist Milch",
        body: "Easy gibt feste Bonuspunkte. Ab Mittel verdoppelt oder rettet Milch deinen Zug.",
        icon: "milk",
    },
    GameInstruction {
        title: "Stufe wählen",
        body: "Easy nutzt große Zahlenfelder; Mittel ergänzt Gold; Schwer ergänzt Farben und Sugar Rush.",
        icon: "combo",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "cookie_monster",
    ruleset_version: 2,
    title: "Cookie Monster",
    tagline: "Keksdose leer essen",
    description: "Räume dein persönliches Cookie-Board ab, meide Schimmel und schalte erst dann die nächste Keksdose frei.",
    accent: "#e9a23b",
    accent_secondary: "#68b0ab",
    visual: "cookie-monster",
    icon: "cookie",
    artwork: "/static/assets/modes/cookie_monster.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &[],
};

pub(super) struct CookieMonsterMode;
pub(super) static COOKIE_MONSTER_MODE: CookieMonsterMode = CookieMonsterMode;

impl GameMode for CookieMonsterMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({
            "streak": player_values(state, &0), "sugar": player_values(state, &false),
            "wave": player_values(state, &1), "collected": player_collections(state),
            "layouts": {}, "last_effect": "", "effect_points": 0, "cookie_wave": 1,
        });
        ensure_layout(state, 1)?;
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        set_effect(state, "", 0);
        let player_id = state
            .players
            .get(state.current_player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        let points = if matches!(event, DartEvent::Hit { field: 25, .. }) {
            milk(state, &player_id)?
        } else {
            cookie_hit(state, &player_id, event)?
        };
        state.players[state.current_player_index].score = state.players[state.current_player_index]
            .score
            .saturating_add(points);
        finish_fixed_round_game(state, "{winner} gewinnt die Keksdose!")?;
        Ok(points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(player) = state.players.get(state.current_player_index) else {
            return json!({});
        };
        overlay(state, &player.id).unwrap_or_else(|_| json!({}))
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        let player_id = state
            .players
            .get(state.current_player_index)
            .ok_or(GameError::NoPlayers)?
            .id
            .clone();
        ensure_layout(state, wave(state, &player_id)?)?;
        Ok(())
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} gewinnt die Keksdose!")
    }
}

fn milk(state: &mut RegisteredGameState, player_id: &str) -> Result<i64, GameError> {
    let adjustment = if difficulty(state)? == "easy" {
        30
    } else {
        state
            .turn_score
            .unsigned_abs()
            .try_into()
            .map_err(|_| invalid_state())?
    };
    set_counter(state, "streak", player_id, 0)?;
    set_effect(state, "cookie_milk", adjustment);
    state.message = if difficulty(state)? == "easy" {
        "MILCH! +30".into()
    } else if state.turn_score < 0 {
        format!("MILK! Turn gerettet {adjustment:+}")
    } else {
        format!("MILK! Turn verdoppelt +{adjustment}")
    };
    Ok(adjustment)
}

fn cookie_hit(
    state: &mut RegisteredGameState,
    player_id: &str,
    event: &DartEvent,
) -> Result<i64, GameError> {
    let current_wave = wave(state, player_id)?;
    ensure_layout(state, current_wave)?;
    let key = matching_cookie(state, current_wave, event)?;
    let Some(key) = key else {
        set_counter(state, "streak", player_id, 0)?;
        state.message = "Keine Krümel".into();
        return Ok(0);
    };
    let collected = collection(state, player_id)?;
    if collected.contains(&key) {
        set_counter(state, "streak", player_id, 0)?;
        state.message = "Hier ist schon alles aufgegessen".into();
        return Ok(0);
    }
    let kind = layout_item(state, current_wave, &key)?["kind"]
        .as_str()
        .ok_or_else(invalid_state)?
        .to_owned();
    if kind == "moldy" {
        set_counter(state, "streak", player_id, 0)?;
        let points = -mold_penalty(difficulty(state)?);
        set_effect(state, "cookie_moldy", points);
        state.message = format!("SCHIMMEL! {points}");
        return Ok(points);
    }
    eat_cookie(state, player_id, current_wave, key, &kind)
}

fn eat_cookie(
    state: &mut RegisteredGameState,
    player_id: &str,
    current_wave: u64,
    key: String,
    kind: &str,
) -> Result<i64, GameError> {
    let level = difficulty(state)?.to_owned();
    let base = cookie_points(&level, kind)?;
    let charged = level == "hard" && flag(state, "sugar", player_id)?;
    let points = base.saturating_mul(if charged { 2 } else { 1 });
    set_flag(state, "sugar", player_id, false)?;
    let mut streak = if level == "hard" {
        counter(state, "streak", player_id)?.saturating_add(1)
    } else {
        0
    };
    if level == "hard" && streak >= 3 {
        set_flag(state, "sugar", player_id, true)?;
        streak = 0;
    }
    set_counter(state, "streak", player_id, streak)?;
    let mut collected = collection(state, player_id)?;
    collected.push(key);
    set_collection(state, player_id, &collected)?;
    if remaining_good(state, current_wave, &collected)? == 0 {
        let next_wave = current_wave.saturating_add(1);
        state.mode_state["wave"][player_id] = Value::from(next_wave);
        set_collection(state, player_id, &[])?;
        ensure_layout(state, next_wave)?;
        state.mode_state["cookie_wave"] = Value::from(next_wave);
        set_effect(state, "cookie_board_clear", points);
        state.message = format!("BOARD GEPUTZT! +{points} · Neue Cookies!");
    } else {
        state.mode_state["cookie_wave"] = Value::from(current_wave);
        set_effect(state, "cookie_eaten", points);
        state.message = format!(
            "{} COOKIE +{points}{}",
            kind.to_uppercase(),
            if flag(state, "sugar", player_id)? {
                " · SUGAR RUSH GELADEN!"
            } else {
                ""
            }
        );
    }
    Ok(points)
}

fn ensure_layout(state: &mut RegisteredGameState, wave: u64) -> Result<(), GameError> {
    let key = wave.to_string();
    if state.mode_state["layouts"].get(&key).is_some() {
        return Ok(());
    }
    let level = difficulty(state)?.to_owned();
    let (targets, mut kinds): (Vec<Target>, Vec<&str>) = if level == "easy" {
        let mut fields = (1..=20).collect::<Vec<u8>>();
        let mut selected = Vec::with_capacity(15);
        for _ in 0..15 {
            let index = state.random_index(fields.len())?;
            selected.push(fields.remove(index));
        }
        (
            selected.into_iter().map(simple_target).collect(),
            [vec!["blue"; 12], vec!["moldy"; 3]].concat(),
        )
    } else {
        let mut pool = target_pool("normal")
            .into_iter()
            .filter(|target| target.field != 25)
            .collect::<Vec<_>>();
        let mut selected = Vec::with_capacity(12);
        for _ in 0..12 {
            let index = state.random_index(pool.len())?;
            selected.push(pool.remove(index));
        }
        let kinds = if level == "normal" {
            [vec!["gold"; 2], vec!["blue"; 7], vec!["moldy"; 3]].concat()
        } else {
            [
                vec!["gold"; 2],
                vec!["blue"; 3],
                vec!["green"; 4],
                vec!["moldy"; 3],
            ]
            .concat()
        };
        (selected, kinds)
    };
    for index in (1..kinds.len()).rev() {
        let swap = state.random_index(index + 1)?;
        kinds.swap(index, swap);
    }
    let mut layout = Map::new();
    for (target, kind) in targets.iter().zip(kinds) {
        let id = if level == "easy" {
            format!("F{}", target.field)
        } else {
            zone_id(target).to_uppercase()
        };
        layout.insert(id, json!({"dart":target_value(target),"kind":kind}));
    }
    state.mode_state["layouts"][key] = Value::Object(layout);
    Ok(())
}

fn overlay(state: &RegisteredGameState, player_id: &str) -> Result<Value, GameError> {
    let level = difficulty(state)?;
    let current_wave = wave(state, player_id)?;
    let board = layout(state, current_wave)?;
    let collected = collection(state, player_id)?;
    let whole = level == "easy";
    let mut bonus = milk_targets();
    let mut targets_out = Vec::new();
    let mut danger = Vec::new();
    let mut zones = Vec::new();
    let mut total_good = 0_usize;
    let mut remaining = 0_usize;
    for (id, item) in board {
        let kind = item["kind"].as_str().ok_or_else(invalid_state)?;
        if kind != "moldy" {
            total_good += 1;
        }
        if collected.contains(id) {
            continue;
        }
        if kind != "moldy" {
            remaining += 1;
        }
        let target = parse_target(&item["dart"])?;
        let label = display_points(level, kind)?;
        let entry = json!({
            "id":zone_id(&target),"field":target.field,"ring":target.ring,"color":cookie_color(kind)?,
            "label":label,"pulse":kind=="moldy","icon":if kind=="moldy"{"cookie_moldy"}else{"cookie"},
            "variant":kind,"match_field":whole,
        });
        if whole {
            zones.push(json!({"field":target.field,"rings":[Ring::SingleInner,Ring::Triple,Ring::SingleOuter,Ring::Double],"role":"control","color":cookie_color(kind)?}));
        }
        if kind == "gold" {
            bonus.push(entry);
        } else if kind == "moldy" {
            danger.push(entry);
        } else {
            targets_out.push(entry);
        }
    }
    Ok(json!({
        "prompt":"ALLE COOKIES ESSEN · SCHIMMEL MEIDEN · BULL = MILCH",
        "bonus":bonus,"targets":targets_out,"danger":danger,"zones":zones,
        "visual_legend":legend(level),
        "panel":{"title":format!("COOKIE BOARD {current_wave}"),"headline":format!("{remaining} Cookies übrig"),
            "subline":if flag(state,"sugar",player_id)?{"SUGAR RUSH BEREIT · nächster Cookie doppelt".into()}else if level=="hard"{format!("Serie {}/3 · Board erst komplett leer essen",counter(state,"streak",player_id)?)}else{"Board erst komplett leer essen".into()},
            "progress":{"value":total_good.saturating_sub(remaining),"max":total_good}},
    }))
}

fn milk_targets() -> Vec<Value> {
    vec![
        json!({"id":"SBULL","field":25,"ring":Ring::SingleBull,"color":"#8fd3ff","label":"MILCH","pulse":true,"icon":"milk","variant":"milk"}),
        json!({"id":"DBULL","field":25,"ring":Ring::DoubleBull,"color":"#8fd3ff","label":"","pulse":true,"icon":"milk","variant":"milk"}),
    ]
}

fn legend(level: &str) -> Vec<Value> {
    let mut items = Vec::new();
    if level != "easy" {
        items.push(json!({"icon":"cookie","color":"#ffcf33","label":"Gold-Cookie","value":"+50"}));
    }
    items.push(json!({"icon":"cookie","color":"#55c7dc","label":"Cookie","value":if level=="hard"{"+25"}else{"+20"}}));
    if level == "hard" {
        items
            .push(json!({"icon":"cookie","color":"#69c98f","label":"Grüner Cookie","value":"+10"}));
    }
    items.push(json!({"icon":"cookie_moldy","color":"#9dac76","label":"Schimmel","value":format!("-{}",mold_penalty(level))}));
    items.push(json!({"icon":"milk","color":"#8fd3ff","label":"Bull-Milch","value":if level=="easy"{"+30"}else{"Turn ×2 / retten"}}));
    items
}

fn matching_cookie(
    state: &RegisteredGameState,
    wave: u64,
    event: &DartEvent,
) -> Result<Option<String>, GameError> {
    let whole = difficulty(state)? == "easy";
    for (key, item) in layout(state, wave)? {
        let target = parse_target(&item["dart"])?;
        if (whole && matches!(event,DartEvent::Hit{field,..} if *field==target.field))
            || same_target(event, &target)
        {
            return Ok(Some(key.clone()));
        }
    }
    Ok(None)
}
fn layout(state: &RegisteredGameState, wave: u64) -> Result<&Map<String, Value>, GameError> {
    state.mode_state["layouts"][wave.to_string()]
        .as_object()
        .ok_or_else(invalid_state)
}
fn layout_item<'a>(
    state: &'a RegisteredGameState,
    wave: u64,
    key: &str,
) -> Result<&'a Value, GameError> {
    layout(state, wave)?.get(key).ok_or_else(invalid_state)
}
fn remaining_good(
    state: &RegisteredGameState,
    wave: u64,
    collected: &[String],
) -> Result<usize, GameError> {
    Ok(layout(state, wave)?
        .iter()
        .filter(|(id, item)| item["kind"] != "moldy" && !collected.contains(*id))
        .count())
}
fn wave(state: &RegisteredGameState, id: &str) -> Result<u64, GameError> {
    state.mode_state["wave"][id]
        .as_u64()
        .ok_or_else(invalid_state)
}
fn difficulty(state: &RegisteredGameState) -> Result<&str, GameError> {
    state.options["difficulty"]
        .as_str()
        .ok_or_else(invalid_state)
}
fn counter(state: &RegisteredGameState, key: &str, id: &str) -> Result<u8, GameError> {
    state.mode_state[key][id]
        .as_u64()
        .and_then(|v| u8::try_from(v).ok())
        .ok_or_else(invalid_state)
}
fn flag(state: &RegisteredGameState, key: &str, id: &str) -> Result<bool, GameError> {
    state.mode_state[key][id]
        .as_bool()
        .ok_or_else(invalid_state)
}
fn set_counter(
    state: &mut RegisteredGameState,
    key: &str,
    id: &str,
    value: u8,
) -> Result<(), GameError> {
    if state.mode_state[key].get(id).is_none() {
        return Err(invalid_state());
    }
    state.mode_state[key][id] = Value::from(value);
    Ok(())
}
fn set_flag(
    state: &mut RegisteredGameState,
    key: &str,
    id: &str,
    value: bool,
) -> Result<(), GameError> {
    if state.mode_state[key].get(id).is_none() {
        return Err(invalid_state());
    }
    state.mode_state[key][id] = Value::from(value);
    Ok(())
}
fn collection(state: &RegisteredGameState, id: &str) -> Result<Vec<String>, GameError> {
    state.mode_state["collected"][id]
        .as_array()
        .ok_or_else(invalid_state)?
        .iter()
        .map(|v| v.as_str().map(str::to_owned).ok_or_else(invalid_state))
        .collect()
}
fn set_collection(
    state: &mut RegisteredGameState,
    id: &str,
    items: &[String],
) -> Result<(), GameError> {
    if state.mode_state["collected"].get(id).is_none() {
        return Err(invalid_state());
    }
    state.mode_state["collected"][id] =
        Value::Array(items.iter().cloned().map(Value::from).collect());
    Ok(())
}
fn player_values<T: Clone + Into<Value>>(state: &RegisteredGameState, value: &T) -> Value {
    Value::Object(
        state
            .players
            .iter()
            .map(|p| (p.id.clone(), value.clone().into()))
            .collect(),
    )
}
fn player_collections(state: &RegisteredGameState) -> Value {
    Value::Object(
        state
            .players
            .iter()
            .map(|p| (p.id.clone(), Value::Array(Vec::new())))
            .collect(),
    )
}
fn set_effect(state: &mut RegisteredGameState, effect: &str, points: i64) {
    state.mode_state["last_effect"] = Value::from(effect);
    state.mode_state["effect_points"] = Value::from(points);
}
fn simple_target(field: u8) -> Target {
    Target {
        label: format!("S{field}"),
        field,
        ring: Ring::SingleOuter,
        multiplier: 1,
        score: u16::from(field),
    }
}
fn cookie_points(level: &str, kind: &str) -> Result<i64, GameError> {
    match (level, kind) {
        ("easy", _) | ("normal", "blue") => Ok(20),
        (_, "gold") => Ok(50),
        ("hard", "blue") => Ok(25),
        ("hard", "green") => Ok(10),
        _ => Err(invalid_state()),
    }
}
fn display_points(level: &str, kind: &str) -> Result<String, GameError> {
    if kind == "moldy" {
        Ok(format!("-{}", mold_penalty(level)))
    } else {
        Ok(format!("+{}", cookie_points(level, kind)?))
    }
}
fn mold_penalty(level: &str) -> i64 {
    match level {
        "easy" => 20,
        "normal" => 25,
        _ => 30,
    }
}
fn cookie_color(kind: &str) -> Result<&'static str, GameError> {
    match kind {
        "gold" => Ok("#ffcf33"),
        "blue" => Ok("#55c7dc"),
        "green" => Ok("#69c98f"),
        "moldy" => Ok("#9dac76"),
        _ => Err(invalid_state()),
    }
}
const fn integer(value: i64, label: &'static str) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Integer(value),
        label,
        description: None,
        description_en: None,
    }
}
const fn option(
    value: &'static str,
    label: &'static str,
    description: &'static str,
) -> GameOptionChoice {
    GameOptionChoice {
        value: GameOptionValue::Text(value),
        label,
        description: Some(description),
        description_en: None,
    }
}
fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid cookie_monster mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    fn game(level: &str) -> RegisteredGame {
        RegisteredGame::new_seeded(
            "cookie_monster",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"difficulty":level,"rounds":5}),
            42,
        )
        .expect("game")
    }

    fn event(item: &Value, seq: u64) -> DartEvent {
        let target = parse_target(&item["dart"]).expect("target");
        DartEvent::Hit {
            seq,
            field: target.field,
            ring: target.ring,
            multiplier: target.multiplier,
            label: target.label,
            score: target.score,
        }
    }

    fn item_of_kind(game: &RegisteredGame, kind: &str, skip: usize) -> Value {
        layout(game.state(), 1)
            .expect("layout")
            .values()
            .filter(|item| item["kind"] == kind)
            .nth(skip)
            .expect("item")
            .clone()
    }

    #[test]
    fn easy_uses_whole_numbers_and_personal_collection() {
        let mut game = game("easy");
        assert_eq!(game.state().random_cursor, 29);
        let cookie = layout(game.state(), 1).expect("layout")["F14"].clone();
        game.apply_throw(&DartEvent::Hit {
            seq: 1,
            field: 14,
            ring: Ring::Triple,
            multiplier: 3,
            label: "T14".into(),
            score: 42,
        })
        .expect("cookie");

        assert_eq!(game.state().players[0].score, 20);
        assert_eq!(game.state().mode_state["last_effect"], "cookie_eaten");
        assert_eq!(game.state().mode_state["collected"]["ada"], json!(["F14"]));
        assert_eq!(game.state().mode_state["collected"]["bob"], json!([]));
        assert_eq!(cookie["kind"], "blue");
    }

    #[test]
    fn hard_sugar_rush_doubles_the_next_good_cookie() {
        let mut game = game("hard");
        for seq in 0..3 {
            let green = item_of_kind(&game, "green", seq);
            game.apply_throw(&event(&green, u64::try_from(seq + 1).expect("seq")))
                .expect("green");
        }
        assert_eq!(game.state().mode_state["sugar"]["ada"], true);
        game.continue_turn().expect("Bob");
        game.next_player().expect("Ada round two");
        let green = item_of_kind(&game, "green", 3);
        game.apply_throw(&event(&green, 4)).expect("charged green");
        assert_eq!(game.state().players[0].score, 50);
        assert_eq!(game.state().mode_state["sugar"]["ada"], false);
    }

    #[test]
    fn milk_rescues_a_negative_normal_visit() {
        let mut game = game("normal");
        let mold = item_of_kind(&game, "moldy", 0);
        game.apply_throw(&event(&mold, 1)).expect("mold");
        game.apply_throw(&DartEvent::Hit {
            seq: 2,
            field: 25,
            ring: Ring::SingleBull,
            multiplier: 1,
            label: "SBull".into(),
            score: 25,
        })
        .expect("milk");
        assert_eq!(game.state().players[0].score, 0);
        assert_eq!(game.state().turn_score, 0);
        assert_eq!(game.state().mode_state["last_effect"], "cookie_milk");
        assert_eq!(game.state().mode_state["effect_points"], 25);
    }

    #[test]
    fn last_good_cookie_creates_one_shared_next_wave() {
        let mut game = game("normal");
        let board = layout(game.state(), 1).expect("layout").clone();
        let good = board
            .iter()
            .filter(|(_, item)| item["kind"] != "moldy")
            .map(|(id, item)| (id.clone(), item.clone()))
            .collect::<Vec<_>>();
        let (last_id, last) = good.last().expect("last").clone();
        let collected = good[..good.len() - 1]
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        set_collection(&mut game.state, "ada", &collected).expect("collected");
        game.apply_throw(&event(&last, 1)).expect("clear");

        assert_eq!(game.state().mode_state["wave"]["ada"], 2);
        assert_eq!(game.state().mode_state["wave"]["bob"], 1);
        assert_eq!(game.state().mode_state["last_effect"], "cookie_board_clear");
        assert!(layout(game.state(), 2).is_ok());
        assert!(!last_id.is_empty());
        assert_eq!(game.state().random_cursor, 46);
    }
}
