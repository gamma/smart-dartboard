use crate::{GameError, GameStatus, seed_from_id, with_seq};
use sdb_contracts::{DartEvent, PlayerRef, Ring};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

mod arcade;
mod avoid_bomb;
mod block_drop;
mod candy_cannon;
mod color_clash;
mod cookie_monster;
mod dart_sweeper;
mod dragon_eggs;
mod eight_ball;
mod ghost_chase;
mod heart_chase;
mod king_of_board;
mod lightning_round;
mod mini_golf;
mod risk_it;
mod robin_hood;
mod simon_says;
mod space_defender;
mod target_rush;
mod treasure_hunt;

use avoid_bomb::AVOID_BOMB_MODE;
use block_drop::BLOCK_DROP_MODE;
use candy_cannon::CANDY_CANNON_MODE;
use color_clash::COLOR_CLASH_MODE;
use cookie_monster::COOKIE_MONSTER_MODE;
use dart_sweeper::DART_SWEEPER_MODE;
use dragon_eggs::DRAGON_EGGS_MODE;
use eight_ball::EIGHT_BALL_MODE;
use ghost_chase::GHOST_CHASE_MODE;
use heart_chase::HEART_CHASE_MODE;
use king_of_board::KING_OF_BOARD_MODE;
use lightning_round::LIGHTNING_ROUND_MODE;
use mini_golf::MINI_GOLF_MODE;
use risk_it::RISK_IT_MODE;
use robin_hood::ROBIN_HOOD_MODE;
use simon_says::SIMON_SAYS_MODE;
use space_defender::SPACE_DEFENDER_MODE;
use target_rush::TARGET_RUSH_MODE;
use treasure_hunt::TREASURE_HUNT_MODE;

