//! Deterministic session, scoring and screen-flow rules.
//!
//! This crate does not create IDs, choose random starters, persist data or
//! execute game rules. Hosts inject IDs and notify it about committed results.

pub use sdb_contracts::StarterSelection;
use sdb_contracts::{GameFormat, PlayerRef, TeamRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const REMATCH_CONFIRM_MILLISECONDS: u64 = 5_000;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedGame {
    pub game_type: String,
    pub options: Value,
    #[serde(default)]
    pub format: GameFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStanding {
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    pub games: u32,
    pub wins: u32,
    pub session_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    pub screen: Screen,
    pub session_id: Option<String>,
    pub session_status: Option<SessionStatus>,
    pub players: Vec<PlayerRef>,
    #[serde(default)]
    pub teams: Vec<TeamRef>,
    pub prepared_game: Option<PreparedGame>,
    pub game_id: Option<String>,
    pub game_player_ids: Vec<String>,
    #[serde(default)]
    pub active_game_counted: bool,
    #[serde(default)]
    pub active_game_winner_ids: Vec<String>,
    #[serde(default)]
    pub active_game_teams: Vec<TeamRef>,
    #[serde(default)]
    pub active_game_winner_team_ids: Vec<String>,
    pub default_starter_id: Option<String>,
    pub selected_starter_id: Option<String>,
    pub starter_selection: StarterSelection,
    pub standings: Vec<SessionStanding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rematch_armed_until_ms: Option<u64>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            screen: Screen::Attract,
            session_id: None,
            session_status: None,
            players: Vec::new(),
            teams: Vec::new(),
            prepared_game: None,
            game_id: None,
            game_player_ids: Vec::new(),
            active_game_counted: false,
            active_game_winner_ids: Vec::new(),
            active_game_teams: Vec::new(),
            active_game_winner_team_ids: Vec::new(),
            default_starter_id: None,
            selected_starter_id: None,
            starter_selection: StarterSelection::Rotation,
            standings: Vec::new(),
            rematch_armed_until_ms: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCore {
    state: SessionState,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("a session requires at least one player")]
    NoPlayers,
    #[error("a session supports at most eight players")]
    TooManyPlayers,
    #[error("session ID is invalid")]
    InvalidSessionId,
    #[error("player ID, name, avatar or color is invalid")]
    InvalidPlayer,
    #[error("session player IDs must be unique")]
    DuplicatePlayer,
    #[error("team definition is invalid")]
    InvalidTeam,
    #[error("team IDs must be unique")]
    DuplicateTeam,
    #[error("team players must form an exact partition of the session lineup")]
    InvalidTeamPlayers,
    #[error("this game requires at least two configured teams")]
    TeamsRequired,
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
        self.start_session_with_teams(session_id, players, Vec::new())
    }

    /// Starts a session with an optional explicit competitive-team partition.
    ///
    /// # Errors
    ///
    /// Rejects invalid players, teams or assignments.
    pub fn start_session_with_teams(
        &mut self,
        session_id: impl Into<String>,
        mut players: Vec<PlayerRef>,
        teams: Vec<TeamRef>,
    ) -> Result<&SessionState, SessionError> {
        if players.is_empty() {
            return Err(SessionError::NoPlayers);
        }
        if players.len() > 8 {
            return Err(SessionError::TooManyPlayers);
        }
        let session_id = session_id.into();
        if session_id.is_empty() || session_id.len() > 128 {
            return Err(SessionError::InvalidSessionId);
        }
        if players.iter().any(|player| !valid_player(player)) {
            return Err(SessionError::InvalidPlayer);
        }
        let mut ids: Vec<&str> = players.iter().map(|player| player.id.as_str()).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SessionError::DuplicatePlayer);
        }
        validate_teams(&players, &teams)?;
        for player in &mut players {
            player.team_id = teams
                .iter()
                .find(|team| team.player_ids.contains(&player.id))
                .map(|team| team.id.clone());
        }
        let starter = players[0].id.clone();
        self.state = SessionState {
            screen: Screen::GameSelect,
            session_id: Some(session_id),
            session_status: Some(SessionStatus::Active),
            standings: players
                .iter()
                .map(|player| SessionStanding {
                    player_id: player.id.clone(),
                    team_id: player.team_id.clone(),
                    games: 0,
                    wins: 0,
                    session_points: 0,
                })
                .collect(),
            players,
            teams,
            prepared_game: None,
            game_id: None,
            game_player_ids: Vec::new(),
            active_game_counted: false,
            active_game_winner_ids: Vec::new(),
            active_game_teams: Vec::new(),
            active_game_winner_team_ids: Vec::new(),
            default_starter_id: Some(starter.clone()),
            selected_starter_id: Some(starter),
            starter_selection: StarterSelection::Rotation,
            rematch_armed_until_ms: None,
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
        self.prepare_game_with_format(game_type, options, GameFormat::Individual)
    }

    /// Selects a mode and materializes its active team configuration.
    ///
    /// # Errors
    ///
    /// Requires configured teams when the mode declares team competition.
    pub fn prepare_game_with_format(
        &mut self,
        game_type: impl Into<String>,
        options: Value,
        format: GameFormat,
    ) -> Result<&SessionState, SessionError> {
        self.require_active_session()?;
        if self.state.game_id.is_some() {
            return Err(SessionError::NoFinishedGame);
        }
        self.state.active_game_teams = match format {
            GameFormat::Individual => Vec::new(),
            GameFormat::Cooperative => vec![TeamRef {
                id: "coop".into(),
                name: "Team".into(),
                color: self.state.players[0].color.clone(),
                player_ids: self
                    .state
                    .players
                    .iter()
                    .map(|player| player.id.clone())
                    .collect(),
            }],
            GameFormat::Teams if self.state.teams.len() >= 2 => self.state.teams.clone(),
            GameFormat::Teams => return Err(SessionError::TeamsRequired),
        };
        self.state.active_game_winner_team_ids.clear();
        self.state.prepared_game = Some(PreparedGame {
            game_type: game_type.into(),
            options,
            format,
        });
        self.state.screen = Screen::Instructions;
        Ok(&self.state)
    }

    /// Returns from instructions to mode selection without starting a game.
    ///
    /// # Errors
    ///
    /// Requires an active session with a currently prepared game.
    pub fn cancel_prepared_game(&mut self) -> Result<&SessionState, SessionError> {
        self.require_active_session()?;
        if self.state.screen != Screen::Instructions || self.state.prepared_game.is_none() {
            return Err(SessionError::NoPreparedGame);
        }
        self.state.prepared_game = None;
        self.state.active_game_teams.clear();
        self.state.active_game_winner_team_ids.clear();
        self.state.selected_starter_id = self.state.default_starter_id.clone();
        self.state.starter_selection = StarterSelection::Rotation;
        self.state.screen = Screen::GameSelect;
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
        let mut ordered: Vec<PlayerRef> = self.state.players[starter_index..]
            .iter()
            .chain(&self.state.players[..starter_index])
            .cloned()
            .collect();
        apply_active_team_ids(&mut ordered, &self.state.active_game_teams);
        self.state.game_player_ids = ordered.iter().map(|player| player.id.clone()).collect();
        self.state.game_id = Some(game_id.into());
        self.state.active_game_counted = false;
        self.state.active_game_winner_ids.clear();
        self.state.active_game_winner_team_ids.clear();
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
        self.state.active_game_counted = true;
        self.state.active_game_winner_ids = winner_ids.to_vec();
        self.state.active_game_winner_team_ids = self
            .state
            .active_game_teams
            .iter()
            .filter(|team| {
                !team.player_ids.is_empty()
                    && team
                        .player_ids
                        .iter()
                        .all(|player_id| winner_ids.contains(player_id))
            })
            .map(|team| team.id.clone())
            .collect();
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
        self.state.active_game_counted = false;
        self.state.active_game_winner_ids.clear();
        self.state.active_game_winner_team_ids.clear();
        self.state.rematch_armed_until_ms = None;
        self.state.screen = Screen::Countdown;
        let mut players = self
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
        apply_active_team_ids(&mut players, &self.state.active_game_teams);
        Ok(players)
    }

    /// Arms a board-button rematch or confirms it within the short arcade window.
    ///
    /// The first press only updates public session state. A second press before
    /// the deadline returns `true`; the runtime then starts the retained mode
    /// atomically with a fresh game ID.
    ///
    /// # Errors
    ///
    /// Requires a finished game with a retained prepared mode.
    pub fn press_rematch_button(&mut self, now_ms: u64) -> Result<bool, SessionError> {
        if self.state.screen != Screen::GameResult
            || self.state.game_player_ids.is_empty()
            || self.state.prepared_game.is_none()
        {
            return Err(SessionError::NoFinishedGame);
        }
        if self
            .state
            .rematch_armed_until_ms
            .is_some_and(|deadline| now_ms < deadline)
        {
            self.state.rematch_armed_until_ms = None;
            return Ok(true);
        }
        self.state.rematch_armed_until_ms =
            Some(now_ms.saturating_add(REMATCH_CONFIRM_MILLISECONDS));
        Ok(false)
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

    /// Reopens the just-finished game and reverses its standings exactly once.
    ///
    /// # Errors
    ///
    /// Requires a counted game on the result screen.
    pub fn reopen_game(&mut self) -> Result<&SessionState, SessionError> {
        if self.state.screen != Screen::GameResult || !self.state.active_game_counted {
            return Err(SessionError::NoFinishedGame);
        }
        for standing in &mut self.state.standings {
            standing.games = standing.games.saturating_sub(1);
            if self
                .state
                .active_game_winner_ids
                .contains(&standing.player_id)
            {
                standing.wins = standing.wins.saturating_sub(1);
                standing.session_points = standing.session_points.saturating_sub(3);
            }
        }
        self.state.active_game_counted = false;
        self.state.active_game_winner_ids.clear();
        self.state.active_game_winner_team_ids.clear();
        self.state.rematch_armed_until_ms = None;
        self.state.screen = Screen::Playing;
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
        self.state.active_game_counted = false;
        self.state.active_game_winner_ids.clear();
        self.state.active_game_teams.clear();
        self.state.active_game_winner_team_ids.clear();
        self.state.rematch_armed_until_ms = None;
        self.state.starter_selection = StarterSelection::Rotation;
    }
}

fn valid_player(player: &PlayerRef) -> bool {
    let valid_color = player.color.len() == 7
        && player.color.starts_with('#')
        && player.color[1..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit());
    !player.id.is_empty()
        && player.id.len() <= 128
        && !player.name.trim().is_empty()
        && player.name.chars().count() <= 32
        && !player.avatar.is_empty()
        && player.avatar.len() <= 32
        && valid_color
}

fn validate_teams(players: &[PlayerRef], teams: &[TeamRef]) -> Result<(), SessionError> {
    if teams.is_empty() {
        return Ok(());
    }
    if teams.len() < 2 || teams.len() > players.len() {
        return Err(SessionError::InvalidTeam);
    }
    if teams.iter().any(|team| !valid_team(team)) {
        return Err(SessionError::InvalidTeam);
    }
    let mut team_ids: Vec<&str> = teams.iter().map(|team| team.id.as_str()).collect();
    team_ids.sort_unstable();
    if team_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SessionError::DuplicateTeam);
    }
    let mut assigned: Vec<&str> = teams
        .iter()
        .flat_map(|team| team.player_ids.iter().map(String::as_str))
        .collect();
    assigned.sort_unstable();
    let mut player_ids: Vec<&str> = players.iter().map(|player| player.id.as_str()).collect();
    player_ids.sort_unstable();
    if assigned != player_ids {
        return Err(SessionError::InvalidTeamPlayers);
    }
    Ok(())
}

