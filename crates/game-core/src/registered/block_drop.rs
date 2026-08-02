use super::{
    GameControlLegend, GameInstruction, GameMetadata, GameMode, GameOption, GameOptionChoice,
    GameOptionValue, RegisteredGameState,
};
use crate::{GameError, GameStatus};
use sdb_contracts::{DartEvent, Ring};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const WIDTH: usize = 5;
const HEIGHT: usize = 8;
const KINDS: [&str; 5] = ["I", "L", "O", "S", "T"];
const ROTATE_LEFT_FIELDS: [u8; 5] = [12, 5, 20, 1, 18];
const RIGHT_FIELDS: [u8; 5] = [4, 13, 6, 10, 15];
const ROTATE_RIGHT_FIELDS: [u8; 5] = [2, 17, 3, 19, 7];
const LEFT_FIELDS: [u8; 5] = [16, 8, 11, 14, 9];

static DIFFICULTY_CHOICES: [GameOptionChoice; 3] = [
    choice(
        "easy",
        "Easy · Double, Triple oder Bull",
        "Double, Triple, Single Bull und Double Bull lassen den Stein fallen.",
        "Doubles, Triples, Single Bull, and Double Bull drop the block.",
    ),
    choice(
        "normal",
        "Mittel · Double oder Bull",
        "Double und beide Bulls lassen den Stein fallen.",
        "Doubles and both Bulls drop the block.",
    ),
    choice(
        "hard",
        "Schwer · nur Bull",
        "Nur Single Bull oder Double Bull lassen den Stein fallen.",
        "Only Single Bull or Double Bull drop the block.",
    ),
];
static PACE_CHOICES: [GameOptionChoice; 2] = [
    choice(
        "classic",
        "Klassisch · 5 Linien",
        "Der Stein sinkt erst, nachdem alle Spieler ihre Aufnahme beendet haben.",
        "The block sinks only after every player has completed the visit.",
    ),
    choice(
        "action",
        "Action · 10 Linien, Sink je Dart",
        "Nach jedem Dart sinkt der Stein eine Zeile; Ziel sind zehn Linien.",
        "The block sinks one row after every dart; clear ten lines.",
    ),
];
static DROP_FLOW_CHOICES: [GameOptionChoice; 2] = [
    choice(
        "continue",
        "Weiterwerfen",
        "Nach einem Drop bleiben übrige Darts spielbar.",
        "After a drop, any remaining darts can still be thrown.",
    ),
    choice(
        "hold",
        "Zug beenden",
        "Ein erfolgreicher Drop beendet sofort die Aufnahme.",
        "A successful drop ends the visit immediately.",
    ),
];
static OPTIONS: [GameOption; 3] = [
    GameOption {
        key: "difficulty",
        label: "Drop-Ziel",
        kind: "choice",
        default: GameOptionValue::Text("easy"),
        choices: &DIFFICULTY_CHOICES,
    },
    GameOption {
        key: "pace",
        label: "Spieltempo",
        kind: "choice",
        default: GameOptionValue::Text("classic"),
        choices: &PACE_CHOICES,
    },
    GameOption {
        key: "drop_flow",
        label: "Nach Drop",
        kind: "choice",
        default: GameOptionValue::Text("continue"),
        choices: &DROP_FLOW_CHOICES,
    },
];
static INSTRUCTIONS: [GameInstruction; 4] = [
    GameInstruction {
        title: "Vier große Flächen",
        body: "Die vier Farbbögen bewegen links/rechts oder drehen links/rechts.",
        icon: "controls",
    },
    GameInstruction {
        title: "Cyan ist Drop",
        body: "Easy nutzt Double, Triple und Bull. Mittel nutzt Double und Bull. Schwer nutzt nur Bull.",
        icon: "power",
    },
    GameInstruction {
        title: "Wählt das Tempo",
        body: "Klassisch sinkt nach jeder Teamrunde. Action sinkt nach jedem Dart und spielt auf zehn Linien.",
        icon: "round",
    },
    GameInstruction {
        title: "Linien löschen",
        body: "Baut gemeinsam das gewählte Linienziel, bevor ein Stein oben herausragt.",
        icon: "blocks",
    },
];
static CONTROL_LEGEND: [GameControlLegend; 5] = [
    legend("left", "#e9c46a", "Nach links"),
    legend("rotate_left", "#a77bff", "Links drehen"),
    legend("rotate_right", "#f4a261", "Rechts drehen"),
    legend("right", "#81b29a", "Nach rechts"),
    GameControlLegend {
        icon: "drop",
        color: "#28e7ff",
        label: "Stein droppen",
        secondary_color: Some("#e76f51"),
        detail: Some("CYAN MARKIERT"),
    },
];
static METADATA: GameMetadata = GameMetadata {
    slug: "block_drop",
    ruleset_version: 2,
    title: "Block Drop Darts",
    tagline: "Gemeinsam fünf Linien bauen",
    description: "Darts steuern einen fröhlichen Block-Puzzler. Alle Spieler bauen gemeinsam am selben 5×8-Feld.",
    accent: "#e07a5f",
    accent_secondary: "#81b29a",
    visual: "block-drop",
    icon: "blocks",
    artwork: "/static/assets/modes/block_drop.webp",
    sound_theme: "arcade",
    min_players: 1,
    max_players: 8,
    options: &OPTIONS,
    instructions: &INSTRUCTIONS,
    control_legend: &CONTROL_LEGEND,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Piece {
    kind: String,
    rotation: usize,
    x: i32,
    y: i32,
}

pub(super) struct BlockDropMode;
pub(super) static BLOCK_DROP_MODE: BlockDropMode = BlockDropMode;

impl GameMode for BlockDropMode {
    fn metadata(&self) -> &'static GameMetadata {
        &METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        state.mode_state = json!({
            "board": vec![vec![0_u8; WIDTH]; HEIGHT],
            "lines": 0,
            "piece_index": 0,
            "gravity_round": 1,
            "last_effect": "",
            "effect_points": 0,
            "cleared_lines": 0,
        });
        spawn(state)?;
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        set_effect(state, "", 0, 0);
        let (action, force_lock, power_bonus) = apply_control(state, event)?;

        if !force_lock {
            if pace(state)? != "action" {
                state.message = action;
                return Ok(0);
            }
            if soft_drop(state)? {
                set_effect(state, "block_sink", 0, 0);
                state.message = format!("{action} · SINK ↓");
                return Ok(0);
            }
            let outcome = lock_piece(state, 0)?;
            set_effect(
                state,
                if outcome.cleared > 0 {
                    "block_line"
                } else {
                    "block_lock"
                },
                outcome.points,
                outcome.cleared,
            );
            if finish_after_lock(state, &outcome)? {
                return Ok(outcome.points);
            }
            state.message = format!(
                "{action} · SINK setzt den Stein · +{}{}",
                outcome.points,
                line_detail(outcome.cleared)
            );
            return Ok(outcome.points);
        }

        let outcome = lock_piece(state, power_bonus)?;
        set_effect(
            state,
            if power_bonus > 0 {
                "block_power_drop"
            } else if outcome.cleared > 0 {
                "block_line"
            } else {
                "block_drop"
            },
            outcome.points,
            outcome.cleared,
        );
        if finish_after_lock(state, &outcome)? {
            return Ok(outcome.points);
        }
        state.message = format!(
            "{action} · +{}{}",
            outcome.points,
            line_detail(outcome.cleared)
        );
        if drop_flow(state)? == "hold" {
            state.status = GameStatus::Hold;
        }
        Ok(outcome.points)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let mut display = board(state).unwrap_or_else(|_| vec![vec![0; WIDTH]; HEIGHT]);
        if let Ok(piece) = piece(state)
            && let Ok(piece_cells) = cells(&piece)
        {
            for (x, y) in piece_cells {
                if let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y))
                    && y < HEIGHT
                    && x < WIDTH
                {
                    display[y][x] = 2;
                }
            }
        }
        let grid_cells = display
            .into_iter()
            .flatten()
            .map(|value| {
                json!({
                    "value": "",
                    "state": if value == 2 { "active" } else if value == 1 { "filled" } else { "" },
                })
            })
            .collect::<Vec<_>>();
        let drops = drop_rings(state).unwrap_or_default();
        let control_rings = [
            Ring::SingleInner,
            Ring::Triple,
            Ring::SingleOuter,
            Ring::Double,
        ]
        .into_iter()
        .filter(|ring| !drops.contains(ring))
        .collect::<Vec<_>>();
        let mut zones = Vec::new();
        for (fields, color) in [
            (&LEFT_FIELDS, "#e9c46a"),
            (&ROTATE_LEFT_FIELDS, "#a77bff"),
            (&ROTATE_RIGHT_FIELDS, "#f4a261"),
            (&RIGHT_FIELDS, "#81b29a"),
        ] {
            for field in fields {
                zones.push(json!({
                    "field": field,
                    "rings": control_rings,
                    "role": "control",
                    "color": color,
                }));
            }
        }
        if !drops.is_empty() {
            for field in 1..=20 {
                zones.push(json!({
                    "field": field,
                    "rings": drops,
                    "role": "control",
                    "color": "#28e7ff",
                }));
            }
        }
        zones.push(json!({
            "field": 25, "rings": [Ring::SingleBull], "role": "control", "color": "#28e7ff",
        }));
        zones.push(json!({
            "field": 25, "rings": [Ring::DoubleBull], "role": "control", "color": "#e76f51",
        }));
        let lines = state.mode_state["lines"].as_u64().unwrap_or_default();
        let goal = line_goal(state).unwrap_or(5);
        let drop_label = match difficulty(state).unwrap_or("hard") {
            "easy" => "DOUBLE · TRIPLE · BULL = DROP",
            "normal" => "DOUBLE · BULL = DROP",
            _ => "BULL = DROP",
        };
        let pace_label = if pace(state).unwrap_or("classic") == "action" {
            "NACH JEDEM DART SINK ↓"
        } else {
            "SINK NACH TEAMRUNDE"
        };
        json!({
            "prompt": format!("GELB ← · LILA ↶ · ORANGE ↷ · GRÜN → · {drop_label}"),
            "zones": zones,
            "panel": {
                "title": "BLOCK DROP",
                "headline": format!("{lines}/{goal} Linien"),
                "subline": format!("Alle bauen gemeinsam · {pace_label}"),
                "progress": {"value": lines, "max": goal},
                "grid": {"columns": WIDTH, "cells": grid_cells},
            },
        })
    }

    fn on_turn_started(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        set_effect(state, "", 0, 0);
        if pace(state)? == "action" {
            return Ok(());
        }
        let gravity_round = state.mode_state["gravity_round"]
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(invalid_state)?;
        if state.round_number <= gravity_round {
            return Ok(());
        }
        state.mode_state["gravity_round"] = Value::from(state.round_number);
        if soft_drop(state)? {
            set_effect(state, "block_sink", 0, 0);
            state.message = format!("Runde {}: Stein fällt eine Zeile", state.round_number);
            return Ok(());
        }
        let outcome = lock_piece(state, 0)?;
        set_effect(
            state,
            if outcome.cleared > 0 {
                "block_line"
            } else {
                "block_lock"
            },
            outcome.points,
            outcome.cleared,
        );
        if finish_after_lock(state, &outcome)? {
            return Ok(());
        }
        state.message = format!(
            "Rundendrop setzt den Stein · +{}{}",
            outcome.points,
            line_detail(outcome.cleared)
        );
        Ok(())
    }
}

