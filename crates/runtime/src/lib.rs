//! Serialized authoritative runtime.
//!
//! A transition is calculated on a clone, committed through [`Repository`],
//! and only then installed as visible in-memory state. This preserves the
//! product's state when persistence fails between a dart and its broadcast.

use sdb_contracts::DartEvent;
use sdb_game_core::{CountUpGame, CountUpState, GameError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeAction {
    StartCountUp {
        players: Vec<(String, String)>,
        rounds: u16,
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
    pub game: Option<CountUpGame>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub command_id: String,
    pub revision: u64,
    pub duplicate: bool,
    pub state: Option<CountUpState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRequest<'a> {
    pub command_id: &'a str,
    pub previous_revision: u64,
    pub next_revision: u64,
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

        let mut next = self.snapshot.clone();
        apply_action(&mut next, action)?;
        next.revision = self.snapshot.revision + 1;
        let result = CommandResult {
            command_id: command_id.into(),
            revision: next.revision,
            duplicate: false,
            state: next.game.as_ref().map(|game| game.state().clone()),
        };
        let snapshot_json = serde_json::to_string(&next)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;
        let result_json = serde_json::to_string(&result)
            .map_err(|error| RuntimeError::InvalidPersistedData(error.to_string()))?;

        match self
            .repository
            .commit(CommitRequest {
                command_id,
                previous_revision: self.snapshot.revision,
                next_revision: next.revision,
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

fn apply_action(snapshot: &mut RuntimeSnapshot, action: RuntimeAction) -> Result<(), RuntimeError> {
    match action {
        RuntimeAction::StartCountUp { players, rounds } => {
            snapshot.game = Some(CountUpGame::new(players, rounds)?);
        }
        RuntimeAction::Dart { event } => {
            snapshot
                .game
                .as_mut()
                .ok_or(RuntimeError::NoGame)?
                .apply_throw(&event)?;
        }
        RuntimeAction::Continue => {
            snapshot
                .game
                .as_mut()
                .ok_or(RuntimeError::NoGame)?
                .continue_turn()?;
        }
        RuntimeAction::Undo => {
            snapshot.game.as_mut().ok_or(RuntimeError::NoGame)?.undo()?;
        }
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

    fn players() -> Vec<(String, String)> {
        vec![("ada".into(), "Ada".into())]
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
}