fn valid_team(team: &TeamRef) -> bool {
    !team.id.is_empty()
        && team.id.len() <= 128
        && !team.name.trim().is_empty()
        && team.name.chars().count() <= 32
        && valid_color(&team.color)
        && !team.player_ids.is_empty()
}

fn valid_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn apply_active_team_ids(players: &mut [PlayerRef], teams: &[TeamRef]) {
    for player in players {
        player.team_id = teams
            .iter()
            .find(|team| team.player_ids.contains(&player.id))
            .map(|team| team.id.clone());
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
                team_id: None,
            },
            PlayerRef {
                id: "bob".into(),
                name: "Bob".into(),
                avatar: "comet".into(),
                color: "#28e7ff".into(),
                team_id: None,
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
    fn cancelling_instructions_returns_to_game_selection() {
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

        session.cancel_prepared_game().expect("cancel");

        assert_eq!(session.state.screen, Screen::GameSelect);
        assert!(session.state.prepared_game.is_none());
        assert_eq!(session.state.selected_starter_id.as_deref(), Some("ada"));
        assert_eq!(session.state.starter_selection, StarterSelection::Rotation);
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

    #[test]
    fn board_rematch_requires_two_presses_inside_the_confirmation_window() {
        let mut session = SessionCore::default();
        session
            .start_session("session", players())
            .expect("session");
        session
            .prepare_game("countup", serde_json::json!({"rounds": 5}))
            .expect("prepare");
        session.start_game("game-1").expect("start");
        session.complete_game(&["ada".into()]).expect("finish");

        assert!(!session.press_rematch_button(1_000).expect("arm"));
        assert_eq!(
            session.state.rematch_armed_until_ms,
            Some(1_000 + REMATCH_CONFIRM_MILLISECONDS)
        );
        assert!(
            !session
                .press_rematch_button(1_000 + REMATCH_CONFIRM_MILLISECONDS)
                .expect("expired press rearms")
        );
        assert!(
            session
                .press_rematch_button(1_001 + REMATCH_CONFIRM_MILLISECONDS)
                .expect("confirm")
        );
        let lineup = session.start_rematch("game-2").expect("rematch");
        assert_eq!(lineup[0].id, "bob");
        assert_eq!(session.state.screen, Screen::Countdown);
        assert!(session.state.rematch_armed_until_ms.is_none());
    }

    #[test]
    fn legacy_session_snapshots_default_to_an_unarmed_rematch() {
        let mut value = serde_json::to_value(SessionState::default()).expect("serialize");
        value
            .as_object_mut()
            .expect("session object")
            .remove("rematch_armed_until_ms");
        let restored: SessionState = serde_json::from_value(value).expect("legacy snapshot");
        assert!(restored.rematch_armed_until_ms.is_none());
    }

    #[test]
    fn reopening_a_win_reverses_games_wins_and_points() {
        let mut session = SessionCore::default();
        session
            .start_session("session", players())
            .expect("session");
        session
            .prepare_game("countup", serde_json::json!({"rounds": 5}))
            .expect("prepare");
        session.start_game("game").expect("start");
        session.complete_game(&["ada".into()]).expect("finish");
        session.reopen_game().expect("reopen");
        assert_eq!(session.state.screen, Screen::Playing);
        assert!(
            session
                .state
                .standings
                .iter()
                .all(|standing| standing.games == 0
                    && standing.wins == 0
                    && standing.session_points == 0)
        );
        assert!(!session.state.active_game_counted);
    }

    #[test]
    fn invalid_player_profiles_are_rejected_before_state_changes() {
        let mut invalid = players();
        invalid[0].color = "pink".into();
        let mut session = SessionCore::default();
        assert_eq!(
            session
                .start_session("session", invalid)
                .expect_err("invalid profile"),
            SessionError::InvalidPlayer
        );
        assert_eq!(session.state, SessionState::default());
    }

    #[test]
    fn cooperative_game_materializes_one_team_and_awards_every_member() {
        let mut session = SessionCore::default();
        session
            .start_session("session", players())
            .expect("session");
        session
            .prepare_game_with_format("boss_fight", serde_json::json!({}), GameFormat::Cooperative)
            .expect("prepare cooperative game");
        assert_eq!(session.state.active_game_teams.len(), 1);
        assert_eq!(session.state.active_game_teams[0].id, "coop");
        assert_eq!(
            session.state.active_game_teams[0].player_ids,
            ["ada", "bob"]
        );

        let lineup = session.start_game("game").expect("start");
        assert!(
            lineup
                .iter()
                .all(|player| player.team_id.as_deref() == Some("coop"))
        );
        session
            .complete_game(&["ada".into(), "bob".into()])
            .expect("team win");
        assert_eq!(session.state.active_game_winner_team_ids, ["coop"]);
        assert!(session.state.standings.iter().all(|standing| {
            standing.games == 1 && standing.wins == 1 && standing.session_points == 3
        }));

        session.reopen_game().expect("reopen");
        assert!(session.state.active_game_winner_team_ids.is_empty());
        assert_eq!(session.state.active_game_teams[0].id, "coop");
    }

    #[test]
    fn competitive_team_partition_is_validated_before_use() {
        let teams = vec![
            TeamRef {
                id: "cyan".into(),
                name: "Team Cyan".into(),
                color: "#28e7ff".into(),
                player_ids: vec!["ada".into()],
            },
            TeamRef {
                id: "pink".into(),
                name: "Team Pink".into(),
                color: "#ff00aa".into(),
                player_ids: vec!["bob".into()],
            },
        ];
        let mut session = SessionCore::default();
        session
            .start_session_with_teams("session", players(), teams.clone())
            .expect("team session");
        session
            .prepare_game_with_format("future_team_mode", Value::Null, GameFormat::Teams)
            .expect("team game");
        assert_eq!(session.state.active_game_teams, teams);

        let mut invalid = teams;
        invalid[1].player_ids = vec!["ada".into()];
        let mut rejected = SessionCore::default();
        assert_eq!(
            rejected
                .start_session_with_teams("session", players(), invalid)
                .expect_err("duplicate assignment"),
            SessionError::InvalidTeamPlayers
        );

        let mut missing = SessionCore::default();
        missing
            .start_session("session", players())
            .expect("session");
        assert_eq!(
            missing
                .prepare_game_with_format("future_team_mode", Value::Null, GameFormat::Teams)
                .expect_err("teams required"),
            SessionError::TeamsRequired
        );
    }
}