fn apply_control(
    state: &mut RegisteredGameState,
    event: &DartEvent,
) -> Result<(String, bool, i64), GameError> {
    match event {
        DartEvent::Miss { .. } => {
            set_effect(state, "block_miss", 0, 0);
            Ok(("MISS · keine Aktion".into(), false, 0))
        }
        DartEvent::Hit {
            field: 25, ring, ..
        } => {
            hard_drop(state)?;
            if *ring == Ring::DoubleBull {
                set_effect(state, "block_power_drop", 0, 0);
                Ok(("DOUBLE BULL · POWER DROP!".into(), true, 25))
            } else {
                set_effect(state, "block_drop", 0, 0);
                Ok(("SINGLE BULL · DROP!".into(), true, 0))
            }
        }
        DartEvent::Hit { ring, .. } if drop_rings(state)?.contains(ring) => {
            hard_drop(state)?;
            set_effect(state, "block_drop", 0, 0);
            Ok((
                format!(
                    "{} · DROP!",
                    if *ring == Ring::Triple {
                        "TRIPLE"
                    } else {
                        "DOUBLE"
                    }
                ),
                true,
                0,
            ))
        }
        DartEvent::Hit { field, .. } if LEFT_FIELDS.contains(field) => {
            move_piece(state, -1)?;
            set_effect(state, "block_move_left", 0, 0);
            Ok(("LINKS".into(), false, 0))
        }
        DartEvent::Hit { field, .. } if RIGHT_FIELDS.contains(field) => {
            move_piece(state, 1)?;
            set_effect(state, "block_move_right", 0, 0);
            Ok(("RECHTS".into(), false, 0))
        }
        DartEvent::Hit { field, .. } if ROTATE_LEFT_FIELDS.contains(field) => {
            rotate_piece(state, -1)?;
            set_effect(state, "block_rotate_left", 0, 0);
            Ok(("LINKS DREHEN".into(), false, 0))
        }
        DartEvent::Hit { .. } => {
            rotate_piece(state, 1)?;
            set_effect(state, "block_rotate_right", 0, 0);
            Ok(("RECHTS DREHEN".into(), false, 0))
        }
    }
}

