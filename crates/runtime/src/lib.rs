//! Serialized authoritative runtime.
//!
//! A transition is calculated on a clone, committed through [`Repository`],
//! and only then installed as visible in-memory state. This preserves the
//! product's state when persistence fails between a dart and its broadcast.

use sdb_contracts::{
    CommandEnvelope, ContractError, DartEvent, ErrorCode, PROTOCOL_VERSION, PlayerRef,
    RuntimeCommand, StarterSelection,
};
use sdb_game_core::{CountUpGame, CountUpState, GameError, GameStatus, OutRule, X01Game, X01State};
use sdb_session_core::{Screen, SessionCore, SessionError, SessionState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeAction {
    StartSession {
        session_id: String,
        players: Vec<PlayerRef>,
    },
    PrepareGame {
        game_type: String,
        options: serde_json::Value,
    },
    StartPreparedGame {
        game_id: String,
    },
    MarkGamePlaying,
    SelectStarter {
        player_id: String,
        selection: StarterSelection,
    },
    NextGame,
    StartRematch {
        game_id: String,
    },
    AbortGame,
    EndSession,
    CloseSession,
    StartCountUp {
        players: Vec<(String, String)>,
        rounds: u16,
    },
    StartX01 {
        players: Vec<(String, String)>,
        start_score: u32,
        out_rule: OutRule,
    },
    Dart {
        event: DartEvent,
    },
    Continue,
    Undo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    pub revision: u64,
    pub game: Option<RuntimeGame>,
    #[serde(default)]
    pub session: SessionCore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "game_type", content = "game", rename_all = "snake_case")]
pub enum RuntimeGame {
    CountUp(CountUpGame),
    X01(X01Game),
}

impl RuntimeGame {
    fn apply_throw(&mut self, event: DartEvent) -> Result<(), GameError> {
        match self {
            Self::CountUp(game) => game.apply_throw(&event).map(|_| ()),
            Self::X01(game) => game.apply_throw(event).map(|_| ()),
        }
    }

    fn continue_turn(&mut self) -> Result<(), GameError> {
        match self {
            Self::CountUp(game) => game.continue_turn().map(|_| ()),
            Self::X01(game) => game.continue_turn().map(|_| ()),
        }
    }

    fn undo(&mut self) -> Result<(), GameError> {
        match self {
            Self::CountUp(game) => game.undo().map(|_| ()),
            Self::X01(game) => game.undo().map(|_| ()),
        }
    }

    #[must_use]
    pub fn state(&self) -> RuntimeGameState {
        match self {
            Self::CountUp(game) => RuntimeGameState::CountUp(game.state().clone()),
            Self::X01(game) => RuntimeGameState::X01(game.state().clone()),
        }
    }

    fn is_finished(&self) -> bool {
        match self {
            Self::CountUp(game) => game.state().status == GameStatus::Finished,
            Self::X01(game) => game.state().status == GameStatus::Finished,
        }
    }

    fn winner_ids(&self) -> &[String] {
        match self {
            Self::CountUp(game) => &game.state().winner_ids,
            Self::X01(game) => &game.state().winner_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "game_type", content = "state", rename_all = "snake_case")]
pub enum RuntimeGameState {
    CountUp(CountUpState),
    X01(X01State),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub command_id: String,
    pub revision: u64,
    pub duplicate: bool,
    pub state: Option<RuntimeGameState>,
    #[serde(default)]
    pub session: SessionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRequest<'a> {
    pub command_id: &'a str,
    pub runtime_instance_id: &'a str,
    pub previous_revision: u64,
    pub next_revision: u64,
    pub action_json: &'a str,
    pub snapshot_json: &'a str,
    pub result_json: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOutcome {
    Committed,
    Duplicate(String),
}

pub trait Repository {
    /// Loads the last committed runtime snapshot.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific diagnostic when reading fails.
    fn load_snapshot(&self) -> Result<Option<String>, String>;

    /// Loads a previously committed result for idempotent command handling.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific diagnostic when reading fails.
    fn load_command_result(&self, command_id: &str) -> Result<Option<String>, String>;