const CRICKET_TARGETS: [u8; 7] = [20, 19, 18, 17, 16, 15, 25];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOptionValue {
    Integer(i64),
    Boolean(bool),
    Text(&'static str),
}

impl Serialize for GameOptionValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Integer(value) => serializer.serialize_i64(*value),
            Self::Boolean(value) => serializer.serialize_bool(*value),
            Self::Text(value) => serializer.serialize_str(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameOptionChoice {
    pub value: GameOptionValue,
    pub label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_en: Option<&'static str>,
}

impl GameOptionValue {
    fn as_json(self) -> Value {
        match self {
            Self::Integer(value) => Value::from(value),
            Self::Boolean(value) => Value::from(value),
            Self::Text(value) => Value::from(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameOption {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub default: GameOptionValue,
    pub choices: &'static [GameOptionChoice],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameInstruction {
    pub title: &'static str,
    pub body: &'static str,
    pub icon: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameControlLegend {
    pub icon: &'static str,
    pub color: &'static str,
    pub label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_color: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GameMetadata {
    pub slug: &'static str,
    pub ruleset_version: u16,
    pub title: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub accent: &'static str,
    pub accent_secondary: &'static str,
    pub visual: &'static str,
    pub icon: &'static str,
    pub artwork: &'static str,
    pub sound_theme: &'static str,
    pub min_players: usize,
    pub max_players: usize,
    pub options: &'static [GameOption],
    pub instructions: &'static [GameInstruction],
    pub control_legend: &'static [GameControlLegend],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredPlayer {
    pub id: String,
    pub name: String,
    #[serde(default = "default_player_avatar")]
    pub avatar: String,
    #[serde(default = "default_player_color")]
    pub color: String,
    pub score: i64,
    #[serde(default)]
    pub marks: BTreeMap<String, u8>,
}

fn default_player_avatar() -> String {
    "comet".into()
}

fn default_player_color() -> String {
    "#28e7ff".into()
}

const fn fallback_player_color(index: usize) -> &'static str {
    const COLORS: [&str; 8] = [
        "#28e7ff", "#ffb52b", "#3dff91", "#ff4f79", "#a77bff", "#ffffff", "#ff7a45", "#66a3ff",
    ];
    COLORS[index % COLORS.len()]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredGameState {
    pub game_type: String,
    pub ruleset_version: u16,
    pub players: Vec<RegisteredPlayer>,
    pub current_player_index: usize,
    pub darts_in_turn: u8,
    pub turn_score: i64,
    pub round_number: u16,
    pub status: GameStatus,
    pub winner_id: Option<String>,
    pub winner_ids: Vec<String>,
    pub result_type: String,
    pub message: String,
    pub options: Value,
    pub overlay: Value,
    pub last_event: Option<DartEvent>,
    #[serde(default)]
    pub editable_darts: Vec<RegisteredDartRecord>,
    #[serde(default)]
    pub random_seed: u64,
    #[serde(default)]
    pub random_cursor: u64,
    #[serde(default)]
    pub mode_state: Value,
}

impl RegisteredGameState {
    /// Returns a deterministic index and advances the persisted random cursor.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::InvalidOptions`] when `upper_bound` is zero.
    pub fn random_index(&mut self, upper_bound: usize) -> Result<usize, GameError> {
        if upper_bound == 0 {
            return Err(GameError::InvalidOptions(
                "random upper bound must be greater than zero".into(),
            ));
        }
        self.random_cursor = self.random_cursor.wrapping_add(1);
        let mut value = self
            .random_seed
            .wrapping_add(0x9e37_79b9_7f4a_7c15_u64.wrapping_mul(self.random_cursor));
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^= value >> 31;
        let bound = u64::try_from(upper_bound)
            .map_err(|_| GameError::InvalidOptions("random upper bound exceeds u64".into()))?;
        usize::try_from(value % bound)
            .map_err(|_| GameError::InvalidOptions("random index exceeds usize".into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredDartRecord {
    pub action_id: u64,
    pub event: DartEvent,
    pub player_id: String,
    pub score_after: i64,
    pub round_number: u16,
    pub dart_in_turn: u8,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RegisteredAction {
    Dart {
        id: u64,
        event: DartEvent,
    },
    Continue {
        id: u64,
    },
    NextPlayer {
        id: u64,
    },
    Mode {
        id: u64,
        action: String,
        payload: Value,
    },
}

impl RegisteredAction {
    const fn id(&self) -> u64 {
        match self {
            Self::Dart { id, .. }
            | Self::Continue { id }
            | Self::NextPlayer { id }
            | Self::Mode { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredGame {
    state: RegisteredGameState,
    #[serde(default)]
    history: Vec<RegisteredGameState>,
    #[serde(default)]
    initial_state: Option<RegisteredGameState>,
    #[serde(default)]
    actions: Vec<RegisteredAction>,
    #[serde(default = "first_action_id")]
    next_action_id: u64,
}

impl RegisteredGame {
    /// Creates a game from the build-time registry and resolves its options.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown mode, invalid players or invalid options.
    pub fn new(
        game_type: &str,
        players: Vec<(String, String)>,
        options: &Value,
    ) -> Result<Self, GameError> {
        let player_ids = players
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>()
            .join("\u{1f}");
        let seed = seed_from_id(&format!("{game_type}\u{1e}{player_ids}\u{1e}{options}"));
        Self::new_seeded(game_type, players, options, seed)
    }

    /// Creates a registered game with an injected deterministic random seed.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown mode, invalid players or invalid options.
    pub fn new_seeded(
        game_type: &str,
        players: Vec<(String, String)>,
        options: &Value,
        random_seed: u64,
    ) -> Result<Self, GameError> {
        let players = players
            .into_iter()
            .enumerate()
            .map(|(index, (id, name))| PlayerRef {
                id,
                name,
                avatar: default_player_avatar(),
                color: fallback_player_color(index).into(),
            })
            .collect();
        Self::new_seeded_with_players(game_type, players, options, random_seed)
    }

    /// Creates a registered game while preserving selected player profiles.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown mode, invalid players or invalid options.
    pub fn new_seeded_with_players(
        game_type: &str,
        players: Vec<PlayerRef>,
        options: &Value,
        random_seed: u64,
    ) -> Result<Self, GameError> {
        if players.is_empty() {
            return Err(GameError::NoPlayers);
        }
        let mode = mode(game_type)?;
        let metadata = mode.metadata();
        if !(metadata.min_players..=metadata.max_players).contains(&players.len()) {
            return Err(GameError::InvalidOptions(format!(
                "{} supports {} to {} players",
                metadata.slug, metadata.min_players, metadata.max_players
            )));
        }
        let options = resolve_options(metadata, options)?;
        let mut state = RegisteredGameState {
            game_type: metadata.slug.into(),
            ruleset_version: metadata.ruleset_version,
            players: players
                .into_iter()
                .map(|player| RegisteredPlayer {
                    id: player.id,
                    name: player.name,
                    avatar: player.avatar,
                    color: player.color,
                    score: 0,
                    marks: BTreeMap::new(),
                })
                .collect(),
            current_player_index: 0,
            darts_in_turn: 0,
            turn_score: 0,
            round_number: 1,
            status: GameStatus::Running,
            winner_id: None,
            winner_ids: Vec::new(),
            result_type: String::new(),
            message: "Game started".into(),
            options,
            overlay: Value::Null,
            last_event: None,
            editable_darts: Vec::new(),
            random_seed,
            random_cursor: 0,
            mode_state: Value::Object(Map::new()),
        };
        mode.initialize(&mut state)?;
        state.overlay = mode.overlay(&state);
        Ok(Self {
            initial_state: Some(state.clone()),
            state,
            history: Vec::new(),
            actions: Vec::new(),
            next_action_id: first_action_id(),
        })
    }

    #[must_use]
    pub const fn state(&self) -> &RegisteredGameState {
        &self.state
    }

    /// Applies one canonical dart event to the registered ruleset.
    ///
    /// # Errors
    ///
    /// Returns an error when the game is not running or its ruleset is unavailable.
    pub fn apply_throw(&mut self, event: &DartEvent) -> Result<&RegisteredGameState, GameError> {
        if self.state.status != GameStatus::Running {
            return Err(GameError::NotRunning);
        }
        let mode = self.resolve_mode()?;
        self.ensure_timeline();
        apply_throw_to_state(mode, &mut self.state, event)?;
        let action_id = self.take_action_id();
        self.actions.push(RegisteredAction::Dart {
            id: action_id,
            event: event.clone(),
        });
        self.refresh_editable_darts()?;
        Ok(&self.state)
    }

    /// Advances a held game to the next player's turn.
    ///
    /// # Errors
    ///
    /// Returns an error unless the game is holding or its ruleset is unavailable.
    pub fn continue_turn(&mut self) -> Result<&RegisteredGameState, GameError> {
        if self.state.status != GameStatus::Hold {
            return Err(GameError::NotHolding);
        }
        let mode = self.resolve_mode()?;
        self.ensure_timeline();
        advance_player(mode, &mut self.state)?;
        let action_id = self.take_action_id();
        self.actions
            .push(RegisteredAction::Continue { id: action_id });
        self.refresh_editable_darts()?;
        Ok(&self.state)
    }

    /// Ends the current visit early or advances an already held visit.
    ///
    /// # Errors
    ///
    /// Returns an error after the game has finished or when the ruleset cannot
    /// complete its skipped-turn transition.
    pub fn next_player(&mut self) -> Result<&RegisteredGameState, GameError> {
        if self.state.status == GameStatus::Finished {
            return Err(GameError::NotRunning);
        }
        let mode = self.resolve_mode()?;
        self.ensure_timeline();
        apply_player_boundary(mode, &mut self.state, true)?;
        let action_id = self.take_action_id();
        self.actions
            .push(RegisteredAction::NextPlayer { id: action_id });
        self.refresh_editable_darts()?;
        Ok(&self.state)
    }

    /// Applies a mode-defined non-dart action.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported actions, invalid payloads or missing rulesets.
    pub fn handle_action(
        &mut self,
        action: &str,
        payload: &Value,
    ) -> Result<&RegisteredGameState, GameError> {
        let mode = self.resolve_mode()?;
        self.ensure_timeline();
        let mut next = self.state.clone();
        mode.handle_action(&mut next, action, payload)?;
        next.overlay = mode.overlay(&next);
        self.state = next;
        let action_id = self.take_action_id();
        self.actions.push(RegisteredAction::Mode {
            id: action_id,
            action: action.into(),
            payload: payload.clone(),
        });
        self.refresh_editable_darts()?;
        Ok(&self.state)
    }

    /// Restores the state before the latest accepted throw, turn or mode action.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NothingToUndo`] when no prior state exists.
    pub fn undo(&mut self) -> Result<&RegisteredGameState, GameError> {
        if self.initial_state.is_none() {
            self.state = self.history.pop().ok_or(GameError::NothingToUndo)?;
            return Ok(&self.state);
        }
        let action = self.actions.pop().ok_or(GameError::NothingToUndo)?;
        if let Err(error) = self.replay() {
            self.actions.push(action);
            return Err(error);
        }
        Ok(&self.state)
    }

    /// Replaces a dart in the current or immediately previous turn.
    ///
    /// The original sequence number and action ID remain stable.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::ActionNotEditable`] for unknown or older darts, or
    /// a ruleset error when replaying the resulting timeline fails.
    pub fn correct_throw(
        &mut self,
        action_id: u64,
        replacement: DartEvent,
    ) -> Result<&RegisteredGameState, GameError> {
        if !self.editable_dart_ids().contains(&action_id) {
            return Err(GameError::ActionNotEditable);
        }
        let original_actions = self.actions.clone();
        let action = self
            .actions
            .iter_mut()
            .find(|action| action.id() == action_id)
            .ok_or(GameError::ActionNotEditable)?;
        let RegisteredAction::Dart { event, .. } = action else {
            return Err(GameError::ActionNotEditable);
        };
        *event = with_seq(replacement, event.seq());
        if let Err(error) = self.replay() {
            self.actions = original_actions;
            self.replay()?;
            return Err(error);
        }
        Ok(&self.state)
    }

    /// Deletes an editable dart and deterministically replays all later actions.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::ActionNotEditable`] for unknown or older darts, or
    /// a ruleset error when replaying the resulting timeline fails.
    pub fn delete_throw(&mut self, action_id: u64) -> Result<&RegisteredGameState, GameError> {
        if !self.editable_dart_ids().contains(&action_id) {
            return Err(GameError::ActionNotEditable);
        }
        let original_actions = self.actions.clone();
        self.actions.retain(
            |action| !matches!(action, RegisteredAction::Dart { id, .. } if *id == action_id),
        );
        if self.actions.len() == original_actions.len() {
            return Err(GameError::ActionNotEditable);
        }
        if let Err(error) = self.replay() {
            self.actions = original_actions;
            self.replay()?;
            return Err(error);
        }
        Ok(&self.state)
    }

    #[must_use]
    pub fn dart_records(&self) -> Vec<RegisteredDartRecord> {
        let Ok(mode) = self.resolve_mode() else {
            return Vec::new();
        };
        let Some(mut state) = self.initial_state.clone() else {
            return Vec::new();
        };
        state.editable_darts.clear();
        let mut records = Vec::new();
        for action in &self.actions {
            match action {
                RegisteredAction::Dart { id, event } if state.status == GameStatus::Running => {
                    let Some(player) = state.players.get(state.current_player_index) else {
                        continue;
                    };
                    let player_id = player.id.clone();
                    if apply_throw_to_state(mode, &mut state, event).is_err() {
                        continue;
                    }
                    let Some(player) = state.players.iter().find(|player| player.id == player_id)
                    else {
                        continue;
                    };
                    let outcome = if matches!(event, DartEvent::Miss { .. }) {
                        "miss"
                    } else if state.status == GameStatus::Finished {
                        "win"
                    } else {
                        "success"
                    };
                    records.push(RegisteredDartRecord {
                        action_id: *id,
                        event: event.clone(),
                        player_id,
                        score_after: player.score,
                        round_number: state.round_number,
                        dart_in_turn: state.darts_in_turn,
                        outcome: outcome.into(),
                    });
                }
                RegisteredAction::Continue { .. } | RegisteredAction::NextPlayer { .. }
                    if matches!(state.status, GameStatus::Running | GameStatus::Hold) =>
                {
                    let force_skip = matches!(action, RegisteredAction::NextPlayer { .. });
                    if apply_player_boundary(mode, &mut state, force_skip).is_err() {
                        break;
                    }
                }
                RegisteredAction::Mode {
                    action, payload, ..
                } => {
                    if mode.handle_action(&mut state, action, payload).is_ok() {
                        state.overlay = mode.overlay(&state);
                    }
                }
                RegisteredAction::Dart { .. }
                | RegisteredAction::Continue { .. }
                | RegisteredAction::NextPlayer { .. } => {}
            }
        }
        records
    }

    fn replay(&mut self) -> Result<(), GameError> {
        let mode = self.resolve_mode()?;
        let mut state = self
            .initial_state
            .clone()
            .ok_or_else(|| GameError::RulesetUnavailable("missing initial state".into()))?;
        state.editable_darts.clear();
        for action in self.actions.clone() {
            match action {
                RegisteredAction::Dart { event, .. } if state.status == GameStatus::Running => {
                    apply_throw_to_state(mode, &mut state, &event)?;
                }
                RegisteredAction::Continue { .. } | RegisteredAction::NextPlayer { .. }
                    if matches!(state.status, GameStatus::Running | GameStatus::Hold) =>
                {
                    let force_skip = matches!(action, RegisteredAction::NextPlayer { .. });
                    apply_player_boundary(mode, &mut state, force_skip)?;
                }
                RegisteredAction::Mode {
                    action, payload, ..
                } => {
                    mode.handle_action(&mut state, &action, &payload)?;
                    state.overlay = mode.overlay(&state);
                }
                RegisteredAction::Dart { .. }
                | RegisteredAction::Continue { .. }
                | RegisteredAction::NextPlayer { .. } => {}
            }
        }
        self.state = state;
        self.refresh_editable_darts()?;
        Ok(())
    }

    fn editable_dart_ids(&self) -> Vec<u64> {
        let mut turns = vec![Vec::new()];
        for action in &self.actions {
            match action {
                RegisteredAction::Dart { id, .. } => {
                    turns.last_mut().expect("turn").push(*id);
                }
                RegisteredAction::Continue { .. } | RegisteredAction::NextPlayer { .. } => {
                    turns.push(Vec::new());
                }
                RegisteredAction::Mode { .. } => {}
            }
        }
        turns.into_iter().rev().take(2).flatten().collect()
    }

    fn refresh_editable_darts(&mut self) -> Result<(), GameError> {
        self.resolve_mode()?;
        let editable_ids = self.editable_dart_ids();
        self.state.editable_darts = self
            .dart_records()
            .into_iter()
            .filter(|record| editable_ids.contains(&record.action_id))
            .collect();
        Ok(())
    }

    fn take_action_id(&mut self) -> u64 {
        let action_id = self.next_action_id;
        self.next_action_id = self.next_action_id.saturating_add(1);
        action_id
    }

    fn ensure_timeline(&mut self) {
        if self.initial_state.is_none() {
            self.initial_state = Some(self.state.clone());
            self.state.editable_darts.clear();
            self.history.clear();
            self.actions.clear();
            self.next_action_id = first_action_id();
        }
    }

    fn resolve_mode(&self) -> Result<&'static dyn GameMode, GameError> {
        let mode = mode(&self.state.game_type)?;
        if mode.metadata().ruleset_version != self.state.ruleset_version {
            return Err(GameError::RulesetUnavailable(format!(
                "{} version {}",
                self.state.game_type, self.state.ruleset_version
            )));
        }
        Ok(mode)
    }
}

const fn first_action_id() -> u64 {
    1
}

fn apply_throw_to_state(
    mode: &'static dyn GameMode,
    state: &mut RegisteredGameState,
    event: &DartEvent,
) -> Result<(), GameError> {
    let mut next = state.clone();
    let turn_value = mode.apply_throw(&mut next, event)?;
    next.darts_in_turn = next.darts_in_turn.saturating_add(1);
    next.turn_score = next.turn_score.saturating_add(turn_value);
    next.last_event = Some(event.clone());
    if next.status == GameStatus::Running && next.darts_in_turn >= 3 {
        next.status = GameStatus::Hold;
        next.message = "Turn complete. Press continue.".into();
    }
    next.overlay = mode.overlay(&next);
    *state = next;
    Ok(())
}

fn advance_player(
    mode: &'static dyn GameMode,
    state: &mut RegisteredGameState,
) -> Result<(), GameError> {
    let mut next = state.clone();
    let previous_index = next.current_player_index;
    let player_count = next.players.len();
    let mut selected = None;
    for offset in 1..=player_count {
        let candidate = (previous_index + offset) % player_count;
        if mode.is_player_active(&next, candidate) {
            selected = Some(candidate);
            break;
        }
    }
    let selected = selected.ok_or_else(|| {
        GameError::RulesetUnavailable("mode has no active player to continue".into())
    })?;
    if selected <= previous_index {
        next.round_number = next.round_number.saturating_add(1);
    }
    next.current_player_index = selected;
    next.darts_in_turn = 0;
    next.turn_score = 0;
    next.status = GameStatus::Running;
    next.message = "Next player".into();
    next.last_event = None;
    mode.on_turn_started(&mut next)?;
    next.overlay = mode.overlay(&next);
    *state = next;
    Ok(())
}

trait GameMode: Sync {
    fn metadata(&self) -> &'static GameMetadata;
    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError>;
    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError>;
    fn overlay(&self, state: &RegisteredGameState) -> Value;

    fn is_player_active(&self, _state: &RegisteredGameState, _player_index: usize) -> bool {
        true
    }

    fn on_turn_started(&self, _state: &mut RegisteredGameState) -> Result<(), GameError> {
        Ok(())
    }

    fn on_turn_skipped(&self, _state: &mut RegisteredGameState) -> Result<(), GameError> {
        Ok(())
    }

    fn fixed_round_winner_message(&self) -> Option<&'static str> {
        None
    }

    fn handle_action(
        &self,
        _state: &mut RegisteredGameState,
        action: &str,
        _payload: &Value,
    ) -> Result<(), GameError> {
        Err(GameError::UnsupportedAction(action.into()))
    }
}

fn finish_fixed_round_game(
    state: &mut RegisteredGameState,
    winner_message: &str,
) -> Result<(), GameError> {
    let is_turn_end = state.darts_in_turn.saturating_add(1) >= 3;
    if !is_turn_end {
        return Ok(());
    }
    finish_action_round_game(state, winner_message)
}

fn finish_action_round_game(
    state: &mut RegisteredGameState,
    winner_message: &str,
) -> Result<(), GameError> {
    let is_last_player = state.current_player_index.saturating_add(1) == state.players.len();
    let rounds = state
        .options
        .get("rounds")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(|| GameError::RulesetUnavailable("invalid round count".into()))?;
    if !is_last_player || state.round_number < rounds {
        return Ok(());
    }

    finish_score_game(state, winner_message)
}

fn finish_score_game(
    state: &mut RegisteredGameState,
    winner_message: &str,
) -> Result<(), GameError> {
    let best_score = state
        .players
        .iter()
        .map(|player| player.score)
        .max()
        .ok_or(GameError::NoPlayers)?;
    let leaders = state
        .players
        .iter()
        .filter(|player| player.score == best_score)
        .collect::<Vec<_>>();
    state.status = GameStatus::Finished;
    if leaders.len() == 1 {
        let winner = leaders[0];
        state.winner_id = Some(winner.id.clone());
        state.winner_ids = vec![winner.id.clone()];
        state.result_type = "individual_win".into();
        state.message = winner_message.replace("{winner}", &winner.name);
    } else {
        state.winner_id = None;
        state.winner_ids.clear();
        state.result_type = "draw".into();
        state.message = format!(
            "Unentschieden: {}",
            leaders
                .iter()
                .map(|player| player.name.as_str())
                .collect::<Vec<_>>()
                .join(" · ")
        );
    }
    Ok(())
}

fn apply_player_boundary(
    mode: &'static dyn GameMode,
    state: &mut RegisteredGameState,
    force_skip: bool,
) -> Result<(), GameError> {
    if state.status == GameStatus::Running {
        mode.on_turn_skipped(state)?;
        if state.status == GameStatus::Running
            && let Some(message) = mode.fixed_round_winner_message()
        {
            finish_action_round_game(state, message)?;
        }
    } else if force_skip && state.status != GameStatus::Hold {
        return Err(GameError::NotRunning);
    }
    if state.status != GameStatus::Finished {
        advance_player(mode, state)?;
    }
    Ok(())
}

struct CricketMode;

static CRICKET_INSTRUCTIONS: [GameInstruction; 4] = [
    GameInstruction {
        title: "Ziele treffen",
        body: "Nur 15, 16, 17, 18, 19, 20 und Bull zählen.",
        icon: "targets",
    },
    GameInstruction {
        title: "Dreimal schließen",
        body: "Single zählt einen, Double zwei und Triple drei Marks.",
        icon: "marks",
    },
    GameInstruction {
        title: "Offen punkten",
        body: "Weitere Treffer punkten, solange ein Gegner das Feld offen hat.",
        icon: "lock",
    },
    GameInstruction {
        title: "Sieg",
        body: "Schließe alle Ziele und habe mindestens so viele Punkte wie jeder Gegner.",
        icon: "trophy",
    },
];

static COUNTUP_ROUND_CHOICES: [GameOptionChoice; 3] = [
    GameOptionChoice {
        value: GameOptionValue::Integer(5),
        label: "5 Runden",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(8),
        label: "8 Runden",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(10),
        label: "10 Runden",
        description: None,
        description_en: None,
    },
];

static COUNTUP_OPTIONS: [GameOption; 1] = [GameOption {
    key: "rounds",
    label: "Runden",
    kind: "choice",
    default: GameOptionValue::Integer(8),
    choices: &COUNTUP_ROUND_CHOICES,
}];

static COUNTUP_INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Punkte sammeln",
        body: "Jeder Treffer wird direkt zu deinem Konto addiert.",
        icon: "score",
    },
    GameInstruction {
        title: "Drei Darts",
        body: "Eine Aufnahme besteht aus drei Würfen.",
        icon: "darts",
    },
    GameInstruction {
        title: "Vorne gewinnt",
        body: "Nach der gewählten Rundenzahl gewinnt der höchste Score.",
        icon: "trophy",
    },
];

static COUNTUP_METADATA: GameMetadata = GameMetadata {
    slug: "countup",
    ruleset_version: 1,
    title: "Count Up",
    tagline: "Jeder Punkt zählt",
    description: "Sammelt über mehrere Aufnahmen so viele Punkte wie möglich.",
    accent: "#28e7ff",
    accent_secondary: "#176dff",
    visual: "neon-orbit",
    icon: "target",
    artwork: "/static/assets/modes/countup.webp",
    sound_theme: "arena",
    min_players: 1,
    max_players: 8,
    options: &COUNTUP_OPTIONS,
    instructions: &COUNTUP_INSTRUCTIONS,
    control_legend: &[],
};

static X01_START_CHOICES: [GameOptionChoice; 3] = [
    GameOptionChoice {
        value: GameOptionValue::Integer(301),
        label: "301",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(501),
        label: "501",
        description: None,
        description_en: None,
    },
    GameOptionChoice {
        value: GameOptionValue::Integer(701),
        label: "701",
        description: None,
        description_en: None,
    },
];

static X01_OUT_CHOICES: [GameOptionChoice; 2] = [
    GameOptionChoice {
        value: GameOptionValue::Text("straight"),
        label: "Straight Out",
        description: Some("Jeder Treffer darf das Spiel exakt auf null beenden."),
        description_en: Some("Any hit may finish the game exactly on zero."),
    },
    GameOptionChoice {
        value: GameOptionValue::Text("double"),
        label: "Double Out",
        description: Some("Der letzte Treffer muss ein Double oder Double Bull sein."),
        description_en: Some("The final hit must be a Double or Double Bull."),
    },
];

static X01_OPTIONS: [GameOption; 2] = [
    GameOption {
        key: "start_score",
        label: "Startpunktzahl",
        kind: "choice",
        default: GameOptionValue::Integer(501),
        choices: &X01_START_CHOICES,
    },
    GameOption {
        key: "out_rule",
        label: "Checkout",
        kind: "choice",
        default: GameOptionValue::Text("straight"),
        choices: &X01_OUT_CHOICES,
    },
];

static X01_INSTRUCTIONS: [GameInstruction; 3] = [
    GameInstruction {
        title: "Herunterspielen",
        body: "Jeder Treffer wird von deinem Restscore abgezogen.",
        icon: "subtract",
    },
    GameInstruction {
        title: "Exakt Null",
        body: "Bringe den Score exakt auf null; bei Double Out muss der letzte Dart ein Double sein.",
        icon: "zero",
    },
    GameInstruction {
        title: "Bust",
        body: "Überwirfst du dich, wird die komplette Aufnahme zurückgesetzt.",
        icon: "bust",
    },
];

static X01_METADATA: GameMetadata = GameMetadata {
    slug: "x01",
    ruleset_version: 1,
    title: "X01",
    tagline: "Runter auf exakt Null",
    description: "Der Turnierklassiker: 301, 501 oder 701 Punkte präzise herunterspielen.",
    accent: "#ffb52b",
    accent_secondary: "#ff3d5f",
    visual: "championship",
    icon: "crown",
    artwork: "/static/assets/modes/x01.webp",
    sound_theme: "championship",
    min_players: 1,
    max_players: 8,
    options: &X01_OPTIONS,
    instructions: &X01_INSTRUCTIONS,
    control_legend: &[],
};

static CRICKET_METADATA: GameMetadata = GameMetadata {
    slug: "cricket",
    ruleset_version: 1,
    title: "Cricket",
    tagline: "Schließen und punkten",
    description: "Schließe 15 bis 20 und Bull, während du offene Felder deiner Gegner punktest.",
    accent: "#3dff91",
    accent_secondary: "#11a56a",
    visual: "clubhouse",
    icon: "shield",
    artwork: "/static/assets/modes/cricket.webp",
    sound_theme: "club",
    min_players: 1,
    max_players: 8,
    options: &[],
    instructions: &CRICKET_INSTRUCTIONS,
    control_legend: &[],
};

impl GameMode for CricketMode {
    fn metadata(&self) -> &'static GameMetadata {
        &CRICKET_METADATA
    }

    fn initialize(&self, state: &mut RegisteredGameState) -> Result<(), GameError> {
        for player in &mut state.players {
            player.score = 0;
            player.marks = CRICKET_TARGETS
                .into_iter()
                .map(|target| (target.to_string(), 0))
                .collect();
        }
        Ok(())
    }

    fn apply_throw(
        &self,
        state: &mut RegisteredGameState,
        event: &DartEvent,
    ) -> Result<i64, GameError> {
        let DartEvent::Hit {
            field,
            multiplier,
            label,
            ..
        } = event
        else {
            let player = &state.players[state.current_player_index];
            state.message = format!("{}: kein Cricket-Ziel", player.name);
            return Ok(0);
        };
        if !CRICKET_TARGETS.contains(field) || *multiplier == 0 {
            let player = &state.players[state.current_player_index];
            state.message = format!("{}: kein Cricket-Ziel", player.name);
            return Ok(0);
        }
        let key = field.to_string();
        let player_id = state.players[state.current_player_index].id.clone();
        let before = state.players[state.current_player_index]
            .marks
            .get(&key)
            .copied()
            .unwrap_or_default();
        let total = before.saturating_add(*multiplier);
        let overflow = total.saturating_sub(3);
        state.players[state.current_player_index]
            .marks
            .insert(key.clone(), total.min(3));
        let opponents_open = state.players.iter().any(|player| {
            player.id != player_id && player.marks.get(&key).copied().unwrap_or_default() < 3
        });
        let scored = if opponents_open {
            i64::from(overflow) * i64::from(*field)
        } else {
            0
        };
        let player = &mut state.players[state.current_player_index];
        player.score = player.score.saturating_add(scored);
        state.message = format!("{}: {label}", player.name);
        let closed_all = CRICKET_TARGETS.iter().all(|target| {
            player
                .marks
                .get(&target.to_string())
                .copied()
                .unwrap_or_default()
                >= 3
        });
        let player_score = player.score;
        let leading = state
            .players
            .iter()
            .filter(|candidate| candidate.id != player_id)
            .all(|candidate| player_score >= candidate.score);
        if closed_all && leading {
            let player = &state.players[state.current_player_index];
            state.status = GameStatus::Finished;
            state.winner_id = Some(player.id.clone());
            state.winner_ids = vec![player.id.clone()];
            state.result_type = "individual_win".into();
            state.message = format!("{} gewinnt!", player.name);
        }
        Ok(scored)
    }

    fn overlay(&self, state: &RegisteredGameState) -> Value {
        let Some(player) = state.players.get(state.current_player_index) else {
            return json!({"prompt": "Cricket", "targets": [], "cricket": {"remaining": []}});
        };
        let mut remaining = Vec::new();
        let mut targets = Vec::new();
        for field in CRICKET_TARGETS {
            let marks = player
                .marks
                .get(&field.to_string())
                .copied()
                .unwrap_or_default()
                .min(3);
            let needed = 3 - marks;
            if needed == 0 {
                continue;
            }
            remaining.push(json!({
                "field": field,
                "label": if field == 25 { "BULL".into() } else { field.to_string() },
                "marks": marks,
                "needed": needed,
            }));
            let rings: &[Ring] = if field == 25 {
                &[Ring::SingleBull, Ring::DoubleBull]
            } else {
                &[
                    Ring::SingleInner,
                    Ring::Triple,
                    Ring::SingleOuter,
                    Ring::Double,
                ]
            };
            for ring in rings {
                targets.push(json!({
                    "id": format!("cricket-{field}-{}", ring_name(*ring)),
                    "field": field,
                    "ring": ring,
                    "color": "green",
                    "label": "",
                    "pulse": false,
                }));
            }
        }
        json!({
            "prompt": "Offene Cricket-Ziele",
            "targets": targets,
            "cricket": {"remaining": remaining},
        })
    }
}

static CRICKET_MODE: CricketMode = CricketMode;
static MODES: [&'static dyn GameMode; 20] = [
    &AVOID_BOMB_MODE,
    &BLOCK_DROP_MODE,
    &CRICKET_MODE,
    &CANDY_CANNON_MODE,
    &COLOR_CLASH_MODE,
    &COOKIE_MONSTER_MODE,
    &DART_SWEEPER_MODE,
    &DRAGON_EGGS_MODE,
    &EIGHT_BALL_MODE,
    &GHOST_CHASE_MODE,
    &HEART_CHASE_MODE,
    &KING_OF_BOARD_MODE,
    &LIGHTNING_ROUND_MODE,
    &MINI_GOLF_MODE,
    &ROBIN_HOOD_MODE,
    &RISK_IT_MODE,
    &SIMON_SAYS_MODE,
    &SPACE_DEFENDER_MODE,
    &TARGET_RUSH_MODE,
    &TREASURE_HUNT_MODE,
];

fn mode(slug: &str) -> Result<&'static dyn GameMode, GameError> {
    MODES
        .iter()
        .copied()
        .find(|mode| mode.metadata().slug == slug)
        .ok_or_else(|| GameError::UnknownMode(slug.into()))
}

#[must_use]
pub fn registered_game_metadata() -> Vec<&'static GameMetadata> {
    let mut metadata = vec![&COUNTUP_METADATA, &X01_METADATA];
    metadata.extend(MODES.iter().map(|mode| mode.metadata()));
    metadata.sort_by_key(|metadata| metadata.slug);
    metadata
}

#[must_use]
pub fn game_metadata(slug: &str) -> Option<&'static GameMetadata> {
    match slug {
        "countup" => Some(&COUNTUP_METADATA),
        "x01" => Some(&X01_METADATA),
        _ => mode(slug).ok().map(GameMode::metadata),
    }
}

fn resolve_options(metadata: &GameMetadata, provided: &Value) -> Result<Value, GameError> {
    let supplied = provided
        .as_object()
        .ok_or_else(|| GameError::InvalidOptions("options must be an object".into()))?;
    let mut resolved = Map::new();
    for option in metadata.options {
        resolved.insert(option.key.into(), option.default.as_json());
    }
    for (key, value) in supplied {
        let option = metadata
            .options
            .iter()
            .find(|option| option.key == key)
            .ok_or_else(|| GameError::InvalidOptions(format!("unknown option: {key}")))?;
        let type_matches = matches!(
            (option.default, value),
            (GameOptionValue::Integer(_), Value::Number(number)) if number.is_i64() || number.is_u64()
        ) || matches!(
            (option.default, value),
            (GameOptionValue::Boolean(_), Value::Bool(_))
        ) || matches!(
            (option.default, value),
            (GameOptionValue::Text(_), Value::String(_))
        );
        if !type_matches {
            return Err(GameError::InvalidOptions(format!(
                "invalid value for {}",
                option.key
            )));
        }
        match value {
            Value::Number(number) => {
                let within_limits = number
                    .as_i64()
                    .is_some_and(|value| (-100_000..=100_000).contains(&value))
                    || number.as_u64().is_some_and(|value| value <= 100_000);
                if !within_limits {
                    return Err(GameError::InvalidOptions(format!(
                        "value for {} is outside the supported range",
                        option.key
                    )));
                }
            }
            Value::String(value) if value.len() > 64 => {
                return Err(GameError::InvalidOptions(format!(
                    "value for {} is too long",
                    option.key
                )));
            }
            _ => {}
        }
        if !option.choices.is_empty()
            && !option
                .choices
                .iter()
                .any(|choice| choice.value.as_json() == *value)
        {
            return Err(GameError::InvalidOptions(format!(
                "unsupported value for {}",
                option.key
            )));
        }
        resolved.insert(key.clone(), value.clone());
    }
    Ok(Value::Object(resolved))
}

const fn ring_name(ring: Ring) -> &'static str {
    match ring {
        Ring::SingleInner => "single_inner",
        Ring::SingleOuter => "single_outer",
        Ring::Triple => "triple",
        Ring::Double => "double",
        Ring::SingleBull => "single_bull",
        Ring::DoubleBull => "double_bull",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(seq: u64, field: u8, ring: Ring, multiplier: u8, label: &str) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring,
            multiplier,
            label: label.into(),
            score: u16::from(field) * u16::from(multiplier),
        }
    }

    #[test]
    fn registry_exposes_complete_cricket_metadata() {
        let metadata = game_metadata("cricket").expect("Cricket metadata");
        assert_eq!(metadata.ruleset_version, 1);
        assert_eq!(metadata.instructions.len(), 4);
        assert!(metadata.artwork.ends_with("cricket.webp"));
        assert!(game_metadata("missing").is_none());
    }

    #[test]
    fn registry_rejects_unknown_options_without_mutating_game_state() {
        let result = RegisteredGame::new(
            "cricket",
            vec![("ada".into(), "Ada".into())],
            &json!({"surprise": true}),
        );
        assert!(matches!(result, Err(GameError::InvalidOptions(_))));
    }

    #[test]
    fn registered_corrections_replay_the_current_and_previous_turn() {
        let mut game =
            RegisteredGame::new("cricket", vec![("ada".into(), "Ada".into())], &json!({}))
                .expect("game");
        game.apply_throw(&hit(1, 20, Ring::Triple, 3, "T20"))
            .expect("first");
        game.apply_throw(&hit(2, 19, Ring::Triple, 3, "T19"))
            .expect("second");
        game.apply_throw(&hit(3, 18, Ring::Triple, 3, "T18"))
            .expect("third");
        game.continue_turn().expect("continue");
        game.apply_throw(&hit(4, 17, Ring::Triple, 3, "T17"))
            .expect("fourth");

        game.correct_throw(
            1,
            DartEvent::Miss {
                seq: 999,
                label: "MISS".into(),
                score: 0,
            },
        )
        .expect("correct previous turn");
        assert_eq!(game.state().players[0].marks["20"], 0);
        assert_eq!(game.dart_records()[0].event.seq(), 1);
        game.delete_throw(2).expect("delete previous turn");
        assert_eq!(game.state().players[0].marks["19"], 0);
        assert_eq!(game.state().players[0].marks["17"], 3);

        game.apply_throw(&DartEvent::Miss {
            seq: 5,
            label: "MISS".into(),
            score: 0,
        })
        .expect("fifth");
        game.apply_throw(&DartEvent::Miss {
            seq: 6,
            label: "MISS".into(),
            score: 0,
        })
        .expect("sixth");
        game.continue_turn().expect("second continue");
        assert_eq!(
            game.correct_throw(1, hit(7, 20, Ring::Triple, 3, "T20")),
            Err(GameError::ActionNotEditable)
        );
    }

    #[test]
    fn first_registry_snapshot_format_remains_restorable() {
        let mut game =
            RegisteredGame::new("cricket", vec![("ada".into(), "Ada".into())], &json!({}))
                .expect("game");
        let initial = game.state().clone();
        game.apply_throw(&hit(1, 20, Ring::Triple, 3, "T20"))
            .expect("throw");
        let legacy = json!({
            "state": game.state(),
            "history": [initial],
        });
        let mut restored: RegisteredGame =
            serde_json::from_value(legacy).expect("legacy registry snapshot");

        restored.undo().expect("legacy undo");
        assert_eq!(restored.state().players[0].marks["20"], 0);
        restored
            .apply_throw(&hit(2, 19, Ring::Triple, 3, "T19"))
            .expect("new timeline action");
        assert_eq!(restored.state().editable_darts[0].action_id, 1);
    }

    #[test]
    fn injected_random_seed_is_reproducible_and_cursor_is_serialized() {
        let players = vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())];
        let mut first = RegisteredGame::new_seeded("eight_ball", players.clone(), &json!({}), 42)
            .expect("first");
        let mut second =
            RegisteredGame::new_seeded("eight_ball", players, &json!({}), 42).expect("second");
        let first_sequence = (0..8)
            .map(|_| first.state.random_index(97).expect("random index"))
            .collect::<Vec<_>>();
        let second_sequence = (0..8)
            .map(|_| second.state.random_index(97).expect("random index"))
            .collect::<Vec<_>>();
        assert_eq!(first_sequence, second_sequence);
        assert_eq!(first.state.random_cursor, 8);

        let restored: RegisteredGame =
            serde_json::from_str(&serde_json::to_string(&first).expect("serialize"))
                .expect("restore");
        assert_eq!(restored.state().random_seed, 42);
        assert_eq!(restored.state().random_cursor, 8);
    }

    #[test]
    fn registered_profiles_preserve_avatar_and_color() {
        let game = RegisteredGame::new_seeded_with_players(
            "eight_ball",
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
            &json!({}),
            42,
        )
        .expect("game");

        assert_eq!(game.state().players[0].avatar, "fox");
        assert_eq!(game.state().players[0].color, "#ff00aa");
        assert_eq!(game.state().players[1].avatar, "comet");
        assert_eq!(game.state().players[1].color, "#00ffaa");
    }

    #[test]
    fn legacy_registered_players_receive_profile_defaults() {
        let game = RegisteredGame::new(
            "eight_ball",
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            &json!({}),
        )
        .expect("game");
        let mut serialized = serde_json::to_value(game).expect("serialize");
        for player in serialized["state"]["players"]
            .as_array_mut()
            .expect("players")
        {
            player.as_object_mut().expect("player").remove("avatar");
            player.as_object_mut().expect("player").remove("color");
        }

        let restored: RegisteredGame = serde_json::from_value(serialized).expect("legacy game");
        assert_eq!(restored.state().players[0].avatar, "comet");
        assert_eq!(restored.state().players[0].color, "#28e7ff");
    }

    #[test]
    fn signed_arcade_scores_survive_snapshot_round_trips() {
        let mut game = RegisteredGame::new_seeded(
            "target_rush",
            vec![("ada".into(), "Ada".into())],
            &json!({"rounds": 3, "difficulty": "normal"}),
            42,
        )
        .expect("game");
        game.state.players[0].score = -40;
        game.state.turn_score = -40;

        let restored: RegisteredGame = serde_json::from_str(
            &serde_json::to_string(&game).expect("serialize signed registry state"),
        )
        .expect("restore signed registry state");

        assert_eq!(restored.state().players[0].score, -40);
        assert_eq!(restored.state().turn_score, -40);
    }
}
