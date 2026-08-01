use super::{
    GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredGameState, finish_fixed_round_game,
};
use crate::GameError;
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

const BOARD_ORDER: [u8; 20] = [
    20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5,
];
const NUMBER_RINGS: [Ring; 4] = [
    Ring::SingleInner,
    Ring::Triple,
    Ring::SingleOuter,
    Ring::Double,
];

static ROUND_CHOICES: [GameOptionChoice; 3] = [
    choice_integer(3, "3 Runden"),
    choice_integer(5, "5 Runden"),
    choice_integer(8, "8 Runden"),
];
static OWNERSHIP_CHOICES: [GameOptionChoice; 3] = [
    GameOptionChoice {
        value: GameOptionValue::Text("area"),
        label: "Leicht · Double-Reihe, Triple-Nachbarn",
        description: Some("Double erobert die ganze Zahl; Triple zusätzlich beide Nachbarzahlen."),
        description_en: Some(
            "A Double captures the whole number; a Triple also captures both neighbors.",
        ),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("segment"),
        label: "Klassisch · Segment genau",
        description: Some("Nur das tatsächlich getroffene physische Segment wird erobert."),
        description_en: Some("Only the exact physical segment hit is captured."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("field"),
        label: "Sehr leicht · Ganzes Zahlenfeld",
        description: Some("Jeder Treffer erobert alle vier Ringe der getroffenen Zahl."),
        description_en: Some("Every hit captures all four rings of the number."),
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
        key: "ownership",
        label: "Eroberung",
        kind: "choice",
        default: GameOptionValue::Text("segment"),
        choices: &OWNERSHIP_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 4] = [
    GameInstruction {
        title: "Felder erobern",
        body: "Treffer übernehmen Felder in deiner Farbe.",
        icon: "capture",
    },
    GameInstruction {
        title: "Leichte Ring-Power",
        body: "Double nimmt die ganze Zahl. Triple nimmt zusätzlich beide Nachbarzahlen.",
        icon: "power",
    },
    GameInstruction {
        title: "Zurückstehlen",
        body: "Triff gegnerische Felder, um sie zu übernehmen.",
        icon: "steal",
    },
    GameInstruction {
        title: "Mehrheit gewinnt",
        body: "Nach den Runden gewinnt die größte Board-Kontrolle.",
        icon: "crown",
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "king_of_board",
    ruleset_version: 2,
    title: "King of the Board",
    tagline: "Erobere die Scheibe",
    description: "Jeder Treffer übernimmt ein Feld in deiner Farbe. Nach den Runden gewinnt die größte Herrschaft.",
    accent: "#9b5cff",
    accent_secondary: "#28e7ff",
    visual: "king-board",
    icon: "flag",
    artwork: "/static/assets/modes/king_of_board.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
};

pub(super) struct KingOfBoardMode;
pub(super) static KING_OF_BOARD_MODE: KingOfBoardMode = KingOfBoardMode;

impl GameMode for KingOfBoardMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({
            "owned": {},
            "last_effect": "",
            "capture_count": 0,
            "capture_cells": [],
            "previous_owner_ids": [],
        });
        state.message = "Erobere die Scheibe!".into();
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        clear_effect(state);
        let Some((field, ring)) = hit_target(event) else {
            state.mode_state["last_effect"] = Value::from("king_miss");
            state.message = "Miss – kein Gebiet".into();
            finish_fixed_round_game(state, "{winner} regiert die Scheibe!")?;
            return Ok(0);
        };
        let cells = capture_cells(state, field, ring)?;
        let player_index = state.current_player_index;
        let player = state
            .players
            .get(player_index)
            .ok_or(GameError::NoPlayers)?;
        let player_id = player.id.clone();
        let player_name = player.name.clone();
        let player_color = player.color.clone();
        let before_score = player.score;
        let mut previous_owner_ids = BTreeSet::new();
        let mut all_held = true;
        let mut opponent_owned = false;
        {
            let owned = owned(state)?;
            for cell in &cells {
                let previous = owned
                    .get(&cell.id())
                    .and_then(|item| item.get("owner_id"))
                    .and_then(Value::as_str);
                match previous {
                    Some(owner_id) if owner_id == player_id => {
                        previous_owner_ids.insert(owner_id.to_owned());
                    }
                    Some(owner_id) => {
                        all_held = false;
                        opponent_owned = true;
                        previous_owner_ids.insert(owner_id.to_owned());
                    }
                    None => all_held = false,
                }
            }
        }
        for cell in &cells {
            owned_mut(state)?.insert(
                cell.id(),
                json!({
                    "owner_id": player_id.clone(),
                    "color": player_color.clone(),
                    "label": cell.id(),
                    "field": cell.field,
                    "ring": cell.ring,
                }),
            );
        }
        recalculate_scores(state)?;
        let score_change = state.players[player_index]
            .score
            .saturating_sub(before_score);
        let (action, effect) = if all_held {
            ("hält", "king_hold")
        } else if opponent_owned {
            ("erobert", "king_steal")
        } else {
            ("übernimmt", "king_capture")
        };
        state.mode_state["last_effect"] = Value::from(effect);
        state.mode_state["capture_count"] = Value::from(cells.len());
        state.mode_state["capture_cells"] =
            Value::Array(cells.iter().map(|cell| Value::from(cell.id())).collect());
        state.mode_state["previous_owner_ids"] =
            Value::Array(previous_owner_ids.into_iter().map(Value::from).collect());
        state.message = format!(
            "{player_name} {action} {} · {} Gebiet{}",
            capture_label(state, event, field, ring)?,
            cells.len(),
            if cells.len() == 1 { "" } else { "e" },
        );
        finish_fixed_round_game(state, "{winner} regiert die Scheibe!")?;
        Ok(score_change)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let owned_items = state.mode_state["owned"]
            .as_object()
            .map(|owned| {
                owned
                    .values()
                    .filter_map(|item| {
                        let field = item.get("field")?.as_u64()?;
                        let ring = item.get("ring")?.clone();
                        Some(json!({
                            "id": item.get("label")?.as_str()?,
                            "field": field,
                            "ring": ring,
                            "color": item.get("color").and_then(Value::as_str).unwrap_or("#28e7ff"),
                            "owner_id": item.get("owner_id")?.as_str()?,
                        }))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let prompt = if ownership(state).unwrap_or_default() == "area" {
            "SINGLE = SEGMENT · DOUBLE = GANZE ZAHL · TRIPLE = ZAHL + NACHBARN"
        } else {
            "EROBERE DIE SCHEIBE!"
        };
        let rows = state
            .players
            .iter()
            .map(|player| {
                json!({
                    "label": player.name,
                    "value": format!("{} Gebiete", player.score),
                    "color": player.color,
                })
            })
            .collect::<Vec<_>>();
        json!({
            "prompt": prompt,
            "owned": owned_items,
            "panel": {
                "title": "BOARD-KONTROLLE",
                "headline": format!("{} / 82 erobert", state.mode_state["owned"].as_object().map_or(0, Map::len)),
                "rows": rows,
            },
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        clear_effect(state);
        Ok(())
    }

    fn on_turn_skipped(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        clear_effect(state);
        Ok(())
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        Some("{winner} regiert die Scheibe!")
    }
}

#[derive(Clone, Copy)]
struct Cell {
    field: u8,
    ring: Ring,
}

impl Cell {
    fn id(self) -> String {
        format!("{}:{}", self.field, ring_name(self.ring))
    }
}

fn hit_target(event: &DartEvent) -> Option<(u8, Ring)> {
    match event {
        DartEvent::Hit { field, ring, .. } => Some((*field, *ring)),
        DartEvent::Miss { .. } => None,
    }
}

fn capture_cells(
    state: &RegisteredGameState,
    field: u8,
    ring: Ring,
) -> Result<Vec<Cell>, GameError> {
    if field == 25 {
        return Ok(vec![Cell { field, ring }]);
    }
    let rule = ownership(state)?;
    if rule == "field" || (rule == "area" && ring == Ring::Double) {
        return Ok(number_cells(field));
    }
    if rule == "area" && ring == Ring::Triple {
        return Ok(neighbor_fields(field)?
            .into_iter()
            .flat_map(number_cells)
            .collect());
    }
    Ok(vec![Cell { field, ring }])
}

fn capture_label(
    state: &RegisteredGameState,
    event: &DartEvent,
    field: u8,
    ring: Ring,
) -> Result<String, GameError> {
    if field == 25 {
        return Ok(event.label().to_owned());
    }
    let rule = ownership(state)?;
    if rule == "area" && ring == Ring::Triple {
        let neighbors = neighbor_fields(field)?;
        return Ok(format!(
            "{} · {} · {}",
            neighbors[0], neighbors[1], neighbors[2]
        ));
    }
    if rule == "field" || (rule == "area" && ring == Ring::Double) {
        return Ok(format!("ganze {field}"));
    }
    Ok(event.label().to_owned())
}

fn number_cells(field: u8) -> Vec<Cell> {
    NUMBER_RINGS
        .into_iter()
        .map(|ring| Cell { field, ring })
        .collect()
}

fn neighbor_fields(field: u8) -> Result<[u8; 3], GameError> {
    let index = BOARD_ORDER
        .iter()
        .position(|candidate| *candidate == field)
        .ok_or_else(invalid_state)?;
    Ok([
        BOARD_ORDER[(index + BOARD_ORDER.len() - 1) % BOARD_ORDER.len()],
        field,
        BOARD_ORDER[(index + 1) % BOARD_ORDER.len()],
    ])
}

fn recalculate_scores(state: &mut RegisteredGameState) -> Result<(), GameError> {
    let mut counts = state
        .players
        .iter()
        .map(|player| (player.id.clone(), 0_i64))
        .collect::<BTreeMap<_, _>>();
    for item in owned(state)?.values() {
        if let Some(owner_id) = item.get("owner_id").and_then(Value::as_str)
            && let Some(count) = counts.get_mut(owner_id)
        {
            *count = count.saturating_add(1);
        }
    }
    for player in &mut state.players {
        player.score = counts.get(&player.id).copied().unwrap_or_default();
    }
    Ok(())
}

fn clear_effect(state: &mut RegisteredGameState) {
    state.mode_state["last_effect"] = Value::from("");
    state.mode_state["capture_count"] = Value::from(0);
    state.mode_state["capture_cells"] = Value::Array(Vec::new());
    state.mode_state["previous_owner_ids"] = Value::Array(Vec::new());
}

fn owned(state: &RegisteredGameState) -> Result<&Map<String, Value>, GameError> {
    state.mode_state["owned"]
        .as_object()
        .ok_or_else(invalid_state)
}

fn owned_mut(state: &mut RegisteredGameState) -> Result<&mut Map<String, Value>, GameError> {
    state.mode_state["owned"]
        .as_object_mut()
        .ok_or_else(invalid_state)
}

fn ownership(state: &RegisteredGameState) -> Result<&str, GameError> {
    state.options["ownership"]
        .as_str()
        .ok_or_else(invalid_state)
}

fn ring_name(ring: Ring) -> &'static str {
    match ring {
        Ring::SingleInner => "single_inner",
        Ring::Triple => "triple",
        Ring::SingleOuter => "single_outer",
        Ring::Double => "double",
        Ring::SingleBull => "single_bull",
        Ring::DoubleBull => "double_bull",
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
    GameError::RulesetUnavailable("invalid king_of_board mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;
    use sdb_contracts::PlayerRef;

    fn game(ownership: &str) -> RegisteredGame {
        RegisteredGame::new_seeded_with_players(
            "king_of_board",
            vec![
                PlayerRef {
                    id: "ada".into(),
                    name: "Ada".into(),
                    avatar: "fox".into(),
                    color: "#ff00aa".into(),
                },
                PlayerRef {
                    id: "bob".into(),
                    name: "Bob".into(),
                    avatar: "comet".into(),
                    color: "#00ffaa".into(),
                },
            ],
            &json!({"rounds": 3, "ownership": ownership}),
            42,
        )
        .expect("game")
    }

    fn hit(seq: u64, field: u8, ring: Ring) -> DartEvent {
        let multiplier = if ring == Ring::Triple {
            3
        } else if matches!(ring, Ring::Double | Ring::DoubleBull) {
            2
        } else {
            1
        };
        DartEvent::Hit {
            seq,
            field,
            ring,
            multiplier,
            label: format!("{}{}", ring_name(ring), field),
            score: u16::from(field) * u16::from(multiplier),
        }
    }

    #[test]
    fn area_triple_captures_the_number_and_physical_neighbors() {
        let mut game = game("area");
        game.apply_throw(&hit(1, 20, Ring::Triple))
            .expect("capture");

        assert_eq!(game.state().players[0].score, 12);
        let fields = owned(game.state())
            .expect("owned")
            .values()
            .filter_map(|item| item["field"].as_u64())
            .collect::<BTreeSet<_>>();
        assert_eq!(fields, BTreeSet::from([1, 5, 20]));
        assert!(game.state().message.contains("5 · 20 · 1"));
    }

    #[test]
    fn stealing_uses_profile_color_recalculates_both_scores_and_undoes() {
        let mut game = game("field");
        game.apply_throw(&hit(1, 20, Ring::SingleOuter))
            .expect("Ada capture");
        game.next_player().expect("Bob turn");
        game.apply_throw(&hit(2, 20, Ring::Double))
            .expect("Bob steal");

        assert_eq!(game.state().players[0].score, 0);
        assert_eq!(game.state().players[1].score, 4);
        assert_eq!(game.state().mode_state["last_effect"], "king_steal");
        assert_eq!(
            game.state().mode_state["previous_owner_ids"],
            json!(["ada"])
        );
        assert!(
            owned(game.state())
                .expect("owned")
                .values()
                .all(|item| item["color"] == "#00ffaa")
        );

        game.undo().expect("undo steal");
        assert_eq!(game.state().players[0].score, 4);
        assert_eq!(game.state().players[1].score, 0);
    }

    #[test]
    fn single_and_double_bull_remain_separate_territories() {
        let mut game = game("field");
        game.apply_throw(&hit(1, 25, Ring::SingleBull))
            .expect("single bull");
        game.apply_throw(&hit(2, 25, Ring::DoubleBull))
            .expect("double bull");

        assert_eq!(game.state().players[0].score, 2);
        assert!(
            owned(game.state())
                .expect("owned")
                .contains_key("25:single_bull")
        );
        assert!(
            owned(game.state())
                .expect("owned")
                .contains_key("25:double_bull")
        );
    }

    #[test]
    fn skipping_the_last_players_final_visit_finishes_the_game() {
        let mut game = game("segment");
        game.state.current_player_index = 1;
        game.state.round_number = 3;
        game.next_player().expect("finish final visit");

        assert_eq!(game.state().status, crate::GameStatus::Finished);
        assert_eq!(game.state().result_type, "draw");
    }
}