    /// Atomically stores the next revision and its command result.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific diagnostic when the transaction fails or the
    /// expected previous revision no longer matches.
    fn commit(&mut self, request: CommitRequest<'_>) -> Result<CommitOutcome, String>;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("wrong runtime instance")]
    WrongRuntimeInstance,
    #[error("stale revision: expected {expected}, current {current}")]
    StaleRevision { expected: u64, current: u64 },
    #[error("no game is running")]
    NoGame,
    #[error("game transition failed: {0}")]
    Game(#[from] GameError),
    #[error("persistence failed: {0}")]
    Persistence(String),
    #[error("persisted data is invalid: {0}")]
    InvalidPersistedData(String),
    #[error("session transition failed: {0}")]
    Session(#[from] SessionError),
    #[error("invalid game options: {0}")]
    InvalidGameOptions(String),
}

pub struct Runtime<R> {
    instance_id: String,
    snapshot: RuntimeSnapshot,
    repository: R,
}

impl<R: Repository> Runtime<R> {
    /// Creates a runtime and restores the last committed snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Persistence`] when the repository cannot be read
    /// and [`RuntimeError::InvalidPersistedData`] for an invalid snapshot.
    pub fn restore(instance_id: impl Into<String>, repository: R) -> Result<Self, RuntimeError> {
        let snapshot = repository
            .load_snapshot()
            .map_err(RuntimeError::Persistence)?
            .map(|json| {
                serde_json::from_str(&json)
                    .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))
            })
            .transpose()?
            .unwrap_or(RuntimeSnapshot {
                revision: 0,
                game: None,
                session: SessionCore::default(),
            });
        Ok(Self {
            instance_id: instance_id.into(),
            snapshot,
            repository,
        })
    }

    #[must_use]
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    #[must_use]
    pub const fn snapshot(&self) -> &RuntimeSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub const fn repository(&self) -> &R {
        &self.repository
    }

    /// Validates and applies one transport-neutral command envelope.
    ///
    /// # Errors
    ///
    /// Returns a stable contract error for incompatible protocol versions,
    /// stale clients, unsupported commands, invalid options or persistence
    /// failures.
    pub fn dispatch_envelope(
        &mut self,
        envelope: CommandEnvelope,
    ) -> Result<CommandResult, ContractError> {
        if envelope.protocol_version != PROTOCOL_VERSION {
            return Err(contract_error(
                ErrorCode::IncompatibleProtocol,
                "incompatible protocol version",
                Some(format!(
                    "expected {PROTOCOL_VERSION}, got {}",
                    envelope.protocol_version
                )),
            ));
        }
        let action = command_to_action(envelope.command)?;
        self.dispatch(
            &envelope.runtime_instance_id,
            &envelope.command_id,
            envelope.expected_revision,
            action,
        )
        .map_err(|error| runtime_contract_error(&error))
    }

    /// Applies, commits and then publishes one command result.
    ///
    /// # Errors
    ///
    /// Rejects a wrong runtime instance, stale revision, invalid game action,
    /// serialization error or failed repository transaction.
    pub fn dispatch(
        &mut self,
        runtime_instance_id: &str,
        command_id: &str,
        expected_revision: Option<u64>,
        action: RuntimeAction,
    ) -> Result<CommandResult, RuntimeError> {
        if runtime_instance_id != self.instance_id {
            return Err(RuntimeError::WrongRuntimeInstance);
        }
        if let Some(json) = self
            .repository
            .load_command_result(command_id)
            .map_err(RuntimeError::Persistence)?
        {
            let mut result: CommandResult = serde_json::from_str(&json)
                .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
            result.duplicate = true;
            return Ok(result);
        }
        if let Some(expected) = expected_revision
            && expected != self.snapshot.revision
        {
            return Err(RuntimeError::StaleRevision {
                expected,
                current: self.snapshot.revision,
            });
        }

        let action_json = serde_json::to_string(&action)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
        let mut next = self.snapshot.clone();
        apply_action(&mut next, action)?;
        next.revision = self.snapshot.revision + 1;
        let result = CommandResult {
            command_id: command_id.into(),
            revision: next.revision,
            duplicate: false,
            state: next.game.as_ref().map(RuntimeGame::state),
            session: next.session.state().clone(),
        };
        let snapshot_json = serde_json::to_string(&next)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
        let result_json = serde_json::to_string(&result)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;

        match self
            .repository
            .commit(CommitRequest {
                command_id,
                runtime_instance_id: &self.instance_id,
                previous_revision: self.snapshot.revision,
                next_revision: next.revision,
                action_json: &action_json,
                snapshot_json: &snapshot_json,
                result_json: &result_json,
            })
            .map_err(RuntimeError::Persistence)?
        {
            CommitOutcome::Committed => {
                self.snapshot = next;
                Ok(result)
            }
            CommitOutcome::Duplicate(json) => {
                let mut result: CommandResult = serde_json::from_str(&json)
                    .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
                result.duplicate = true;
                Ok(result)
            }
        }
    }

    #[must_use]
    pub fn into_repository(self) -> R {
        self.repository
    }
}

fn command_to_action(command: RuntimeCommand) -> Result<RuntimeAction, ContractError> {
    match command {
        RuntimeCommand::StartSession {
            session_id,
            players,
        } => Ok(RuntimeAction::StartSession {
            session_id,
            players,
        }),
        RuntimeCommand::PrepareGame { game_type, options } => {
            Ok(RuntimeAction::PrepareGame { game_type, options })
        }
        RuntimeCommand::StartPreparedGame { game_id } => {
            Ok(RuntimeAction::StartPreparedGame { game_id })
        }
        RuntimeCommand::MarkGamePlaying => Ok(RuntimeAction::MarkGamePlaying),
        RuntimeCommand::SelectStarter {
            player_id,
            selection,
        } => Ok(RuntimeAction::SelectStarter {
            player_id,
            selection,
        }),
        RuntimeCommand::NextGame => Ok(RuntimeAction::NextGame),
        RuntimeCommand::StartRematch { game_id } => Ok(RuntimeAction::StartRematch { game_id }),
        RuntimeCommand::EndSession => Ok(RuntimeAction::EndSession),
        RuntimeCommand::CloseSession => Ok(RuntimeAction::CloseSession),
        RuntimeCommand::IngestDart { event } => Ok(RuntimeAction::Dart { event }),
        RuntimeCommand::StartGame {
            game_type,
            player_ids,
            options,
        } => {
            let players = player_ids.into_iter().map(|id| (id.clone(), id)).collect();
            match game_type.as_str() {
                "countup" => {
                    let rounds = option_u64(&options, "rounds", 8)?;
                    let rounds = u16::try_from(rounds).map_err(|_| {
                        invalid_command("countup rounds must fit into an unsigned 16-bit integer")
                    })?;
                    Ok(RuntimeAction::StartCountUp { players, rounds })
                }
                "x01" => {
                    let start_score = option_u64(&options, "start_score", 501)?;
                    let start_score = u32::try_from(start_score).map_err(|_| {
                        invalid_command("X01 start_score must fit into an unsigned 32-bit integer")
                    })?;
                    let out_rule = match options
                        .get("out_rule")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("straight")
                    {
                        "straight" => OutRule::Straight,
                        "double" => OutRule::Double,
                        _ => {
                            return Err(invalid_command("X01 out_rule must be straight or double"));
                        }
                    };
                    Ok(RuntimeAction::StartX01 {
                        players,
                        start_score,
                        out_rule,
                    })
                }
                _ => Err(invalid_command("unsupported game type")),
            }
        }
        RuntimeCommand::ContinueTurn | RuntimeCommand::NextPlayer => Ok(RuntimeAction::Continue),
        RuntimeCommand::Undo => Ok(RuntimeAction::Undo),
        RuntimeCommand::AbortGame => Ok(RuntimeAction::AbortGame),
        RuntimeCommand::GameAction { .. } => Err(invalid_command(
            "command is not implemented by this runtime slice",
        )),
    }
}

fn option_u64(options: &serde_json::Value, name: &str, default: u64) -> Result<u64, ContractError> {
    options.get(name).map_or(Ok(default), |value| {
        value
            .as_u64()
            .ok_or_else(|| invalid_command(&format!("{name} must be an unsigned integer")))
    })
}

fn invalid_command(message: &str) -> ContractError {
    contract_error(ErrorCode::InvalidCommand, message, None)
}

fn contract_error(code: ErrorCode, message: &str, details: Option<String>) -> ContractError {
    ContractError {
        code,
        message: message.into(),
        details,
    }
}

fn runtime_contract_error(error: &RuntimeError) -> ContractError {
    let code = match error {
        RuntimeError::WrongRuntimeInstance => ErrorCode::WrongRuntimeInstance,
        RuntimeError::StaleRevision { .. } => ErrorCode::StaleRevision,
        RuntimeError::Persistence(_) => ErrorCode::PersistenceFailed,
        RuntimeError::NoGame
        | RuntimeError::Game(_)
        | RuntimeError::InvalidPersistedData(_)
        | RuntimeError::Session(_)
        | RuntimeError::InvalidGameOptions(_) => ErrorCode::InvalidCommand,
    };
    contract_error(code, &error.to_string(), None)
}

fn apply_action(snapshot: &mut RuntimeSnapshot, action: RuntimeAction) -> Result<(), RuntimeError> {
    match action {
        RuntimeAction::StartSession {
            session_id,
            players,
        } => {
            snapshot.session.start_session(session_id, players)?;
            snapshot.game = None;
        }
        RuntimeAction::PrepareGame { game_type, options } => {
            snapshot.session.prepare_game(game_type, options)?;
        }
        RuntimeAction::StartPreparedGame { game_id } => {
            start_prepared_game(snapshot, game_id, false)?;
        }
        RuntimeAction::MarkGamePlaying => {
            snapshot.session.mark_playing()?;
        }
        RuntimeAction::SelectStarter {
            player_id,
            selection,
        } => {
            snapshot.session.select_starter(&player_id, selection)?;
        }
        RuntimeAction::NextGame => {
            snapshot.session.next_game()?;
            snapshot.game = None;
        }
        RuntimeAction::StartRematch { game_id } => {
            start_prepared_game(snapshot, game_id, true)?;
        }
        RuntimeAction::AbortGame => {
            if snapshot.session.state().session_id.is_some() {
                snapshot.session.abort_game()?;
            }
            snapshot.game = None;
        }
        RuntimeAction::EndSession => {
            snapshot.session.end_session()?;
        }
        RuntimeAction::CloseSession => {
            snapshot.session.close_session();
            snapshot.game = None;
        }
        RuntimeAction::StartCountUp { players, rounds } => {
            snapshot.game = Some(RuntimeGame::CountUp(CountUpGame::new(players, rounds)?));
        }
        RuntimeAction::StartX01 {
            players,
            start_score,
            out_rule,
        } => {
            snapshot.game = Some(RuntimeGame::X01(X01Game::new(
                players,
                start_score,
                out_rule,
            )?));
        }
        RuntimeAction::Dart { event } => {
            snapshot
                .game
                .as_mut()
                .ok_or(RuntimeError::NoGame)?
                .apply_throw(event)?;
            sync_finished_game(snapshot)?;
        }
        RuntimeAction::Continue => {
            snapshot
                .game
                .as_mut()
                .ok_or(RuntimeError::NoGame)?
                .continue_turn()?;
            sync_finished_game(snapshot)?;
        }
        RuntimeAction::Undo => {
            let game = snapshot.game.as_mut().ok_or(RuntimeError::NoGame)?;
            let was_finished = game.is_finished();
            game.undo()?;
            if was_finished
                && !game.is_finished()
                && snapshot.session.state().screen == Screen::GameResult
            {
                snapshot.session.reopen_game()?;
            }
        }
    }
    Ok(())
}

fn start_prepared_game(
    snapshot: &mut RuntimeSnapshot,
    game_id: String,
    rematch: bool,
) -> Result<(), RuntimeError> {
    let prepared = snapshot
        .session
        .state()
        .prepared_game
        .clone()
        .ok_or(SessionError::NoPreparedGame)?;
    let ordered = if rematch {
        snapshot.session.start_rematch(game_id)?
    } else {
        snapshot.session.start_game(game_id)?
    };
    snapshot.game = Some(game_from_options(
        &prepared.game_type,
        ordered,
        &prepared.options,
    )?);
    Ok(())
}

fn game_from_options(
    game_type: &str,
    players: Vec<PlayerRef>,
    options: &serde_json::Value,
) -> Result<RuntimeGame, RuntimeError> {
    let players = players
        .into_iter()
        .map(|player| (player.id, player.name))
        .collect();
    match game_type {
        "countup" => {
            let rounds = options
                .get("rounds")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(8);
            let rounds = u16::try_from(rounds).map_err(|_| {
                RuntimeError::InvalidGameOptions("countup rounds are out of range".into())
            })?;
            Ok(RuntimeGame::CountUp(CountUpGame::new(players, rounds)?))
        }
        "x01" => {
            let start_score = options
                .get("start_score")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(501);
            let start_score = u32::try_from(start_score).map_err(|_| {
                RuntimeError::InvalidGameOptions("X01 start score is out of range".into())
            })?;
            let out_rule = match options
                .get("out_rule")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("straight")
            {
                "straight" => OutRule::Straight,
                "double" => OutRule::Double,
                _ => {
                    return Err(RuntimeError::InvalidGameOptions(
                        "X01 out rule must be straight or double".into(),
                    ));
                }
            };
            Ok(RuntimeGame::X01(X01Game::new(
                players,
                start_score,
                out_rule,
            )?))
        }
        _ => Err(RuntimeError::InvalidGameOptions(format!(
            "unsupported game type: {game_type}"
        ))),
    }
}

fn sync_finished_game(snapshot: &mut RuntimeSnapshot) -> Result<(), RuntimeError> {
    let Some(game) = snapshot.game.as_ref() else {
        return Ok(());
    };
    if game.is_finished()
        && matches!(
            snapshot.session.state().screen,
            Screen::Countdown | Screen::Playing
        )
    {
        snapshot.session.complete_game(game.winner_ids())?;
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct MemoryRepository {
    snapshot: Option<String>,
    results: HashMap<String, String>,
    fail_next_commit: bool,
}

impl MemoryRepository {
    pub fn fail_next_commit(&mut self) {
        self.fail_next_commit = true;
    }
}

impl Repository for MemoryRepository {
    fn load_snapshot(&self) -> Result<Option<String>, String> {
        Ok(self.snapshot.clone())
    }

    fn load_command_result(&self, command_id: &str) -> Result<Option<String>, String> {
        Ok(self.results.get(command_id).cloned())
    }

    fn commit(&mut self, request: CommitRequest<'_>) -> Result<CommitOutcome, String> {
        if self.fail_next_commit {
            self.fail_next_commit = false;
            return Err("injected commit failure".into());
        }
        if let Some(result) = self.results.get(request.command_id) {
            return Ok(CommitOutcome::Duplicate(result.clone()));
        }
        self.snapshot = Some(request.snapshot_json.into());
        self.results
            .insert(request.command_id.into(), request.result_json.into());
        Ok(CommitOutcome::Committed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdb_contracts::Ring;

    fn players() -> Vec<(String, String)> {
        vec![("ada".into(), "Ada".into())]
    }

    fn session_players() -> Vec<PlayerRef> {
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
    fn failed_commit_does_not_publish_state() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime.repository.fail_next_commit();
        let error = runtime
            .dispatch(
                "runtime",
                "start-1",
                Some(0),
                RuntimeAction::StartCountUp {
                    players: players(),
                    rounds: 5,
                },
            )
            .expect_err("commit must fail");
        assert!(matches!(error, RuntimeError::Persistence(_)));
        assert_eq!(runtime.snapshot.revision, 0);
        assert!(runtime.snapshot.game.is_none());
    }

    #[test]
    fn duplicate_command_returns_original_revision() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let first = runtime
            .dispatch(
                "runtime",
                "start-1",
                Some(0),
                RuntimeAction::StartCountUp {
                    players: players(),
                    rounds: 5,
                },
            )
            .expect("first command");
        let duplicate = runtime
            .dispatch("runtime", "start-1", None, RuntimeAction::Undo)
            .expect("duplicate command");
        assert_eq!(first.revision, 1);
        assert_eq!(duplicate.revision, 1);
        assert!(duplicate.duplicate);
        assert_eq!(runtime.snapshot.revision, 1);
    }

    #[test]
    fn x01_uses_the_same_atomic_runtime_boundary() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "start-x01",
                Some(0),
                RuntimeAction::StartX01 {
                    players: players(),
                    start_score: 40,
                    out_rule: OutRule::Double,
                },
            )
            .expect("start X01");
        let result = runtime
            .dispatch(
                "runtime",
                "checkout",
                Some(1),
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Double,
                        multiplier: 2,
                        label: "D20".into(),
                        score: 40,
                    },
                },
            )
            .expect("checkout");

        let RuntimeGameState::X01(state) = result.state.expect("state") else {
            panic!("wrong game type");
        };
        assert_eq!(result.revision, 2);
        assert_eq!(state.winner_id.as_deref(), Some("ada"));
        assert_eq!(state.players[0].score, 0);
    }

