//! Transactional `SQLite` implementation of the runtime repository.

use rusqlite::{Connection, OptionalExtension, params};
use sdb_runtime::{CommitOutcome, CommitRequest, Repository};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database schema {found} is newer than supported schema {supported}")]
    UnsupportedSchema { found: u32, supported: u32 },
    #[error("database integrity check failed: {0}")]
    Integrity(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeJournalEntry {
    pub revision: u64,
    pub command_id: String,
    pub runtime_instance_id: String,
    pub action_json: String,
    pub snapshot_json: String,
    pub committed_at: String,
}

#[derive(Debug)]
pub struct SqliteRepository {
    connection: Connection,
}

impl SqliteRepository {
    /// Opens the runtime database and applies the current schema.
    ///
    /// # Errors
    ///
    /// Returns an error when the database cannot be opened, migrated or fails
    /// its post-migration integrity check. Newer schemas are never downgraded.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&connection)?;
        verify_integrity(&connection)?;
        Ok(Self { connection })
    }

    /// Opens an isolated in-memory repository.
    ///
    /// # Errors
    ///
    /// Returns an error when schema initialization or verification fails.
    pub fn in_memory() -> Result<Self, StorageError> {
        Self::open(":memory:")
    }

    /// Returns the installed schema version.
    ///
    /// # Errors
    ///
    /// Returns the underlying `SQLite` error when the pragma cannot be read.
    pub fn schema_version(&self) -> Result<u32, StorageError> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    /// Returns committed journal entries in ascending revision order.
    ///
    /// # Errors
    ///
    /// Returns an error when the journal cannot be queried or a stored
    /// revision is outside the unsigned range.
    pub fn journal(&self, limit: usize) -> Result<Vec<RuntimeJournalEntry>, StorageError> {
        let limit = i64::try_from(limit.clamp(1, 10_000))
            .map_err(|_| StorageError::Integrity("journal limit is out of range".into()))?;
        let mut statement = self.connection.prepare(
            "SELECT revision, command_id, runtime_instance_id, action_json,
                    snapshot_json, committed_at
             FROM runtime_journal ORDER BY revision ASC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (
                revision,
                command_id,
                runtime_instance_id,
                action_json,
                snapshot_json,
                committed_at,
            ) = row?;
            let revision = u64::try_from(revision).map_err(|_| {
                StorageError::Integrity("journal contains a negative revision".into())
            })?;
            Ok(RuntimeJournalEntry {
                revision,
                command_id,
                runtime_instance_id,
                action_json,
                snapshot_json,
                committed_at,
            })
        })
        .collect()
    }
}

