//! Deterministic session, scoring and screen-flow rules.
//!
//! This crate does not create IDs, choose random starters, persist data or
//! execute game rules. Hosts inject IDs and notify it about committed results.

use sdb_contracts::PlayerRef;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Screen {
    #[default]
    Attract,
    Players,
    GameSelect,
    Instructions,
    Countdown,
    Playing,
    GameResult,
    SessionSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Finished,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StarterSelection {
    #[default]
    Rotation,
    Manual,
    Random,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparedGame {
    pub game_type: String,
    pub options: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStanding {
    pub player_id: String,
    pub games: u32,
    pub wins: u32,
    pub session_points: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionState {
    pub screen: Screen,
    pub session_id: Option<String>,
    pub session_status: Option<SessionStatus>,
    pub players: Vec<PlayerRef>,
    pub prepared_game: Option<PreparedGame>,
    pub game_id: Option<String>,
    pub game_player_ids: Vec<String>,
    pub default_starter_id: Option<String>,
    pub selected_starter_id: Option<String>,
    pub starter_selection: StarterSelection,
    pub standings: Vec<SessionStanding>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            screen: Screen::Attract,
            session_id: None,
            session_status: None,
            players: Vec::new(),
            prepared_game: None,
            game_id: None,
            game_player_ids: Vec::new(),
            default_starter_id: None,
            selected_starter_id: None,
            starter_selection: StarterSelection::Rotation,
            standings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionCore {
    state: SessionState,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("a session requires at least one player")]
    NoPlayers,
    #[error("session player IDs must be unique")]
    DuplicatePlayer,
    #[error("an active session is required")]
    NoActiveSession,
    #[error("a prepared game is required")]
    NoPreparedGame,
    #[error("a running game is required")]
    NoRunningGame,
    #[error("a finished game is required")]
    NoFinishedGame,
    #[error("selected starter is not part of the session")]
    InvalidStarter,
    #[error("winner is not part of the game")]
    InvalidWinner,
    #[error("return to game selection before ending the session")]
    InvalidEndScreen,
}

impl SessionCore {
    #[must_use]
    pub const fn state(&self) -> &SessionState {
        &self.state
    }

    /// Starts a session from host-provided stable IDs.
    ///
    /// # Errors
    ///
    /// Rejects empty lineups and duplicate player IDs.
    pub fn start_session(
        &mut self,
        session_id: impl Into<String>,
        players: Vec<PlayerRef>,
    ) -> Result<&SessionState, SessionError> {
        if players.is_empty() {
            return Err(SessionError::NoPlayers);
        }
        let mut ids: Vec<&str> = players.iter().map(|player| player.id.as_str()).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SessionError::DuplicatePlayer);
        }
        let starter = players[0].id.clone();
        self.state = SessionState {
            screen: Screen::GameSelect,
            session_id: Some(session_id.into()),
            session_status: Some(SessionStatus::Active),
            standings: players
                .iter()
                .map(|player| SessionStanding {
                    player_id: player.id.clone(),
                    games: 0,
                    wins: 0,
                    session_points: 0,
                })
                .collect(),
            players,
            prepared_game: None,
            game_id: None,
            game_player_ids: Vec::new(),
            default_starter_id: Some(starter.clone()),
            selected_starter_id: Some(starter),
            starter_selection: StarterSelection::Rotation,
        };
        Ok(&self.state)
    }

    /// Selects and validates the next mode outside the game-rule layer.
    ///
    /// # Errors
    ///
    /// Requires an active session with no running game.
    pub fn prepare_game(
        &mut self,
        game_type: impl Into<String>,
        options: Value,
    ) -> Result<&SessionState, SessionError> {
        self.require_active_session()?;
        if self.state.game_id.is_some() {
            return Err(SessionError::NoFinishedGame);
        }
        self.state.prepared_game = Some(PreparedGame {
            game_type: game_type.into(),
            options,
        });
        self.state.screen = Screen::Instructions;
        Ok(&self.state)
    }

    /// Chooses a host-selected manual or deterministic random starter.
    ///
    /// # Errors
    ///
    /// Rejects players outside the active session.
    pub fn select_starter(
        &mut self,
        player_id: &str,
        selection: StarterSelection,
    ) -> Result<&SessionState, SessionError> {
        self.require_active_session()?;
        if !self
            .state
            .players
            .iter()
            .any(|player| player.id == player_id)
        {
            return Err(SessionError::InvalidStarter);
        }
        self.state.selected_starter_id = Some(player_id.into());
        self.state.starter_selection = selection;
        Ok(&self.state)
    }

    /// Starts the prepared game and returns its rotated player order.
    ///
    /// # Errors
    ///
    /// Requires an active session, prepared game and valid starter.
    pub fn start_game(
        &mut self,
        game_id: impl Into<String>,
    ) -> Result<Vec<PlayerRef>, SessionError> {
        self.require_active_session()?;
        if self.state.prepared_game.is_none() {
            return Err(SessionError::NoPreparedGame);
        }
        let starter = self
            .state
            .selected_starter_id
            .as_deref()
            .ok_or(SessionError::InvalidStarter)?;
        let starter_index = self
            .state
            .players
            .iter()
            .position(|player| player.id == starter)
            .ok_or(SessionError::InvalidStarter)?;
        let ordered: Vec<PlayerRef> = self.state.players[starter_index..]
            .iter()
            .chain(&self.state.players[..starter_index])
            .cloned()
            .collect();
        self.state.game_player_ids = ordered.iter().map(|player| player.id.clone()).collect();
        self.state.game_id = Some(game_id.into());
        self.state.screen = Screen::Countdown;
        Ok(ordered)
    }

    /// Moves the started game to its live screen.
    ///
    /// # Errors
    ///
    /// Requires a currently running game record.
    pub fn mark_playing(&mut self) -> Result<&SessionState, SessionError> {
        if self.state.game_id.is_none() || self.state.screen != Screen::Countdown {
            return Err(SessionError::NoRunningGame);
        }
        self.state.screen = Screen::Playing;
        Ok(&self.state)
    }

    /// Records a committed game result and awards three points per winner.
    ///
    /// # Errors
    ///
    /// Requires a running game and winners contained in its lineup.
    pub fn complete_game(&mut self, winner_ids: &[String]) -> Result<&SessionState, SessionError> {
        if self.state.game_id.is_none()
            || !matches!(self.state.screen, Screen::Countdown | Screen::Playing)
        {
            return Err(SessionError::NoRunningGame);
        }
        if winner_ids
            .iter()
            .any(|winner| !self.state.game_player_ids.contains(winner))
        {
            return Err(SessionError::InvalidWinner);
        }
        for standing in &mut self.state.standings {
            standing.games += 1;
            if winner_ids.contains(&standing.player_id) {
                standing.wins += 1;
                standing.session_points += 3;
            }
        }
        self.state.screen = Screen::GameResult;
        Ok(&self.state)
    }

    /// Clears a finished game and rotates the next default starter.
    ///
    /// # Errors
    ///
    /// Requires the game-result screen.
    pub fn next_game(&mut self) -> Result<&SessionState, SessionError> {
        if self.state.screen != Screen::GameResult || self.state.game_player_ids.is_empty() {
            return Err(SessionError::NoFinishedGame);
        }
        let previous_starter = self.state.game_player_ids[0].clone();
        let next_starter = self.next_player_id(&previous_starter)?;
        self.clear_game_selection();
        self.state.default_starter_id = Some(next_starter.clone());
        self.state.selected_starter_id = Some(next_starter);
        self.state.screen = Screen::GameSelect;
        Ok(&self.state)
    }

    /// Starts the same mode immediately with the player order rotated once.
    ///
    /// # Errors
    ///
    /// Requires a finished game with a retained prepared mode.
    pub fn start_rematch(
        &mut self,
        game_id: impl Into<String>,
    ) -> Result<Vec<PlayerRef>, SessionError> {
        if self.state.screen != Screen::GameResult
            || self.state.game_player_ids.is_empty()
            || self.state.prepared_game.is_none()
        {
            return Err(SessionError::NoFinishedGame);
        }
        self.state.game_player_ids.rotate_left(1);
        let next_starter = self.state.game_player_ids[0].clone();
        self.state.default_starter_id = Some(next_starter.clone());
        self.state.selected_starter_id = Some(next_starter);
        self.state.starter_selection = StarterSelection::Rotation;
        self.state.game_id = Some(game_id.into());
        self.state.screen = Screen::Countdown;
        let players = self
            .state
            .game_player_ids
            .iter()
            .map(|id| {
                self.state
                    .players
                    .iter()
                    .find(|player| &player.id == id)
                    .cloned()
                    .ok_or(SessionError::InvalidStarter)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(players)
    }

    /// Aborts a running game without changing standings or starter rotation.
    ///
    /// # Errors
    ///
    /// Requires a running game.
    pub fn abort_game(&mut self) -> Result<&SessionState, SessionError> {
        if self.state.game_id.is_none()
            || !matches!(self.state.screen, Screen::Countdown | Screen::Playing)
        {
            return Err(SessionError::NoRunningGame);
        }
        self.clear_game_selection();
        self.state.selected_starter_id = self.state.default_starter_id.clone();
        self.state.screen = Screen::GameSelect;
        Ok(&self.state)
    }

    /// Finishes a session only from game selection.
    ///
    /// # Errors
    ///
    /// Rejects attempts to skip directly out of an active game or result.
    pub fn end_session(&mut self) -> Result<&SessionState, SessionError> {
        self.require_active_session()?;
        if self.state.screen != Screen::GameSelect {
            return Err(SessionError::InvalidEndScreen);
        }
        self.state.session_status = Some(SessionStatus::Finished);
        self.state.screen = Screen::SessionSummary;
        Ok(&self.state)
    }

    pub fn close_session(&mut self) {
        self.state = SessionState::default();
    }

    fn require_active_session(&self) -> Result<(), SessionError> {
        if self.state.session_status != Some(SessionStatus::Active) {
            return Err(SessionError::NoActiveSession);
        }
        Ok(())
    }

    fn next_player_id(&self, current: &str) -> Result<String, SessionError> {
        let index = self
            .state
            .players
            .iter()
            .position(|player| player.id == current)
            .ok_or(SessionError::InvalidStarter)?;
        Ok(self.state.players[(index + 1) % self.state.players.len()]
            .id
            .clone())
    }

    fn clear_game_selection(&mut self) {
        self.state.prepared_game = None;
        self.state.game_id = None;
        self.state.game_player_ids.clear();
        self.state.starter_selection = StarterSelection::Rotation;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players() -> Vec<PlayerRef> {
        vec![
            PlayerRef {
                id: "ada".into(),
                name: "Ada".into(),
                avatar: "nova".into(),
                color: "#ff00aa".into(),
            },
            PlayerRef {
                id: "bob".into(),
                name: "Bob".into(),
                avatar: "comet".into(),
                color: "#28e7ff".into(),
            },
        ]
    }

    #[test]
    fn manual_starter_survives_repreparing_before_start() {
        let mut session = SessionCore::default();
        session
            .start_session("session", players())
            .expect("session");
        session
            .prepare_game("countup", serde_json::json!({"rounds": 5}))
            .expect("prepare");
        session
            .select_starter("bob", StarterSelection::Manual)
            .expect("starter");
        session
            .prepare_game("countup", serde_json::json!({"rounds": 8}))
            .expect("reprepare");
        let lineup = session.start_game("game").expect("start");
        assert_eq!(lineup[0].id, "bob");
        assert_eq!(session.state.starter_selection, StarterSelection::Manual);
    }

    #[test]
    fn invalid_winner_does_not_mutate_standings() {
        let mut session = SessionCore::default();
        session
            .start_session("session", players())
            .expect("session");
        session
            .prepare_game("countup", serde_json::json!({"rounds": 5}))
            .expect("prepare");
        session.start_game("game").expect("start");
        let before = session.state.clone();
        let error = session
            .complete_game(&["mallory".into()])
            .expect_err("unknown winner");
        assert_eq!(error, SessionError::InvalidWinner);
        assert_eq!(session.state, before);
    }

    #[test]
    fn session_cannot_end_from_a_result_screen() {
        let mut session = SessionCore::default();
        session
            .start_session("session", players())
            .expect("session");
        session
            .prepare_game("countup", serde_json::json!({"rounds": 5}))
            .expect("prepare");
        session.start_game("game").expect("start");
        session.complete_game(&[]).expect("draw");
        assert_eq!(
            session.end_session().expect_err("must return first"),
            SessionError::InvalidEndScreen
        );
    }
}