struct LockOutcome {
    cleared: usize,
    points: i64,
    can_continue: bool,
}

fn spawn(state: &mut RegisteredGameState) -> Result<bool, GameError> {
    let index = state.mode_state["piece_index"]
        .as_u64()
        .ok_or_else(invalid_state)?;
    let kind = KINDS[state.random_index(KINDS.len())?];
    let width = shape(kind, 0)?
        .iter()
        .map(|(x, _)| *x)
        .max()
        .unwrap_or_default()
        + 1;
    let piece = Piece {
        kind: kind.into(),
        rotation: 0,
        x: (i32::try_from(WIDTH).map_err(|_| invalid_state())? - width) / 2,
        y: 0,
    };
    let can_continue = valid(state, &cells(&piece)?)?;
    state.mode_state["piece"] = serde_json::to_value(piece).map_err(|_| invalid_state())?;
    state.mode_state["piece_index"] = Value::from(index.saturating_add(1));
    Ok(can_continue)
}

fn move_piece(state: &mut RegisteredGameState, dx: i32) -> Result<(), GameError> {
    let mut current = piece(state)?;
    let mut candidate = current.clone();
    candidate.x = candidate.x.saturating_add(dx);
    if valid(state, &cells(&candidate)?)? {
        current.x = candidate.x;
        set_piece(state, &current)?;
    }
    Ok(())
}

