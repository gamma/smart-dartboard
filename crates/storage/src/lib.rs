//! Transactional `SQLite` implementation of the runtime repository.

use rusqlite::{Connection, OptionalExtension, params};
use sdb_runtime::{CommitOutcome, CommitRequest, Repository};
use std::path::Path;

pub struct SqliteRepository {
    connection: Connection,
}

impl SqliteRepository {
    /// Opens the runtime database and applies the current schema.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the database cannot be opened or migrated.
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
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
        Ok(Self { connection })
    }

    /// Opens an isolated in-memory repository.
    ///
    /// # Errors
    ///
    /// Returns the `SQLite` error when the connection or schema fails.
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::open(":memory:")
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
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(CommitOutcome::Committed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdb_runtime::{Runtime, RuntimeAction};

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
        assert_eq!(
            runtime
                .snapshot()
                .game
                .as_ref()
                .expect("game")
                .state()
                .players[0]
                .name,
            "Ada"
        );
        let duplicate = runtime
            .dispatch("second", "start", None, RuntimeAction::Undo)
            .expect("deduplicated");
        assert!(duplicate.duplicate);
        assert_eq!(duplicate.revision, 1);
        std::fs::remove_file(temporary).expect("remove test database");
    }
}