    #[test]
    fn command_envelope_validates_protocol_and_dispatches_x01() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let incompatible = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION + 1,
                command_id: "bad-version".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::Undo,
            })
            .expect_err("protocol must be rejected");
        assert_eq!(incompatible.code, ErrorCode::IncompatibleProtocol);

        let started = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "start".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::StartGame {
                    game_type: "x01".into(),
                    player_ids: vec!["ada".into()],
                    options: serde_json::json!({
                        "start_score": 301,
                        "out_rule": "double"
                    }),
                },
            })
            .expect("start X01");
        assert_eq!(started.revision, 1);
        let Some(RuntimeGameState::X01(state)) = started.state else {
            panic!("wrong game type");
        };
        assert_eq!(state.start_score, 301);
        assert_eq!(state.out_rule, OutRule::Double);
    }

    #[test]
    fn command_envelope_returns_stable_revision_and_instance_errors() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let wrong_instance = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "wrong-instance".into(),
                runtime_instance_id: "other".into(),
                expected_revision: Some(0),
                command: RuntimeCommand::Undo,
            })
            .expect_err("instance must be rejected");
        assert_eq!(wrong_instance.code, ErrorCode::WrongRuntimeInstance);

        let stale = runtime
            .dispatch_envelope(CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                command_id: "stale".into(),
                runtime_instance_id: "runtime".into(),
                expected_revision: Some(7),
                command: RuntimeCommand::StartGame {
                    game_type: "countup".into(),
                    player_ids: vec!["ada".into()],
                    options: serde_json::json!({"rounds": 8}),
                },
            })
            .expect_err("revision must be rejected");
        assert_eq!(stale.code, ErrorCode::StaleRevision);
    }

    #[test]
    fn session_result_and_game_state_commit_or_rollback_together() {
        let repository = MemoryRepository::default();
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "session",
                None,
                RuntimeAction::StartSession {
                    session_id: "session-1".into(),
                    players: session_players(),
                },
            )
            .expect("session");
        runtime
            .dispatch(
                "runtime",
                "prepare",
                None,
                RuntimeAction::PrepareGame {
                    game_type: "x01".into(),
                    options: serde_json::json!({
                        "start_score": 40,
                        "out_rule": "double"
                    }),
                },
            )
            .expect("prepare");
        runtime
            .dispatch(
                "runtime",
                "start",
                None,
                RuntimeAction::StartPreparedGame {
                    game_id: "game-1".into(),
                },
            )
            .expect("start");
        runtime
            .dispatch("runtime", "playing", None, RuntimeAction::MarkGamePlaying)
            .expect("playing");
        let checkout = DartEvent::Hit {
            seq: 1,
            field: 20,
            ring: Ring::Double,
            multiplier: 2,
            label: "D20".into(),
            score: 40,
        };
        runtime
            .dispatch(
                "runtime",
                "checkout",
                None,
                RuntimeAction::Dart {
                    event: checkout.clone(),
                },
            )
            .expect("checkout");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::GameResult);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            3
        );

        runtime
            .dispatch("runtime", "undo", None, RuntimeAction::Undo)
            .expect("undo checkout");
        assert_eq!(runtime.snapshot.session.state().screen, Screen::Playing);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            0
        );

        runtime.repository.fail_next_commit();
        let error = runtime
            .dispatch(
                "runtime",
                "failed-checkout",
                None,
                RuntimeAction::Dart { event: checkout },
            )
            .expect_err("commit must fail");
        assert!(matches!(error, RuntimeError::Persistence(_)));
        assert_eq!(runtime.snapshot.session.state().screen, Screen::Playing);
        assert_eq!(
            runtime.snapshot.session.state().standings[0].session_points,
            0
        );
        let Some(RuntimeGame::X01(game)) = &runtime.snapshot.game else {
            panic!("wrong game type");
        };
        assert_eq!(game.state().players[0].score, 40);
    }
}