fn rotate_piece(state: &mut RegisteredGameState, direction: i32) -> Result<(), GameError> {
    let current = piece(state)?;
    let count = rotation_count(&current.kind)?;
    let rotation = usize::try_from(
        (i32::try_from(current.rotation).map_err(|_| invalid_state())? + direction)
            .rem_euclid(i32::try_from(count).map_err(|_| invalid_state())?),
    )
    .map_err(|_| invalid_state())?;
    for kick in [0, -1, 1, -2, 2] {
        let candidate = Piece {
            kind: current.kind.clone(),
            rotation,
            x: current.x.saturating_add(kick),
            y: current.y,
        };
        if valid(state, &cells(&candidate)?)? {
            return set_piece(state, &candidate);
        }
    }
    Ok(())
}

fn soft_drop(state: &mut RegisteredGameState) -> Result<bool, GameError> {
    let mut current = piece(state)?;
    let mut candidate = current.clone();
    candidate.y = candidate.y.saturating_add(1);
    if valid(state, &cells(&candidate)?)? {
        current.y = candidate.y;
        set_piece(state, &current)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn hard_drop(state: &mut RegisteredGameState) -> Result<(), GameError> {
    while soft_drop(state)? {}
    Ok(())
}

fn lock_piece(state: &mut RegisteredGameState, power_bonus: i64) -> Result<LockOutcome, GameError> {
    let mut grid = board(state)?;
    for (x, y) in cells(&piece(state)?)? {
        let x = usize::try_from(x).map_err(|_| invalid_state())?;
        let y = usize::try_from(y).map_err(|_| invalid_state())?;
        *grid
            .get_mut(y)
            .and_then(|row| row.get_mut(x))
            .ok_or_else(invalid_state)? = 1;
    }
    let mut remaining = grid
        .into_iter()
        .filter(|row| !row.iter().all(|value| *value != 0))
        .collect::<Vec<_>>();
    let cleared = HEIGHT.saturating_sub(remaining.len());
    let mut next = vec![vec![0_u8; WIDTH]; cleared];
    next.append(&mut remaining);
    state.mode_state["board"] = serde_json::to_value(next).map_err(|_| invalid_state())?;
    let total_lines = state.mode_state["lines"]
        .as_u64()
        .ok_or_else(invalid_state)?
        .saturating_add(u64::try_from(cleared).map_err(|_| invalid_state())?);
    state.mode_state["lines"] = Value::from(total_lines);
    let can_continue = spawn(state)?;
    let points = 10_i64
        .saturating_add(line_points(cleared))
        .saturating_add(power_bonus);
    for player in &mut state.players {
        player.score = player.score.saturating_add(points);
    }
    Ok(LockOutcome {
        cleared,
        points,
        can_continue,
    })
}

fn finish_after_lock(
    state: &mut RegisteredGameState,
    outcome: &LockOutcome,
) -> Result<bool, GameError> {
    let lines = state.mode_state["lines"]
        .as_u64()
        .ok_or_else(invalid_state)?;
    if lines >= line_goal(state)? {
        finish_team(
            state,
            true,
            &format!("LINIENZIEL! Das Team gewinnt mit {lines} Linien"),
        );
        state.mode_state["effect_points"] = Value::from(outcome.points);
        return Ok(true);
    }
    if !outcome.can_continue {
        finish_team(state, false, "BLOCK OUT! Das Feld ist voll");
        state.mode_state["effect_points"] = Value::from(outcome.points);
        return Ok(true);
    }
    Ok(false)
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
    state.mode_state["last_effect"] = Value::from(if won { "block_win" } else { "block_out" });
}

fn cells(piece: &Piece) -> Result<Vec<(i32, i32)>, GameError> {
    Ok(shape(&piece.kind, piece.rotation)?
        .iter()
        .map(|(dx, dy)| (piece.x.saturating_add(*dx), piece.y.saturating_add(*dy)))
        .collect())
}

fn valid(state: &RegisteredGameState, candidate: &[(i32, i32)]) -> Result<bool, GameError> {
    let grid = board(state)?;
    Ok(candidate.iter().all(|(x, y)| {
        let (Ok(x), Ok(y)) = (usize::try_from(*x), usize::try_from(*y)) else {
            return false;
        };
        y < HEIGHT && x < WIDTH && grid[y][x] == 0
    }))
}

fn board(state: &RegisteredGameState) -> Result<Vec<Vec<u8>>, GameError> {
    let grid: Vec<Vec<u8>> =
        serde_json::from_value(state.mode_state["board"].clone()).map_err(|_| invalid_state())?;
    if grid.len() != HEIGHT || grid.iter().any(|row| row.len() != WIDTH) {
        return Err(invalid_state());
    }
    Ok(grid)
}

fn piece(state: &RegisteredGameState) -> Result<Piece, GameError> {
    serde_json::from_value(state.mode_state["piece"].clone()).map_err(|_| invalid_state())
}

fn set_piece(state: &mut RegisteredGameState, piece: &Piece) -> Result<(), GameError> {
    state.mode_state["piece"] = serde_json::to_value(piece).map_err(|_| invalid_state())?;
    Ok(())
}

fn shape(kind: &str, rotation: usize) -> Result<&'static [(i32, i32)], GameError> {
    let shapes: &[&[(i32, i32)]] = match kind {
        "I" => &[
            &[(0, 0), (1, 0), (2, 0), (3, 0)],
            &[(0, 0), (0, 1), (0, 2), (0, 3)],
        ],
        "O" => &[&[(0, 0), (1, 0), (0, 1), (1, 1)]],
        "T" => &[
            &[(0, 0), (1, 0), (2, 0), (1, 1)],
            &[(1, 0), (0, 1), (1, 1), (1, 2)],
            &[(1, 0), (0, 1), (1, 1), (2, 1)],
            &[(0, 0), (0, 1), (1, 1), (0, 2)],
        ],
        "L" => &[
            &[(0, 0), (0, 1), (0, 2), (1, 2)],
            &[(0, 0), (1, 0), (2, 0), (0, 1)],
            &[(0, 0), (1, 0), (1, 1), (1, 2)],
            &[(2, 0), (0, 1), (1, 1), (2, 1)],
        ],
        "S" => &[
            &[(1, 0), (2, 0), (0, 1), (1, 1)],
            &[(0, 0), (0, 1), (1, 1), (1, 2)],
        ],
        _ => return Err(invalid_state()),
    };
    shapes
        .get(rotation % shapes.len())
        .copied()
        .ok_or_else(invalid_state)
}

fn rotation_count(kind: &str) -> Result<usize, GameError> {
    Ok(match kind {
        "I" | "S" => 2,
        "O" => 1,
        "T" | "L" => 4,
        _ => return Err(invalid_state()),
    })
}

fn drop_rings(state: &RegisteredGameState) -> Result<Vec<Ring>, GameError> {
    Ok(match difficulty(state)? {
        "easy" => vec![Ring::Triple, Ring::Double],
        "normal" => vec![Ring::Double],
        "hard" => Vec::new(),
        _ => return Err(invalid_state()),
    })
}

fn line_goal(state: &RegisteredGameState) -> Result<u64, GameError> {
    Ok(if pace(state)? == "action" { 10 } else { 5 })
}

const fn line_points(cleared: usize) -> i64 {
    match cleared {
        1 => 50,
        2 => 120,
        3 => 250,
        4 => 500,
        _ => 0,
    }
}

fn line_detail(cleared: usize) -> String {
    if cleared == 0 {
        String::new()
    } else {
        format!(" · {cleared} Linie{}!", if cleared == 1 { "" } else { "n" })
    }
}

fn set_effect(state: &mut RegisteredGameState, effect: &str, points: i64, cleared: usize) {
    state.mode_state["last_effect"] = Value::from(effect);
    state.mode_state["effect_points"] = Value::from(points);
    state.mode_state["cleared_lines"] = Value::from(cleared);
}

fn difficulty(state: &RegisteredGameState) -> Result<&str, GameError> {
    state.options["difficulty"]
        .as_str()
        .ok_or_else(invalid_state)
}

fn pace(state: &RegisteredGameState) -> Result<&str, GameError> {
    state.options["pace"].as_str().ok_or_else(invalid_state)
}

fn drop_flow(state: &RegisteredGameState) -> Result<&str, GameError> {
    state.options["drop_flow"]
        .as_str()
        .ok_or_else(invalid_state)
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

const fn legend(icon: &'static str, color: &'static str, label: &'static str) -> GameControlLegend {
    GameControlLegend {
        icon,
        color,
        label,
        secondary_color: None,
        detail: None,
    }
}

fn invalid_state() -> GameError {
    GameError::RulesetUnavailable("invalid block_drop mode state".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredGame;

    fn game(options: &Value) -> RegisteredGame {
        RegisteredGame::new_seeded(
            "block_drop",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            options,
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
            label: format!("{field}"),
            score: u16::from(field) * u16::from(multiplier),
        }
    }

    #[test]
    fn bulls_drop_and_power_drop_awards_every_team_member() {
        let mut game = game(&json!({"difficulty":"easy","pace":"classic","drop_flow":"continue"}));
        let before = game.state().mode_state["piece_index"]
            .as_u64()
            .expect("index");
        game.apply_throw(&hit(1, 25, Ring::DoubleBull))
            .expect("drop");

        assert_eq!(game.state().mode_state["piece_index"], before + 1);
        assert_eq!(game.state().players[0].score, 35);
        assert_eq!(game.state().players[1].score, 35);
        assert_eq!(game.state().mode_state["last_effect"], "block_power_drop");
    }

    #[test]
    fn classic_gravity_runs_only_when_the_team_round_wraps() {
        let mut game = game(&json!({"difficulty":"easy","pace":"classic","drop_flow":"continue"}));
        let start_y = piece(game.state()).expect("piece").y;
        game.next_player().expect("Bob");
        assert_eq!(piece(game.state()).expect("piece").y, start_y);
        game.next_player().expect("next round");
        assert_eq!(piece(game.state()).expect("piece").y, start_y + 1);
        assert_eq!(game.state().round_number, 2);
    }

    #[test]
    fn rotations_move_both_directions_and_are_undoable() {
        let mut game = RegisteredGame::new_seeded(
            "block_drop",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({"difficulty":"easy","pace":"classic","drop_flow":"continue"}),
            17,
        )
        .expect("game");
        game.apply_throw(&hit(1, 20, Ring::SingleOuter))
            .expect("left rotate");
        assert_eq!(piece(game.state()).expect("piece").rotation, 3);
        game.apply_throw(&hit(2, 3, Ring::SingleOuter))
            .expect("right rotate");
        assert_eq!(piece(game.state()).expect("piece").rotation, 0);
        game.undo().expect("undo");
        assert_eq!(piece(game.state()).expect("piece").rotation, 3);
    }

    #[test]
    fn hold_flow_ends_the_visit_after_a_drop() {
        let mut game = game(&json!({"difficulty":"hard","pace":"classic","drop_flow":"hold"}));
        game.apply_throw(&hit(1, 25, Ring::SingleBull))
            .expect("drop");
        assert_eq!(game.state().status, GameStatus::Hold);
        assert_eq!(game.state().darts_in_turn, 1);
    }

    #[test]
    fn corrupted_piece_shape_is_rejected_instead_of_locking_nothing() {
        let mut game = game(&json!({"difficulty":"easy","pace":"classic","drop_flow":"continue"}));
        game.state.mode_state["piece"]["kind"] = Value::from("invalid");

        assert!(matches!(
            game.apply_throw(&hit(1, 16, Ring::SingleOuter)),
            Err(GameError::RulesetUnavailable(_))
        ));
    }
}
