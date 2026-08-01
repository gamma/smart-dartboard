//! Deterministic, platform-independent game rules.
//!
//! Count Up is the first parity slice. Further modes use the same transition
//! boundary and are added only together with shared Python/Rust fixtures.

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CountUpSnapshot {
    state: CountUpState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountUpGame {
    state: CountUpState,
    history: Vec<CountUpSnapshot>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GameError {
    #[error("count up requires at least one player")]
    NoPlayers,
    #[error("round count must be greater than zero")]
    InvalidRounds,
    #[error("game is not accepting darts")]
    NotRunning,
    #[error("turn is not waiting for continue")]
    NotHolding,
    #[error("there is no action to undo")]
    NothingToUndo,
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
        Ok(Self {
            state: CountUpState {
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
            },
            history: Vec::new(),
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
        self.remember();
        let score = u32::from(event.score());
        let player = &mut self.state.players[self.state.current_player_index];
        player.score += score;
        let player_name = player.name.clone();
        self.state.darts_in_turn += 1;
        self.state.turn_score += score;
        self.state.message = format!("{player_name}: {}", event.label());

        let last_dart = self.state.darts_in_turn == 3;
        let last_player = self.state.current_player_index + 1 == self.state.players.len();
        if last_dart && last_player && self.state.round_number >= self.state.rounds {
            self.finish();
        } else if last_dart {
            self.state.status = GameStatus::Hold;
            self.state.message = "Turn complete. Press continue.".into();
        }
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
        self.remember();
        let last_player = self.state.current_player_index + 1 == self.state.players.len();
        self.state.current_player_index =
            (self.state.current_player_index + 1) % self.state.players.len();
        if last_player {
            self.state.round_number += 1;
        }
        self.state.darts_in_turn = 0;
        self.state.turn_score = 0;
        self.state.status = GameStatus::Running;
        self.state.message = "Next player".into();
        Ok(&self.state)
    }

    /// Reverts the last accepted throw or continue transition.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NothingToUndo`] when no transition exists.
    pub fn undo(&mut self) -> Result<&CountUpState, GameError> {
        let snapshot = self.history.pop().ok_or(GameError::NothingToUndo)?;
        self.state = snapshot.state;
        Ok(&self.state)
    }

    fn remember(&mut self) {
        self.history.push(CountUpSnapshot {
            state: self.state.clone(),
        });
    }

    fn finish(&mut self) {
        self.state.status = GameStatus::Finished;
        let high_score = self
            .state
            .players
            .iter()
            .map(|player| player.score)
            .max()
            .unwrap_or_default();
        let winners: Vec<&Player> = self
            .state
            .players
            .iter()
            .filter(|player| player.score == high_score)
            .collect();
        if winners.len() == 1 {
            self.state.winner_id = Some(winners[0].id.clone());
            self.state.winner_ids = vec![winners[0].id.clone()];
            self.state.result_type = "individual_win".into();
            self.state.message = format!("{} gewinnt Count Up!", winners[0].name);
        } else {
            self.state.winner_id = None;
            self.state.winner_ids.clear();
            self.state.result_type = "draw".into();
            self.state.message = "Unentschieden bei Count Up!".into();
        }
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
}
