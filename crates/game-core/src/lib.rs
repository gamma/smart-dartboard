//! Deterministic, platform-independent game rules.
//!
//! Count Up and X01 are the first parity slices. Further modes use the static
//! registered-mode boundary and are added only together with shared
//! Python/Rust fixtures.

mod registered;

pub use registered::{
    GameInstruction, GameMetadata, GameOption, GameOptionChoice, GameOptionValue,
    RegisteredDartRecord, RegisteredGame, RegisteredGameState, RegisteredPlayer, game_metadata,
    registered_game_metadata,
};

use sdb_contracts::DartEvent;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    Running,
    Hold,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub score: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountUpState {
    pub players: Vec<Player>,
    pub current_player_index: usize,
    pub darts_in_turn: u8,
    pub turn_score: u32,
    pub round_number: u16,
    pub rounds: u16,
    pub status: GameStatus,
    pub winner_id: Option<String>,
    pub winner_ids: Vec<String>,
    pub result_type: String,
    pub message: String,
    #[serde(default)]
    pub editable_darts: Vec<CountUpDartRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CountUpSnapshot {
    state: CountUpState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountUpDartRecord {
    pub action_id: u64,
    pub event: DartEvent,
    pub player_id: String,
    pub score_after: u32,
    pub round_number: u16,
    pub dart_in_turn: u8,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CountUpAction {
    Dart { id: u64, event: DartEvent },
    Continue { id: u64 },
}

impl CountUpAction {
    const fn id(&self) -> u64 {
        match self {
            Self::Dart { id, .. } | Self::Continue { id } => *id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountUpGame {
    state: CountUpState,
    #[serde(default)]
    history: Vec<CountUpSnapshot>,
    #[serde(default)]
    initial_state: Option<CountUpState>,
    #[serde(default)]
    actions: Vec<CountUpAction>,
    #[serde(default = "first_action_id")]
    next_action_id: u64,
}

const fn first_action_id() -> u64 {
    1
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GameError {
    #[error("a game requires at least one player")]
    NoPlayers,
    #[error("round count must be greater than zero")]
    InvalidRounds,
    #[error("game is not accepting darts")]
    NotRunning,
    #[error("turn is not waiting for continue")]
    NotHolding,
    #[error("there is no action to undo")]
    NothingToUndo,
    #[error("X01 start score must be greater than zero")]
    InvalidStartScore,
    #[error("only darts from the current or previous turn can be edited")]
    ActionNotEditable,
    #[error("unknown registered game mode: {0}")]
    UnknownMode(String),
    #[error("registered game ruleset is unavailable: {0}")]
    RulesetUnavailable(String),
    #[error("invalid registered game options: {0}")]
    InvalidOptions(String),
    #[error("registered game action is unsupported: {0}")]
    UnsupportedAction(String),
}

impl CountUpGame {
    /// Starts a deterministic Count Up game.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NoPlayers`] for an empty player list and
    /// [`GameError::InvalidRounds`] for zero rounds.
    pub fn new(players: Vec<(String, String)>, rounds: u16) -> Result<Self, GameError> {
        if players.is_empty() {
            return Err(GameError::NoPlayers);
        }
        if rounds == 0 {
            return Err(GameError::InvalidRounds);
        }
        let state = CountUpState {
            players: players
                .into_iter()
                .map(|(id, name)| Player { id, name, score: 0 })
                .collect(),
            current_player_index: 0,
            darts_in_turn: 0,
            turn_score: 0,
            round_number: 1,
            rounds,
            status: GameStatus::Running,
            winner_id: None,
            winner_ids: Vec::new(),
            result_type: String::new(),
            message: "Game started".into(),
            editable_darts: Vec::new(),
        };
        Ok(Self {
            initial_state: Some(state.clone()),
            state,
            history: Vec::new(),
            actions: Vec::new(),
            next_action_id: first_action_id(),
        })
    }

    #[must_use]
    pub const fn state(&self) -> &CountUpState {
        &self.state
    }

    /// Applies one canonical dart to the current player.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NotRunning`] while the game is holding or finished.
    pub fn apply_throw(&mut self, event: &DartEvent) -> Result<&CountUpState, GameError> {
        if self.state.status != GameStatus::Running {
            return Err(GameError::NotRunning);
        }
        self.ensure_timeline();
        Self::apply_throw_internal(&mut self.state, event);
        let action_id = self.take_action_id();
        self.actions.push(CountUpAction::Dart {
            id: action_id,
            event: event.clone(),
        });
        self.refresh_editable_darts();
        Ok(&self.state)
    }

    /// Advances to the next player after a completed three-dart turn.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NotHolding`] unless the current turn is holding.
    pub fn continue_turn(&mut self) -> Result<&CountUpState, GameError> {
        if self.state.status != GameStatus::Hold {
            return Err(GameError::NotHolding);
        }
        self.ensure_timeline();
        Self::advance_player(&mut self.state);
        let action_id = self.take_action_id();
        self.actions.push(CountUpAction::Continue { id: action_id });
        self.refresh_editable_darts();
        Ok(&self.state)
    }

    /// Reverts the last accepted throw or continue transition.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NothingToUndo`] when no transition exists.
    pub fn undo(&mut self) -> Result<&CountUpState, GameError> {
        if self.initial_state.is_none() {
            let snapshot = self.history.pop().ok_or(GameError::NothingToUndo)?;
            self.state = snapshot.state;
            return Ok(&self.state);
        }
        self.actions.pop().ok_or(GameError::NothingToUndo)?;
        self.replay();
        Ok(&self.state)
    }

    /// Replaces a dart in the current or immediately previous turn.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::ActionNotEditable`] for unknown or older darts.
    pub fn correct_throw(
        &mut self,
        action_id: u64,
        replacement: DartEvent,
    ) -> Result<&CountUpState, GameError> {
        if !self.editable_dart_ids().contains(&action_id) {
            return Err(GameError::ActionNotEditable);
        }
        let action = self
            .actions
            .iter_mut()
            .find(|action| action.id() == action_id)
            .ok_or(GameError::ActionNotEditable)?;
        let CountUpAction::Dart { event, .. } = action else {
            return Err(GameError::ActionNotEditable);
        };
        *event = with_seq(replacement, event.seq());
        self.replay();
        Ok(&self.state)
    }

    /// Deletes a dart in the current or immediately previous turn.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::ActionNotEditable`] for unknown or older darts.
    pub fn delete_throw(&mut self, action_id: u64) -> Result<&CountUpState, GameError> {
        if !self.editable_dart_ids().contains(&action_id) {
            return Err(GameError::ActionNotEditable);
        }
        let before = self.actions.len();
        self.actions
            .retain(|action| !matches!(action, CountUpAction::Dart { id, .. } if *id == action_id));
        if self.actions.len() == before {
            return Err(GameError::ActionNotEditable);
        }
        self.replay();
        Ok(&self.state)
    }

    #[must_use]
    pub fn dart_records(&self) -> Vec<CountUpDartRecord> {
        let Some(mut state) = self.initial_state.clone() else {
            return Vec::new();
        };
        state.editable_darts.clear();
        let mut records = Vec::new();
        for action in &self.actions {
            match action {
                CountUpAction::Dart { id, event } if state.status == GameStatus::Running => {
                    let player_id = state.players[state.current_player_index].id.clone();
                    Self::apply_throw_internal(&mut state, event);
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
                    records.push(CountUpDartRecord {
                        action_id: *id,
                        event: event.clone(),
                        player_id,
                        score_after: player.score,
                        round_number: state.round_number,
                        dart_in_turn: state.darts_in_turn,
                        outcome: outcome.into(),
                    });
                }
                CountUpAction::Continue { .. }
                    if matches!(state.status, GameStatus::Running | GameStatus::Hold) =>
                {
                    Self::advance_player(&mut state);
                }
                CountUpAction::Dart { .. } | CountUpAction::Continue { .. } => {}
            }
        }
        records
    }

    fn apply_throw_internal(state: &mut CountUpState, event: &DartEvent) {
        let score = u32::from(event.score());
        let player = &mut state.players[state.current_player_index];
        player.score = player.score.saturating_add(score);
        let player_name = player.name.clone();
        state.darts_in_turn = state.darts_in_turn.saturating_add(1);
        state.turn_score = state.turn_score.saturating_add(score);
        state.message = format!("{player_name}: {}", event.label());

        let last_dart = state.darts_in_turn == 3;
        let last_player = state.current_player_index + 1 == state.players.len();
        if last_dart && last_player && state.round_number >= state.rounds {
            Self::finish(state);
        } else if last_dart {
            state.status = GameStatus::Hold;
            state.message = "Turn complete. Press continue.".into();
        }
    }

    fn advance_player(state: &mut CountUpState) {
        let last_player = state.current_player_index + 1 == state.players.len();
        state.current_player_index = (state.current_player_index + 1) % state.players.len();
        if last_player {
            state.round_number = state.round_number.saturating_add(1);
        }
        state.darts_in_turn = 0;
        state.turn_score = 0;
        state.status = GameStatus::Running;
        state.message = "Next player".into();
    }

    fn replay(&mut self) {
        let Some(mut state) = self.initial_state.clone() else {
            return;
        };
        state.editable_darts.clear();
        for action in self.actions.clone() {
            match action {
                CountUpAction::Dart { event, .. } if state.status == GameStatus::Running => {
                    Self::apply_throw_internal(&mut state, &event);
                }
                CountUpAction::Continue { .. }
                    if matches!(state.status, GameStatus::Running | GameStatus::Hold) =>
                {
                    Self::advance_player(&mut state);
                }
                CountUpAction::Dart { .. } | CountUpAction::Continue { .. } => {}
            }
        }
        self.state = state;
        self.refresh_editable_darts();
    }

    fn editable_dart_ids(&self) -> Vec<u64> {
        let mut turns = vec![Vec::new()];
        for action in &self.actions {
            match action {
                CountUpAction::Dart { id, .. } => turns.last_mut().expect("turn").push(*id),
                CountUpAction::Continue { .. } => turns.push(Vec::new()),
            }
        }
        turns.into_iter().rev().take(2).flatten().collect()
    }

    fn refresh_editable_darts(&mut self) {
        let editable_ids = self.editable_dart_ids();
        self.state.editable_darts = self
            .dart_records()
            .into_iter()
            .filter(|record| editable_ids.contains(&record.action_id))
            .collect();
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

    fn finish(state: &mut CountUpState) {
        state.status = GameStatus::Finished;
        let high_score = state
            .players
            .iter()
            .map(|player| player.score)
            .max()
            .unwrap_or_default();
        let winners: Vec<&Player> = state
            .players
            .iter()
            .filter(|player| player.score == high_score)
            .collect();
        if winners.len() == 1 {
            state.winner_id = Some(winners[0].id.clone());
            state.winner_ids = vec![winners[0].id.clone()];
            state.result_type = "individual_win".into();
            state.message = format!("{} gewinnt Count Up!", winners[0].name);
        } else {
            state.winner_id = None;
            state.winner_ids.clear();
            state.result_type = "draw".into();
            state.message = "Unentschieden bei Count Up!".into();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutRule {
    Straight,
    Double,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct X01State {
    pub players: Vec<Player>,
    pub current_player_index: usize,
    pub darts_in_turn: u8,
    pub turn_score: u32,
    pub round_number: u16,
    pub status: GameStatus,
    pub winner_id: Option<String>,
    pub winner_ids: Vec<String>,
    pub result_type: String,
    pub message: String,
    pub start_score: u32,
    pub out_rule: OutRule,
    pub turn_start_score: u32,
    pub last_bust: bool,
    #[serde(default)]
    pub editable_darts: Vec<X01DartRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct X01DartRecord {
    pub action_id: u64,
    pub event: DartEvent,
    pub player_id: String,
    pub score_after: u32,
    pub round_number: u16,
    pub dart_in_turn: u8,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum X01Action {
    Dart { id: u64, event: DartEvent },
    Continue { id: u64 },
}

impl X01Action {
    const fn id(&self) -> u64 {
        match self {
            Self::Dart { id, .. } | Self::Continue { id } => *id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct X01Game {
    initial_state: X01State,
    state: X01State,
    actions: Vec<X01Action>,
    next_action_id: u64,
}

impl X01Game {
    /// Starts an X01 game with deterministic player IDs and options.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NoPlayers`] for an empty player list and
    /// [`GameError::InvalidStartScore`] for a zero start score.
    pub fn new(
        players: Vec<(String, String)>,
        start_score: u32,
        out_rule: OutRule,
    ) -> Result<Self, GameError> {
        if players.is_empty() {
            return Err(GameError::NoPlayers);
        }
        if start_score == 0 {
            return Err(GameError::InvalidStartScore);
        }
        let state = X01State {
            players: players
                .into_iter()
                .map(|(id, name)| Player {
                    id,
                    name,
                    score: start_score,
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
            start_score,
            out_rule,
            turn_start_score: start_score,
            last_bust: false,
            editable_darts: Vec::new(),
        };
        Ok(Self {
            initial_state: state.clone(),
            state,
            actions: Vec::new(),
            next_action_id: 1,
        })
    }

    #[must_use]
    pub const fn state(&self) -> &X01State {
        &self.state
    }

    /// Applies and records one dart, returning its stable action ID.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NotRunning`] while the game is holding or finished.
    pub fn apply_throw(&mut self, event: DartEvent) -> Result<u64, GameError> {
        if self.state.status != GameStatus::Running {
            return Err(GameError::NotRunning);
        }
        let action_id = self.take_action_id();
        self.apply_throw_internal(&event);
        self.actions.push(X01Action::Dart {
            id: action_id,
            event,
        });
        self.refresh_editable_darts();
        Ok(action_id)
    }

    /// Advances from Hold to the next player and records the turn boundary.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NotHolding`] unless the current turn is holding.
    pub fn continue_turn(&mut self) -> Result<u64, GameError> {
        if self.state.status != GameStatus::Hold {
            return Err(GameError::NotHolding);
        }
        let action_id = self.take_action_id();
        self.advance_player();
        self.actions.push(X01Action::Continue { id: action_id });
        self.refresh_editable_darts();
        Ok(action_id)
    }

    /// Reverts the most recent dart or continue action by deterministic replay.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NothingToUndo`] when the timeline is empty.
    pub fn undo(&mut self) -> Result<&X01State, GameError> {
        self.actions.pop().ok_or(GameError::NothingToUndo)?;
        self.replay();
        Ok(&self.state)
    }

    /// Replaces a dart in the current or immediately previous turn.
    ///
    /// The original sequence number and action ID remain stable.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::ActionNotEditable`] for unknown or older darts.
    pub fn correct_throw(
        &mut self,
        action_id: u64,
        replacement: DartEvent,
    ) -> Result<&X01State, GameError> {
        if !self.editable_dart_ids().contains(&action_id) {
            return Err(GameError::ActionNotEditable);
        }
        let action = self
            .actions
            .iter_mut()
            .find(|action| action.id() == action_id)
            .ok_or(GameError::ActionNotEditable)?;
        let X01Action::Dart { event, .. } = action else {
            return Err(GameError::ActionNotEditable);
        };
        *event = with_seq(replacement, event.seq());
        self.replay();
        Ok(&self.state)
    }

    /// Deletes a dart in the current or immediately previous turn and replays
    /// every later action, preserving explicit player transitions.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::ActionNotEditable`] for unknown or older darts.
    pub fn delete_throw(&mut self, action_id: u64) -> Result<&X01State, GameError> {
        if !self.editable_dart_ids().contains(&action_id) {
            return Err(GameError::ActionNotEditable);
        }
        let before = self.actions.len();
        self.actions
            .retain(|action| !matches!(action, X01Action::Dart { id, .. } if *id == action_id));
        if self.actions.len() == before {
            return Err(GameError::ActionNotEditable);
        }
        self.replay();
        Ok(&self.state)
    }

    #[must_use]
    pub fn dart_actions(&self) -> Vec<(u64, &DartEvent)> {
        self.actions
            .iter()
            .filter_map(|action| match action {
                X01Action::Dart { id, event } => Some((*id, event)),
                X01Action::Continue { .. } => None,
            })
            .collect()
    }

    /// Replays the canonical action timeline into storage-ready dart records.
    ///
    /// Corrected darts keep their original action ID and sequence number.
    /// Deleted darts are absent. Player, turn and score metadata reflect the
    /// fully replayed current timeline rather than the original input order.
    #[must_use]
    pub fn dart_records(&self) -> Vec<X01DartRecord> {
        let mut replay = Self {
            initial_state: self.initial_state.clone(),
            state: self.initial_state.clone(),
            actions: Vec::new(),
            next_action_id: self.next_action_id,
        };
        replay.state.editable_darts.clear();
        let mut records = Vec::new();
        for action in &self.actions {
            match action {
                X01Action::Dart { id, event } if replay.state.status == GameStatus::Running => {
                    let Some(player) = replay.state.players.get(replay.state.current_player_index)
                    else {
                        continue;
                    };
                    let player_id = player.id.clone();
                    replay.apply_throw_internal(event);
                    let Some(player) = replay
                        .state
                        .players
                        .iter()
                        .find(|player| player.id == player_id)
                    else {
                        continue;
                    };
                    let outcome = if replay.state.last_bust {
                        "bust"
                    } else if matches!(event, DartEvent::Miss { .. }) {
                        "miss"
                    } else if replay.state.status == GameStatus::Finished {
                        "checkout"
                    } else {
                        "success"
                    };
                    records.push(X01DartRecord {
                        action_id: *id,
                        event: event.clone(),
                        player_id,
                        score_after: player.score,
                        round_number: replay.state.round_number,
                        dart_in_turn: replay.state.darts_in_turn,
                        outcome: outcome.into(),
                    });
                }
                X01Action::Continue { .. }
                    if matches!(replay.state.status, GameStatus::Running | GameStatus::Hold) =>
                {
                    replay.advance_player();
                }
                X01Action::Dart { .. } | X01Action::Continue { .. } => {}
            }
        }
        records
    }

    fn apply_throw_internal(&mut self, event: &DartEvent) {
        let score = u32::from(event.score());
        let player = &mut self.state.players[self.state.current_player_index];
        let new_score = player.score.checked_sub(score);
        let checkout_invalid = self.state.out_rule == OutRule::Double
            && (new_score == Some(1) || (new_score == Some(0) && event.multiplier() != 2));

        self.state.darts_in_turn += 1;
        self.state.last_bust = new_score.is_none() || checkout_invalid;
        if self.state.last_bust {
            player.score = self.state.turn_start_score;
            self.state.status = GameStatus::Hold;
            self.state.message = "Bust – Aufnahme wird zurückgesetzt".into();
            return;
        }

        let new_score = new_score.expect("non-bust score exists");
        player.score = new_score;
        self.state.turn_score += score;
        self.state.message = format!("{}: {}", player.name, event.label());
        if new_score == 0 {
            self.state.status = GameStatus::Finished;
            self.state.winner_id = Some(player.id.clone());
            self.state.winner_ids = vec![player.id.clone()];
            self.state.result_type = "individual_win".into();
            self.state.message = format!("{} gewinnt!", player.name);
        } else if self.state.darts_in_turn >= 3 {
            self.state.status = GameStatus::Hold;
            self.state.message = "Turn complete. Press continue.".into();
        }
    }

    fn advance_player(&mut self) {
        let last_player = self.state.current_player_index + 1 == self.state.players.len();
        self.state.current_player_index =
            (self.state.current_player_index + 1) % self.state.players.len();
        if last_player {
            self.state.round_number += 1;
        }
        self.state.darts_in_turn = 0;
        self.state.turn_score = 0;
        self.state.status = GameStatus::Running;
        self.state.last_bust = false;
        self.state.turn_start_score = self.state.players[self.state.current_player_index].score;
        self.state.message = "Next player".into();
    }

    fn replay(&mut self) {
        let actions = self.actions.clone();
        self.state = self.initial_state.clone();
        for action in actions {
            match action {
                X01Action::Dart { event, .. } => {
                    if self.state.status == GameStatus::Running {
                        self.apply_throw_internal(&event);
                    }
                }
                X01Action::Continue { .. } => {
                    if matches!(self.state.status, GameStatus::Running | GameStatus::Hold) {
                        self.advance_player();
                    }
                }
            }
        }
        self.refresh_editable_darts();
    }

    fn editable_dart_ids(&self) -> Vec<u64> {
        let mut turns = vec![Vec::new()];
        for action in &self.actions {
            match action {
                X01Action::Dart { id, .. } => turns.last_mut().expect("turn").push(*id),
                X01Action::Continue { .. } => turns.push(Vec::new()),
            }
        }
        turns.into_iter().rev().take(2).flatten().collect()
    }

    fn refresh_editable_darts(&mut self) {
        let editable_ids = self.editable_dart_ids();
        self.state.editable_darts = self
            .dart_records()
            .into_iter()
            .filter(|record| editable_ids.contains(&record.action_id))
            .collect();
    }

    fn take_action_id(&mut self) -> u64 {
        let action_id = self.next_action_id;
        self.next_action_id += 1;
        action_id
    }
}

pub(crate) fn with_seq(event: DartEvent, seq: u64) -> DartEvent {
    match event {
        DartEvent::Hit {
            field,
            ring,
            multiplier,
            label,
            score,
            ..
        } => DartEvent::Hit {
            seq,
            field,
            ring,
            multiplier,
            label,
            score,
        },
        DartEvent::Miss { label, score, .. } => DartEvent::Miss { seq, label, score },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdb_contracts::Ring;

    fn t20(seq: u64) -> DartEvent {
        DartEvent::Hit {
            seq,
            field: 20,
            ring: Ring::Triple,
            multiplier: 3,
            label: "T20".into(),
            score: 60,
        }
    }

    fn single(seq: u64, field: u8) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring: Ring::SingleInner,
            multiplier: 1,
            label: format!("S{field}"),
            score: u16::from(field),
        }
    }

    fn double(seq: u64, field: u8) -> DartEvent {
        DartEvent::Hit {
            seq,
            field,
            ring: Ring::Double,
            multiplier: 2,
            label: format!("D{field}"),
            score: u16::from(field) * 2,
        }
    }

    #[test]
    fn count_up_holds_after_three_darts() {
        let mut game = CountUpGame::new(
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            5,
        )
        .expect("game");
        game.apply_throw(&t20(1)).expect("dart 1");
        game.apply_throw(&t20(2)).expect("dart 2");
        game.apply_throw(&t20(3)).expect("dart 3");
        assert_eq!(game.state.players[0].score, 180);
        assert_eq!(game.state.status, GameStatus::Hold);
        game.continue_turn().expect("continue");
        assert_eq!(game.state.current_player_index, 1);
    }

    #[test]
    fn undo_reverts_continue_and_throw() {
        let mut game = CountUpGame::new(vec![("ada".into(), "Ada".into())], 5).expect("game");
        for seq in 1..=3 {
            game.apply_throw(&t20(seq)).expect("dart");
        }
        game.continue_turn().expect("continue");
        game.undo().expect("undo continue");
        assert_eq!(game.state.status, GameStatus::Hold);
        game.undo().expect("undo third dart");
        assert_eq!(game.state.darts_in_turn, 2);
        assert_eq!(game.state.players[0].score, 120);
    }

    #[test]
    fn count_up_correction_and_deletion_replay_player_transitions() {
        let mut game = CountUpGame::new(
            vec![("ada".into(), "Ada".into()), ("bob".into(), "Bob".into())],
            5,
        )
        .expect("game");
        for seq in 1..=3 {
            game.apply_throw(&single(seq, 20)).expect("Ada dart");
        }
        game.continue_turn().expect("continue");
        game.apply_throw(&single(4, 10)).expect("Bob dart");

        game.correct_throw(1, t20(999)).expect("correct Ada dart");
        assert_eq!(game.state.players[0].score, 100);
        assert_eq!(game.state.current_player_index, 1);
        assert_eq!(game.dart_records()[0].event.seq(), 1);
        game.delete_throw(3).expect("delete Ada dart");
        assert_eq!(game.state.players[0].score, 80);
        assert_eq!(game.state.players[1].score, 10);
        assert_eq!(game.state.current_player_index, 1);
    }

    #[test]
    fn first_count_up_snapshot_format_remains_restorable() {
        let mut game = CountUpGame::new(vec![("ada".into(), "Ada".into())], 5).expect("game");
        let initial = game.state().clone();
        game.apply_throw(&single(1, 20)).expect("throw");
        let legacy = serde_json::json!({
            "state": game.state(),
            "history": [{"state": initial}],
        });
        let mut restored: CountUpGame =
            serde_json::from_value(legacy).expect("legacy CountUp snapshot");

        restored.undo().expect("legacy undo");
        assert_eq!(restored.state().players[0].score, 0);
        restored.apply_throw(&t20(2)).expect("new timeline action");
        assert_eq!(restored.state().editable_darts[0].action_id, 1);
    }

    #[test]
    fn x01_double_out_finishes_only_on_a_double() {
        let players = vec![("ada".into(), "Ada".into())];
        let mut invalid = X01Game::new(players.clone(), 20, OutRule::Double).expect("game");
        invalid.apply_throw(single(1, 20)).expect("dart");
        assert_eq!(invalid.state.players[0].score, 20);
        assert_eq!(invalid.state.status, GameStatus::Hold);
        assert!(invalid.state.last_bust);

        let mut valid = X01Game::new(players, 40, OutRule::Double).expect("game");
        valid.apply_throw(double(1, 20)).expect("dart");
        assert_eq!(valid.state.players[0].score, 0);
        assert_eq!(valid.state.status, GameStatus::Finished);
        assert_eq!(valid.state.winner_id.as_deref(), Some("ada"));
    }

    #[test]
    fn x01_bust_restores_turn_start_and_keeps_prior_turn_value() {
        let mut game =
            X01Game::new(vec![("ada".into(), "Ada".into())], 101, OutRule::Straight).expect("game");
        game.apply_throw(t20(1)).expect("first dart");
        game.apply_throw(t20(2)).expect("bust dart");
        assert_eq!(game.state.players[0].score, 101);
        assert_eq!(game.state.turn_score, 60);
        assert_eq!(game.state.darts_in_turn, 2);
        assert_eq!(game.state.status, GameStatus::Hold);
        assert!(game.state.last_bust);
    }

    #[test]
    fn x01_correction_and_deletion_replay_later_darts() {
        let mut game =
            X01Game::new(vec![("ada".into(), "Ada".into())], 301, OutRule::Straight).expect("game");
        let first = game.apply_throw(single(1, 20)).expect("first dart");
        let second = game.apply_throw(single(2, 20)).expect("second dart");

        game.correct_throw(first, t20(99)).expect("correction");
        assert_eq!(game.state.players[0].score, 221);
        assert_eq!(game.state.turn_score, 80);
        assert_eq!(game.dart_actions()[0].1.seq(), 1);
        let records = game.dart_records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].action_id, first);
        assert_eq!(records[0].event.label(), "T20");
        assert_eq!(records[0].score_after, 241);
        assert_eq!(records[1].score_after, 221);
        assert_eq!(game.state.editable_darts.len(), 2);
        assert_eq!(game.state.editable_darts[0].action_id, first);

        game.delete_throw(second).expect("deletion");
        assert_eq!(game.state.players[0].score, 241);
        assert_eq!(game.state.turn_score, 60);
        assert_eq!(game.state.darts_in_turn, 1);
        let records = game.dart_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action_id, first);
        assert_eq!(records[0].dart_in_turn, 1);
        assert_eq!(game.state.editable_darts.len(), 1);

        game.undo().expect("undo remaining dart");
        assert_eq!(game.state.players[0].score, 301);
        assert_eq!(game.state.darts_in_turn, 0);
    }

    #[test]
    fn x01_undo_restores_a_finished_game() {
        let mut game =
            X01Game::new(vec![("ada".into(), "Ada".into())], 40, OutRule::Straight).expect("game");
        game.apply_throw(double(1, 20)).expect("checkout");
        game.undo().expect("undo checkout");
        assert_eq!(game.state.players[0].score, 40);
        assert_eq!(game.state.status, GameStatus::Running);
        assert_eq!(game.state.winner_id, None);
    }
}