fn migrate(connection: &Connection) -> Result<(), StorageError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSchema {
            found: version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    if version < 1 {
        connection.execute_batch(
            "
            BEGIN;
            CREATE TABLE IF NOT EXISTS runtime_meta (
                singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                revision INTEGER NOT NULL,
                snapshot_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS processed_commands (
                command_id TEXT PRIMARY KEY,
                committed_revision INTEGER NOT NULL,
                result_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS effect_outbox (
                effect_id TEXT PRIMARY KEY,
                committed_revision INTEGER NOT NULL,
                effect_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending'
            );
            PRAGMA user_version=1;
            COMMIT;
            ",
        )?;
    }
    if version < 2 {
        connection.execute_batch(
            "
            BEGIN;
            CREATE TABLE runtime_journal (
                revision INTEGER PRIMARY KEY,
                command_id TEXT NOT NULL UNIQUE,
                runtime_instance_id TEXT NOT NULL,
                action_json TEXT NOT NULL,
                snapshot_json TEXT NOT NULL,
                committed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            PRAGMA user_version=2;
            COMMIT;
            ",
        )?;
    }
    Ok(())
}

fn verify_integrity(connection: &Connection) -> Result<(), StorageError> {
    let result: String = connection.pragma_query_value(None, "quick_check", |row| row.get(0))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(StorageError::Integrity(result))
    }
}

impl Repository for SqliteRepository {
    fn load_snapshot(&self) -> Result<Option<String>, String> {
        self.connection
            .query_row(
                "SELECT snapshot_json FROM runtime_meta WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    fn load_command_result(&self, command_id: &str) -> Result<Option<String>, String> {
        self.connection
            .query_row(
                "SELECT result_json FROM processed_commands WHERE command_id=?1",
                [command_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    fn commit(&mut self, request: CommitRequest<'_>) -> Result<CommitOutcome, String> {
        let next_revision = i64::try_from(request.next_revision)
            .map_err(|_| "revision exceeds SQLite integer range".to_string())?;
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| error.to_string())?;
        if let Some(result) = transaction
            .query_row(
                "SELECT result_json FROM processed_commands WHERE command_id=?1",
                [request.command_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
        {
            return Ok(CommitOutcome::Duplicate(result));
        }
        let current_revision = transaction
            .query_row(
                "SELECT revision FROM runtime_meta WHERE singleton=1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .unwrap_or(0);
        let current_revision = u64::try_from(current_revision)
            .map_err(|_| "database contains a negative revision".to_string())?;
        if current_revision != request.previous_revision {
            return Err(format!(
                "concurrent revision change: expected {}, found {current_revision}",
                request.previous_revision
            ));
        }
        transaction
            .execute(
                "INSERT INTO processed_commands(command_id, committed_revision, result_json)
                 VALUES(?1, ?2, ?3)",
                params![request.command_id, next_revision, request.result_json],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO runtime_meta(singleton, revision, snapshot_json)
                 VALUES(1, ?1, ?2)
                 ON CONFLICT(singleton) DO UPDATE SET
                    revision=excluded.revision,
                    snapshot_json=excluded.snapshot_json",
                params![next_revision, request.snapshot_json],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO runtime_journal(
                    revision, command_id, runtime_instance_id, action_json, snapshot_json
                 ) VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    next_revision,
                    request.command_id,
                    request.runtime_instance_id,
                    request.action_json,
                    request.snapshot_json
                ],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(CommitOutcome::Committed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdb_contracts::PlayerRef;
    use sdb_runtime::{Runtime, RuntimeAction};
    use sdb_session_core::Screen;

    #[test]
    fn sqlite_restores_committed_runtime_and_deduplicates() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-runtime-{}-{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&temporary);
        {
            let repository = SqliteRepository::open(&temporary).expect("open");
            let mut runtime = Runtime::restore("first", repository).expect("runtime");
            runtime
                .dispatch(
                    "first",
                    "start",
                    Some(0),
                    RuntimeAction::StartCountUp {
                        players: vec![("ada".into(), "Ada".into())],
                        rounds: 5,
                    },
                )
                .expect("commit");
        }
        let repository = SqliteRepository::open(&temporary).expect("reopen");
        let mut runtime = Runtime::restore("second", repository).expect("restore");
        assert_eq!(runtime.snapshot().revision, 1);
        let game = runtime.snapshot().game.as_ref().expect("game");
        let sdb_runtime::RuntimeGame::CountUp(game) = game else {
            panic!("restored wrong game type");
        };
        assert_eq!(game.state().players[0].name, "Ada");
        let duplicate = runtime
            .dispatch("second", "start", None, RuntimeAction::Undo)
            .expect("deduplicated");
        assert!(duplicate.duplicate);
        assert_eq!(duplicate.revision, 1);
        let repository = runtime.into_repository();
        let journal = repository.journal(10).expect("journal");
        assert_eq!(journal.len(), 1);
        assert_eq!(journal[0].revision, 1);
        assert_eq!(journal[0].command_id, "start");
        assert_eq!(journal[0].runtime_instance_id, "first");
        let action: serde_json::Value =
            serde_json::from_str(&journal[0].action_json).expect("action JSON");
        assert_eq!(action["type"], "start_count_up");
        std::fs::remove_file(temporary).expect("remove test database");
    }

    #[test]
    fn sqlite_restores_session_and_game_from_the_same_snapshot() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-session-runtime-{}-{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&temporary);
        {
            let repository = SqliteRepository::open(&temporary).expect("open");
            let mut runtime = Runtime::restore("first", repository).expect("runtime");
            runtime
                .dispatch(
                    "first",
                    "session",
                    None,
                    RuntimeAction::StartSession {
                        session_id: "session-1".into(),
                        players: vec![PlayerRef {
                            id: "ada".into(),
                            name: "Ada".into(),
                            avatar: "nova".into(),
                            color: "#ff00aa".into(),
                        }],
                    },
                )
                .expect("session");
            runtime
                .dispatch(
                    "first",
                    "prepare",
                    None,
                    RuntimeAction::PrepareGame {
                        game_type: "x01".into(),
                        options: serde_json::json!({
                            "start_score": 301,
                            "out_rule": "double"
                        }),
                    },
                )
                .expect("prepare");
            runtime
                .dispatch(
                    "first",
                    "start",
                    None,
                    RuntimeAction::StartPreparedGame {
                        game_id: "game-1".into(),
                    },
                )
                .expect("start");
        }

        let repository = SqliteRepository::open(&temporary).expect("reopen");
        let runtime = Runtime::restore("second", repository).expect("restore");
        assert_eq!(runtime.snapshot().revision, 3);
        assert_eq!(runtime.snapshot().session.state().screen, Screen::Countdown);
        assert_eq!(
            runtime.snapshot().session.state().game_id.as_deref(),
            Some("game-1")
        );
        assert!(matches!(
            runtime.snapshot().game,
            Some(sdb_runtime::RuntimeGame::X01(_))
        ));
        std::fs::remove_file(temporary).expect("remove test database");
    }

    #[test]
    fn version_one_database_migrates_forward_without_losing_runtime_tables() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-migration-{}-{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&temporary);
        {
            let connection = Connection::open(&temporary).expect("legacy database");
            connection
                .execute_batch(
                    "
                    CREATE TABLE runtime_meta (
                        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                        revision INTEGER NOT NULL,
                        snapshot_json TEXT NOT NULL
                    );
                    CREATE TABLE processed_commands (
                        command_id TEXT PRIMARY KEY,
                        committed_revision INTEGER NOT NULL,
                        result_json TEXT NOT NULL
                    );
                    CREATE TABLE effect_outbox (
                        effect_id TEXT PRIMARY KEY,
                        committed_revision INTEGER NOT NULL,
                        effect_json TEXT NOT NULL,
                        status TEXT NOT NULL DEFAULT 'pending'
                    );
                    PRAGMA user_version=1;
                    ",
                )
                .expect("legacy schema");
        }
        let repository = SqliteRepository::open(&temporary).expect("migrate");
        assert_eq!(repository.schema_version().expect("version"), 2);
        assert!(repository.journal(10).expect("journal").is_empty());
        std::fs::remove_file(temporary).expect("remove test database");
    }

    #[test]
    fn newer_database_schema_is_rejected_without_downgrade() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-future-schema-{}-{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&temporary);
        {
            let connection = Connection::open(&temporary).expect("future database");
            connection
                .pragma_update(None, "user_version", 99)
                .expect("future version");
        }
        let error = SqliteRepository::open(&temporary).expect_err("must reject downgrade");
        assert!(matches!(
            error,
            StorageError::UnsupportedSchema {
                found: 99,
                supported: CURRENT_SCHEMA_VERSION
            }
        ));
        let connection = Connection::open(&temporary).expect("inspect future database");
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, 99);
        drop(connection);
        std::fs::remove_file(temporary).expect("remove test database");
    }

    #[test]
    fn failed_journal_insert_rolls_back_snapshot_and_deduplication() {
        let repository = SqliteRepository::in_memory().expect("repository");
        repository
            .connection
            .execute_batch(
                "CREATE TRIGGER fail_runtime_journal
                 BEFORE INSERT ON runtime_journal
                 BEGIN
                   SELECT RAISE(ABORT, 'injected journal failure');
                 END;",
            )
            .expect("failure trigger");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let error = runtime
            .dispatch(
                "runtime",
                "start",
                Some(0),
                RuntimeAction::StartCountUp {
                    players: vec![("ada".into(), "Ada".into())],
                    rounds: 5,
                },
            )
            .expect_err("transaction must fail");
        assert!(matches!(error, sdb_runtime::RuntimeError::Persistence(_)));
        assert_eq!(runtime.snapshot().revision, 0);
        let repository = runtime.into_repository();
        assert!(
            repository
                .load_snapshot()
                .expect("snapshot query")
                .is_none()
        );
        assert!(
            repository
                .load_command_result("start")
                .expect("command query")
                .is_none()
        );
        assert!(repository.journal(10).expect("journal query").is_empty());
    }
}
