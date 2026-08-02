//! Transactional `SQLite` implementation of the runtime repository.

use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, backup::Backup, params};
use sdb_companion::{CompanionRole, PairedDevice};
use sdb_contracts::{
    ArtTheme, CalibrationSettings, DartEvent, DartSource, EffectDelivery, PlatformEffect,
    ProjectorGeometry, RuntimeSettings, SoundOutput, SoundStatus, UiLanguage,
};
use sdb_game_core::GameStatus;
use sdb_runtime::{
    CommitOutcome, CommitRequest, Repository, RuntimeAction, RuntimeGame, RuntimeSnapshot,
};
use sdb_session_core::{Screen, SessionCore, SessionStatus};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

pub const CURRENT_SCHEMA_VERSION: u32 = 6;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database schema {found} is newer than supported schema {supported}")]
    UnsupportedSchema { found: u32, supported: u32 },
    #[error("database integrity check failed: {0}")]
    Integrity(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyDatabaseImport {
    pub source: PathBuf,
    pub backup: PathBuf,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionHistory {
    pub id: String,
    pub status: String,
    pub language: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub games: u64,
    pub finished_games: u64,
    pub player_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerStatistics {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub games: u64,
    pub wins: u64,
    pub darts: u64,
    pub total_points: u64,
    pub total_mode_points: i64,
    pub best_dart: u64,
    pub misses: u64,
    pub three_dart_average: f64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeStatistics {
    pub game_type: String,
    pub ruleset_version: u64,
    pub options: serde_json::Value,
    pub starts: u64,
    pub finished: u64,
    pub aborted: u64,
    pub interrupted: u64,
    pub average_seconds: Option<f64>,
    pub darts: u64,
    pub successes: u64,
    pub partials: u64,
    pub dangers: u64,
    pub misses: u64,
    pub success_rate: f64,
    pub completion_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeatmapSegment {
    pub field: u64,
    pub ring: String,
    pub darts: u64,
    pub successes: u64,
    pub dangers: u64,
    pub neutral: u64,
    pub dart_points: i64,
    pub mode_points: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeatmapStatistics {
    pub resolution: &'static str,
    pub segments: Vec<HeatmapSegment>,
    pub total_darts: u64,
    pub board_hits: u64,
    pub misses: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingRecommendation {
    pub field: u64,
    pub ring: String,
    pub attempts: u64,
    pub successes: u64,
    pub success_rate: f64,
    #[serde(skip_serializing_if = "is_false")]
    pub starter: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingRecommendations {
    pub player_id: String,
    pub recommendations: Vec<TrainingRecommendation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GamePlayerHistory {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub position: u64,
    pub final_score: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameHistory {
    pub id: String,
    pub session_id: String,
    pub game_type: String,
    pub status: String,
    pub options: serde_json::Value,
    pub winner_ids: Vec<String>,
    pub result_type: String,
    pub finish_reason: String,
    pub ruleset_version: u64,
    pub app_version: String,
    pub environment: String,
    pub darts: u64,
    pub winner_count: u64,
    pub initial_state: Option<serde_json::Value>,
    pub final_state: Option<serde_json::Value>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub players: Vec<GamePlayerHistory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDetail {
    pub session: SessionHistory,
    pub players: Vec<PlayerProfile>,
    pub games: Vec<GameHistory>,
    pub statistics: Vec<PlayerStatistics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThrowHistory {
    pub id: u64,
    pub action_id: Option<u64>,
    pub seq: u64,
    pub player_id: Option<String>,
    pub event: serde_json::Value,
    pub score_after: i64,
    pub round_number: u64,
    pub dart_in_turn: u64,
    pub field: Option<u64>,
    pub ring: Option<String>,
    pub multiplier: Option<u64>,
    pub dart_score: i64,
    pub mode_points: i64,
    pub outcome: String,
    pub source: String,
    pub task: Option<serde_json::Value>,
    pub event_id: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameEventHistory {
    pub id: u64,
    pub ordinal: u64,
    pub event_type: String,
    pub player_id: Option<String>,
    pub source: String,
    pub payload: serde_json::Value,
    pub task: Option<serde_json::Value>,
    pub frame: Option<serde_json::Value>,
    pub effective: bool,
    pub corrects_event_id: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDetail {
    pub game: GameHistory,
    pub throws: Vec<ThrowHistory>,
    pub events: Vec<GameEventHistory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameReplay {
    pub game_id: String,
    pub initial_state: Option<serde_json::Value>,
    pub final_state: Option<serde_json::Value>,
    pub events: Vec<GameEventHistory>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportSummary {
    pub schema_version: u32,
    pub players_added: u64,
    pub players_reused: u64,
    pub sessions_added: u64,
    pub games_added: u64,
    pub throws_added: u64,
    pub events_added: u64,
    pub interrupted_sessions: u64,
    pub interrupted_games: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableArchive {
    schema_version: u32,
    database_schema_version: u32,
    exported_at: String,
    players: Vec<PlayerProfile>,
    sessions: Vec<Option<SessionDetail>>,
    games: Vec<PortableGame>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableGame {
    detail: Option<GameDetail>,
    replay: Option<GameReplay>,
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

    /// Creates the new runtime database from a legacy Python database when no
    /// runtime database exists yet. The source remains untouched and an
    /// preserved pre-migration snapshot is retained beside it.
    ///
    /// # Errors
    ///
    /// Returns an error when the legacy database is incompatible, copying or
    /// migration fails, or the resulting database does not pass integrity
    /// validation.
    pub fn open_with_legacy_import(
        runtime_path: impl AsRef<Path>,
        legacy_path: impl AsRef<Path>,
    ) -> Result<(Self, Option<LegacyDatabaseImport>), StorageError> {
        let runtime_path = runtime_path.as_ref();
        let legacy_path = legacy_path.as_ref();
        if runtime_path.exists() || !legacy_path.exists() {
            return Self::open(runtime_path).map(|repository| (repository, None));
        }

        validate_legacy_database(legacy_path)?;
        remove_sidecar_files(runtime_path)?;
        let backup_path = legacy_backup_path(legacy_path)?;
        let backup_staging = staging_path(&backup_path)?;
        remove_staging_file(&backup_staging)?;
        online_backup(legacy_path, &backup_staging)?;
        validate_legacy_database(&backup_staging)?;
        fs::rename(&backup_staging, &backup_path)?;

        let runtime_staging = staging_path(runtime_path)?;
        remove_staging_file(&runtime_staging)?;
        online_backup(&backup_path, &runtime_staging)?;
        {
            let mut repository = Self::open(&runtime_staging)?;
            repository.finalize_legacy_import()?;
            verify_integrity(&repository.connection)?;
        }
        fs::rename(&runtime_staging, runtime_path)?;
        let repository = Self::open(runtime_path)?;
        Ok((
            repository,
            Some(LegacyDatabaseImport {
                source: legacy_path.to_path_buf(),
                backup: backup_path,
            }),
        ))
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

    fn finalize_legacy_import(&mut self) -> Result<(), StorageError> {
        let settings = legacy_runtime_settings(&self.connection)?;
        let snapshot = RuntimeSnapshot {
            revision: 0,
            game: None,
            session: SessionCore::default(),
            settings,
        };
        let snapshot_json = serde_json::to_string(&snapshot).map_err(|error| {
            StorageError::Integrity(format!(
                "cannot serialize imported runtime settings: {error}"
            ))
        })?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO runtime_meta(singleton, revision, snapshot_json)
             VALUES(1, 0, ?1)
             ON CONFLICT(singleton) DO NOTHING",
            [snapshot_json],
        )?;
        transaction.execute(
            "UPDATE games SET status='interrupted', winner_id=NULL, result_type='',
                    finish_reason='legacy_runtime_migration',
                    ended_at=COALESCE(ended_at, CURRENT_TIMESTAMP)
             WHERE status='running'",
            [],
        )?;
        transaction.execute(
            "UPDATE sessions SET status='interrupted',
                    ended_at=COALESCE(ended_at, CURRENT_TIMESTAMP)
             WHERE status='active'",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Returns one small host preference, if present.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid key or an underlying `SQLite` failure.
    pub fn preference(&self, key: &str) -> Result<Option<String>, StorageError> {
        validate_preference_key(key)?;
        Ok(self
            .connection
            .query_row(
                "SELECT value FROM app_preferences WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Persists one small host preference independently of runtime revisions.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid key, a value larger than 4 KiB or an
    /// underlying `SQLite` failure.
    pub fn save_preference(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        validate_preference_key(key)?;
        if value.len() > 4_096 {
            return Err(StorageError::Integrity(
                "app preference exceeds 4096 bytes".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO app_preferences(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Returns active projector companions in deterministic device order.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid stored role, negative timestamp or an
    /// underlying `SQLite` failure.
    pub fn companion_devices(&self) -> Result<Vec<PairedDevice>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT device_id, device_name, role, token_hash, paired_at_ms
             FROM companion_devices
             WHERE revoked_at_ms IS NULL
             ORDER BY device_id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (device_id, device_name, role, token_hash, paired_at_ms) = row?;
            if role != "projector" {
                return Err(StorageError::Integrity(format!(
                    "unknown companion role: {role}"
                )));
            }
            Ok(PairedDevice {
                device_id,
                device_name,
                role: CompanionRole::Projector,
                token_hash,
                paired_at_ms: u64::try_from(paired_at_ms).map_err(|_| {
                    StorageError::Integrity("negative companion pairing timestamp".into())
                })?,
            })
        })
        .collect()
    }

    /// Persists or replaces a projector grant without ever receiving its
    /// plaintext token.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed metadata, timestamps outside `SQLite`'s
    /// signed range, or an underlying `SQLite` failure.
    pub fn save_companion_device(&mut self, device: &PairedDevice) -> Result<(), StorageError> {
        if device.device_id.is_empty()
            || device.device_id.len() > 128
            || device.device_name.trim().is_empty()
            || device.device_name.len() > 80
            || device.token_hash.len() != 64
            || !device
                .token_hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(StorageError::Integrity(
                "invalid companion device metadata".into(),
            ));
        }
        let paired_at_ms = i64::try_from(device.paired_at_ms)
            .map_err(|_| StorageError::Integrity("companion timestamp is out of range".into()))?;
        self.connection.execute(
            "INSERT INTO companion_devices(
                 device_id, device_name, role, token_hash, paired_at_ms, revoked_at_ms
             ) VALUES(?1, ?2, 'projector', ?3, ?4, NULL)
             ON CONFLICT(device_id) DO UPDATE SET
                 device_name=excluded.device_name,
                 role=excluded.role,
                 token_hash=excluded.token_hash,
                 paired_at_ms=excluded.paired_at_ms,
                 revoked_at_ms=NULL",
            params![
                device.device_id,
                device.device_name,
                device.token_hash,
                paired_at_ms
            ],
        )?;
        Ok(())
    }

    /// Revokes a companion grant while retaining a local audit timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error for timestamps outside `SQLite`'s signed range or an
    /// underlying `SQLite` failure.
    pub fn revoke_companion_device(
        &mut self,
        device_id: &str,
        revoked_at_ms: u64,
    ) -> Result<bool, StorageError> {
        let revoked_at_ms = i64::try_from(revoked_at_ms)
            .map_err(|_| StorageError::Integrity("companion timestamp is out of range".into()))?;
        Ok(self.connection.execute(
            "UPDATE companion_devices SET revoked_at_ms=?1
             WHERE device_id=?2 AND revoked_at_ms IS NULL",
            params![revoked_at_ms, device_id],
        )? > 0)
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

    /// Lists durable player profiles in deterministic display order.
    ///
    /// # Errors
    ///
    /// Returns an error when the profile projection cannot be queried.
    pub fn players(&self) -> Result<Vec<PlayerProfile>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, avatar, color, created_at
             FROM players ORDER BY lower(name), created_at, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(PlayerProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                avatar: row.get(2)?,
                color: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    /// Lists recent sessions including aggregate counts.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid stored count or failed query.
    pub fn sessions(&self, limit: usize) -> Result<Vec<SessionHistory>, StorageError> {
        let limit = i64::try_from(limit.clamp(1, 500))
            .map_err(|_| StorageError::Integrity("session limit is out of range".into()))?;
        let mut statement = self.connection.prepare(
            "SELECT s.id, s.status, s.language, s.started_at, s.ended_at,
                    COUNT(DISTINCT g.id),
                    COUNT(DISTINCT CASE WHEN g.status='finished' THEN g.id END),
                    COUNT(DISTINCT sp.player_id)
             FROM sessions s
             LEFT JOIN session_players sp ON sp.session_id=s.id
             LEFT JOIN games g ON g.session_id=s.id
             GROUP BY s.id
             ORDER BY s.started_at DESC, s.id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })?;
        rows.map(|row| {
            let (id, status, language, started_at, ended_at, games, finished, players) = row?;
            Ok(SessionHistory {
                id,
                status,
                language,
                started_at,
                ended_at,
                games: nonnegative(games, "session game count")?,
                finished_games: nonnegative(finished, "finished game count")?,
                player_count: nonnegative(players, "session player count")?,
            })
        })
        .collect()
    }

    /// Computes lifetime player statistics from finished production games.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid projected values or a failed query.
    pub fn player_statistics(&self) -> Result<Vec<PlayerStatistics>, StorageError> {
        self.statistics(None, false)
    }

    /// Computes lifetime player statistics and optionally includes test games.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid projected values or a failed query.
    pub fn player_statistics_including_test(
        &self,
        include_test: bool,
    ) -> Result<Vec<PlayerStatistics>, StorageError> {
        self.statistics(None, include_test)
    }

    /// Computes the finished-game leaderboard for one session.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid projected values or a failed query.
    pub fn session_statistics(
        &self,
        session_id: &str,
    ) -> Result<Vec<PlayerStatistics>, StorageError> {
        self.statistics(Some(session_id), true)
    }

    fn statistics(
        &self,
        session_id: Option<&str>,
        include_test: bool,
    ) -> Result<Vec<PlayerStatistics>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT p.id, p.name, p.avatar, p.color,
                    COUNT(DISTINCT g.id) AS games,
                    COUNT(DISTINCT CASE WHEN gw.player_id IS NOT NULL THEN g.id END) AS wins,
                    COUNT(t.id) AS darts, COALESCE(SUM(t.dart_score), 0) AS total_points,
                    COALESCE(SUM(t.mode_points), 0) AS total_mode_points,
                    COALESCE(MAX(t.dart_score), 0) AS best_dart,
                    COALESCE(SUM(CASE WHEN t.outcome='miss' THEN 1 ELSE 0 END), 0) AS misses
             FROM players p
             LEFT JOIN game_players gp ON gp.player_id=p.id
             LEFT JOIN games g ON g.id=gp.game_id
                 AND g.status='finished'
                 AND (?1 IS NULL OR g.session_id=?1)
                 AND (?2=1 OR g.environment='production')
             LEFT JOIN throws t ON t.game_id=g.id AND t.player_id=p.id
             LEFT JOIN game_winners gw ON gw.game_id=g.id AND gw.player_id=p.id
             WHERE ?1 IS NULL OR EXISTS (
                 SELECT 1 FROM session_players sp
                 WHERE sp.session_id=?1 AND sp.player_id=p.id
             )
             GROUP BY p.id
             ORDER BY wins DESC, total_points DESC, lower(p.name), p.id",
        )?;
        let rows = statement.query_map(params![session_id, i64::from(include_test)], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
            ))
        })?;
        rows.map(|row| {
            let (id, name, avatar, color, games, wins, darts, points, mode_points, best, misses) =
                row?;
            let games = nonnegative(games, "statistics games")?;
            let wins = nonnegative(wins, "statistics wins")?;
            let darts = nonnegative(darts, "statistics darts")?;
            let total_points = nonnegative(points, "statistics points")?;
            let games_f64 = f64::from(u32::try_from(games).map_err(|_| {
                StorageError::Integrity("statistics game count exceeds supported range".into())
            })?);
            let wins_f64 = f64::from(u32::try_from(wins).map_err(|_| {
                StorageError::Integrity("statistics win count exceeds supported range".into())
            })?);
            let darts_f64 = f64::from(u32::try_from(darts).map_err(|_| {
                StorageError::Integrity("statistics dart count exceeds supported range".into())
            })?);
            let points_f64 = f64::from(u32::try_from(total_points).map_err(|_| {
                StorageError::Integrity("statistics points exceed supported range".into())
            })?);
            let three_dart_average = if darts == 0 {
                0.0
            } else {
                round_to(points_f64 / darts_f64 * 3.0, 2)
            };
            let win_rate = if games == 0 {
                0.0
            } else {
                round_to(wins_f64 / games_f64 * 100.0, 1)
            };
            Ok(PlayerStatistics {
                id,
                name,
                avatar,
                color,
                games,
                wins,
                darts,
                total_points,
                total_mode_points: mode_points,
                best_dart: nonnegative(best, "statistics best dart")?,
                misses: nonnegative(misses, "statistics misses")?,
                three_dart_average,
                win_rate,
            })
        })
        .collect()
    }

    /// Aggregates mode outcomes, durations and hit quality by ruleset/options.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed projected values or a failed query.
    pub fn mode_statistics(&self, include_test: bool) -> Result<Vec<ModeStatistics>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT g.game_type, g.ruleset_version, g.options_json,
                    COUNT(DISTINCT g.id),
                    COUNT(DISTINCT CASE WHEN g.status='finished' THEN g.id END),
                    COUNT(DISTINCT CASE WHEN g.status='aborted' THEN g.id END),
                    COUNT(DISTINCT CASE WHEN g.status='interrupted' THEN g.id END),
                    ROUND(AVG(CASE WHEN g.ended_at IS NOT NULL
                        THEN (julianday(g.ended_at)-julianday(g.started_at))*86400 END), 1),
                    COALESCE(SUM((SELECT COUNT(*) FROM throws t WHERE t.game_id=g.id)), 0),
                    COALESCE(SUM((SELECT COUNT(*) FROM throws t
                        WHERE t.game_id=g.id AND t.outcome='success')), 0),
                    COALESCE(SUM((SELECT COUNT(*) FROM throws t
                        WHERE t.game_id=g.id AND t.outcome='partial')), 0),
                    COALESCE(SUM((SELECT COUNT(*) FROM throws t
                        WHERE t.game_id=g.id AND t.outcome='danger')), 0),
                    COALESCE(SUM((SELECT COUNT(*) FROM throws t
                        WHERE t.game_id=g.id AND t.outcome='miss')), 0)
             FROM games g
             WHERE ?1=1 OR g.environment='production'
             GROUP BY g.game_type, g.ruleset_version, g.options_json
             ORDER BY g.game_type, g.ruleset_version, g.options_json",
        )?;
        let rows = statement.query_map([i64::from(include_test)], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<f64>>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, i64>(12)?,
            ))
        })?;
        rows.map(|row| {
            let (
                game_type,
                ruleset,
                options,
                starts,
                finished,
                aborted,
                interrupted,
                average_seconds,
                darts,
                successes,
                partials,
                dangers,
                misses,
            ) = row?;
            let starts = nonnegative(starts, "mode starts")?;
            let finished = nonnegative(finished, "mode finishes")?;
            let darts = nonnegative(darts, "mode darts")?;
            let successes = nonnegative(successes, "mode successes")?;
            Ok(ModeStatistics {
                game_type,
                ruleset_version: nonnegative(ruleset, "mode ruleset version")?,
                options: parse_json(&options, "mode options")?,
                starts,
                finished,
                aborted: nonnegative(aborted, "mode aborts")?,
                interrupted: nonnegative(interrupted, "mode interruptions")?,
                average_seconds,
                darts,
                successes,
                partials: nonnegative(partials, "mode partials")?,
                dangers: nonnegative(dangers, "mode dangers")?,
                misses: nonnegative(misses, "mode misses")?,
                success_rate: percentage(successes, darts, 1)?,
                completion_rate: percentage(finished, starts, 1)?,
            })
        })
        .collect()
    }

    /// Builds a dartboard-segment heatmap from completed games.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid projected values or a failed query.
    pub fn heatmap(
        &self,
        player_id: Option<&str>,
        session_id: Option<&str>,
        game_type: Option<&str>,
        include_test: bool,
    ) -> Result<HeatmapStatistics, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT t.field, t.ring, COUNT(*),
                    COALESCE(SUM(CASE WHEN t.outcome='success' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN t.outcome='danger' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN t.outcome='neutral' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(t.dart_score), 0), COALESCE(SUM(t.mode_points), 0)
             FROM throws t JOIN games g ON g.id=t.game_id
             WHERE t.field IS NOT NULL AND g.status='finished'
               AND (?1=1 OR g.environment='production')
               AND (?2 IS NULL OR t.player_id=?2)
               AND (?3 IS NULL OR g.session_id=?3)
               AND (?4 IS NULL OR g.game_type=?4)
             GROUP BY t.field, t.ring ORDER BY t.field, t.ring",
        )?;
        let query_params = params![i64::from(include_test), player_id, session_id, game_type];
        let rows = statement.query_map(query_params, |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })?;
        let segments = rows
            .map(|row| {
                let (field, ring, darts, successes, dangers, neutral, dart_points, mode_points) =
                    row?;
                Ok(HeatmapSegment {
                    field: nonnegative(field, "heatmap field")?,
                    ring,
                    darts: nonnegative(darts, "heatmap darts")?,
                    successes: nonnegative(successes, "heatmap successes")?,
                    dangers: nonnegative(dangers, "heatmap dangers")?,
                    neutral: nonnegative(neutral, "heatmap neutral hits")?,
                    dart_points,
                    mode_points,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
        let (total_darts, board_hits, misses) = self.connection.query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(CASE WHEN t.field IS NOT NULL THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN t.outcome='miss' THEN 1 ELSE 0 END), 0)
             FROM throws t JOIN games g ON g.id=t.game_id
             WHERE g.status='finished'
               AND (?1=1 OR g.environment='production')
               AND (?2 IS NULL OR t.player_id=?2)
               AND (?3 IS NULL OR g.session_id=?3)
               AND (?4 IS NULL OR g.game_type=?4)",
            params![i64::from(include_test), player_id, session_id, game_type],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;
        Ok(HeatmapStatistics {
            resolution: "dartboard_segment",
            segments,
            total_darts: nonnegative(total_darts, "heatmap total darts")?,
            board_hits: nonnegative(board_hits, "heatmap board hits")?,
            misses: nonnegative(misses, "heatmap misses")?,
        })
    }

    /// Suggests weak target zones from the latest task-aware production throws.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed task telemetry or a failed query.
    #[allow(clippy::too_many_lines)] // Parsing flexible legacy task targets is kept beside aggregation.
    pub fn training_recommendations(
        &self,
        player_id: &str,
    ) -> Result<Option<TrainingRecommendations>, StorageError> {
        if !self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM players WHERE id=?1)",
            [player_id],
            |row| row.get::<_, bool>(0),
        )? {
            return Ok(None);
        }
        let mut statement = self.connection.prepare(
            "SELECT t.field, t.ring, t.outcome, t.task_json
             FROM throws t JOIN games g ON g.id=t.game_id
             WHERE t.player_id=?1 AND g.status='finished'
               AND g.environment='production' AND t.task_json IS NOT NULL
             ORDER BY t.id DESC LIMIT 2000",
        )?;
        let rows = statement.query_map([player_id], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut zones: HashMap<(u64, String), (u64, u64)> = HashMap::new();
        for row in rows {
            let (actual_field, actual_ring, outcome, task_json) = row?;
            let task = parse_json(&task_json, "training task")?;
            let Some(targets) = task.get("targets").and_then(serde_json::Value::as_array) else {
                continue;
            };
            for target in targets {
                let Some(field) = target.get("field").and_then(serde_json::Value::as_u64) else {
                    continue;
                };
                let rings: Vec<&str> = target
                    .get("rings")
                    .and_then(serde_json::Value::as_array)
                    .map(|items| items.iter().filter_map(serde_json::Value::as_str).collect())
                    .or_else(|| {
                        target
                            .get("ring")
                            .and_then(serde_json::Value::as_str)
                            .map(|ring| vec![ring])
                    })
                    .unwrap_or_default();
                for ring in rings {
                    let entry = zones.entry((field, ring.to_owned())).or_default();
                    entry.0 = entry.0.saturating_add(1);
                    if outcome == "success"
                        && actual_field.and_then(|value| u64::try_from(value).ok()) == Some(field)
                        && actual_ring.as_deref() == Some(ring)
                    {
                        entry.1 = entry.1.saturating_add(1);
                    }
                }
            }
        }
        let mut recommendations = zones
            .into_iter()
            .filter_map(|((field, ring), (attempts, successes))| {
                (attempts >= 3).then(|| TrainingRecommendation {
                    field,
                    ring,
                    attempts,
                    successes,
                    success_rate: percentage(successes, attempts, 1).unwrap_or(0.0),
                    starter: false,
                })
            })
            .collect::<Vec<_>>();
        recommendations.sort_by(|left, right| {
            left.success_rate
                .total_cmp(&right.success_rate)
                .then_with(|| right.attempts.cmp(&left.attempts))
                .then_with(|| left.field.cmp(&right.field))
                .then_with(|| left.ring.cmp(&right.ring))
        });
        if recommendations.is_empty() {
            recommendations = vec![
                TrainingRecommendation {
                    field: 20,
                    ring: "double".into(),
                    attempts: 0,
                    successes: 0,
                    success_rate: 0.0,
                    starter: true,
                },
                TrainingRecommendation {
                    field: 25,
                    ring: "single_bull".into(),
                    attempts: 0,
                    successes: 0,
                    success_rate: 0.0,
                    starter: true,
                },
            ];
        }
        recommendations.truncate(8);
        Ok(Some(TrainingRecommendations {
            player_id: player_id.into(),
            recommendations,
        }))
    }

    /// Returns the portable, runtime-secret-free history archive.
    ///
    /// # Errors
    ///
    /// Returns an error when any projected record cannot be loaded.
    pub fn export_data(&self) -> Result<serde_json::Value, StorageError> {
        let players = self.players()?;
        let session_ids = self.ids("SELECT id FROM sessions ORDER BY started_at, id")?;
        let game_ids = self.ids("SELECT id FROM games ORDER BY started_at, id")?;
        let sessions = session_ids
            .iter()
            .map(|id| self.session_detail(id))
            .collect::<Result<Vec<_>, _>>()?;
        let games = game_ids
            .iter()
            .map(|id| {
                Ok(serde_json::json!({
                    "detail": self.game_detail(id)?, "replay": self.game_replay(id)?,
                }))
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
        let exported_at: String =
            self.connection
                .query_row("SELECT CURRENT_TIMESTAMP", [], |row| row.get(0))?;
        Ok(serde_json::json!({
            "schema_version": 2,
            "database_schema_version": CURRENT_SCHEMA_VERSION,
            "exported_at": exported_at,
            "players": players,
            "sessions": sessions,
            "games": games,
        }))
    }

    /// Imports a versioned portable history archive in one transaction.
    ///
    /// Existing identical player profiles are reused. Session or game ID
    /// collisions and conflicting profiles reject the complete archive.
    /// Runtime snapshots, settings, effects and companion credentials are
    /// never imported.
    ///
    /// # Errors
    ///
    /// Returns an integrity error for an incompatible or internally
    /// inconsistent archive. Any `SQLite` failure rolls back every imported row.
    pub fn import_data(&mut self, value: serde_json::Value) -> Result<ImportSummary, StorageError> {
        let archive: PortableArchive = serde_json::from_value(value)
            .map_err(|error| StorageError::Integrity(format!("invalid archive JSON: {error}")))?;
        validate_archive_header(&archive)?;
        let transaction = self.connection.transaction()?;
        let summary = import_archive(&transaction, &archive)?;
        verify_integrity(&transaction)?;
        transaction.commit()?;
        Ok(summary)
    }

    fn ids(&self, query: &str) -> Result<Vec<String>, StorageError> {
        let mut statement = self.connection.prepare(query)?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    /// Loads one session with its ordered profiles and games.
    ///
    /// # Errors
    ///
    /// Returns an error when projected rows are invalid or cannot be queried.
    pub fn session_detail(&self, session_id: &str) -> Result<Option<SessionDetail>, StorageError> {
        let Some(session) = load_session_history(&self.connection, session_id)? else {
            return Ok(None);
        };
        Ok(Some(SessionDetail {
            session,
            players: load_session_players(&self.connection, session_id)?,
            games: load_session_games(&self.connection, session_id)?,
            statistics: self.session_statistics(session_id)?,
        }))
    }

    /// Loads a game with its canonical throws and immutable event history.
    ///
    /// # Errors
    ///
    /// Returns an error when projected rows are invalid or cannot be queried.
    pub fn game_detail(&self, game_id: &str) -> Result<Option<GameDetail>, StorageError> {
        let Some(game) = load_game_history(&self.connection, game_id)? else {
            return Ok(None);
        };
        Ok(Some(GameDetail {
            game,
            throws: load_throws(&self.connection, game_id)?,
            events: load_game_events(&self.connection, game_id)?,
        }))
    }

    /// Loads the complete replay envelope including ineffective audit events.
    ///
    /// # Errors
    ///
    /// Returns an error when stored JSON is invalid or cannot be queried.
    pub fn game_replay(&self, game_id: &str) -> Result<Option<GameReplay>, StorageError> {
        let Some(game) = load_game_history(&self.connection, game_id)? else {
            return Ok(None);
        };
        let mut events = load_game_events(&self.connection, game_id)?;
        if events.is_empty() {
            events = legacy_replay_events(&game, load_throws(&self.connection, game_id)?)?;
        }
        Ok(Some(GameReplay {
            game_id: game.id,
            initial_state: game.initial_state,
            final_state: game.final_state,
            events,
        }))
    }
}

fn validate_archive_header(archive: &PortableArchive) -> Result<(), StorageError> {
    if archive.schema_version != 2 {
        return Err(StorageError::Integrity(format!(
            "unsupported archive schema {}",
            archive.schema_version
        )));
    }
    if archive.database_schema_version == 0 {
        return Err(StorageError::Integrity(
            "archive database schema must be positive".into(),
        ));
    }
    if archive.database_schema_version > CURRENT_SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSchema {
            found: archive.database_schema_version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    if !valid_archive_text(&archive.exported_at, 64)
        || archive.players.len() > 10_000
        || archive.sessions.len() > 10_000
        || archive.games.len() > 100_000
    {
        return Err(StorageError::Integrity(
            "archive header or collection limits are invalid".into(),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // One validation pass keeps cross-record invariants explicit.
fn import_archive(
    transaction: &Transaction<'_>,
    archive: &PortableArchive,
) -> Result<ImportSummary, StorageError> {
    let mut profiles = HashMap::new();
    for player in &archive.players {
        validate_archive_player(player)?;
        if profiles.insert(player.id.as_str(), player).is_some() {
            return Err(StorageError::Integrity(format!(
                "duplicate archive player {}",
                player.id
            )));
        }
    }

    let sessions = archive
        .sessions
        .iter()
        .map(|session| {
            session.as_ref().ok_or_else(|| {
                StorageError::Integrity("archive contains a missing session detail".into())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let games = archive
        .games
        .iter()
        .map(|game| {
            game.detail.as_ref().ok_or_else(|| {
                StorageError::Integrity("archive contains a missing game detail".into())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut session_ids = HashSet::new();
    let mut session_games = HashMap::<&str, &GameHistory>::new();
    let mut session_players = HashMap::<&str, HashSet<&str>>::new();
    for detail in &sessions {
        validate_archive_session(detail, &profiles)?;
        if !session_ids.insert(detail.session.id.as_str()) {
            return Err(StorageError::Integrity(format!(
                "duplicate archive session {}",
                detail.session.id
            )));
        }
        session_players.insert(
            detail.session.id.as_str(),
            detail
                .players
                .iter()
                .map(|player| player.id.as_str())
                .collect(),
        );
        for game in &detail.games {
            if game.session_id != detail.session.id {
                return Err(StorageError::Integrity(format!(
                    "game {} appears under the wrong session projection",
                    game.id
                )));
            }
            if session_games.insert(game.id.as_str(), game).is_some() {
                return Err(StorageError::Integrity(format!(
                    "game {} appears in multiple session projections",
                    game.id
                )));
            }
        }
    }
    let mut game_ids = HashSet::new();
    for (envelope, detail) in archive.games.iter().zip(&games) {
        validate_archive_game(detail, envelope.replay.as_ref(), &profiles)?;
        if !game_ids.insert(detail.game.id.as_str()) {
            return Err(StorageError::Integrity(format!(
                "duplicate archive game {}",
                detail.game.id
            )));
        }
        if !session_ids.contains(detail.game.session_id.as_str()) {
            return Err(StorageError::Integrity(format!(
                "game {} references an absent session",
                detail.game.id
            )));
        }
        if detail.game.players.iter().any(|player| {
            session_players
                .get(detail.game.session_id.as_str())
                .is_none_or(|players| !players.contains(player.id.as_str()))
        }) {
            return Err(StorageError::Integrity(format!(
                "game {} contains a player outside its session",
                detail.game.id
            )));
        }
        if session_games.get(detail.game.id.as_str()).copied() != Some(&detail.game) {
            return Err(StorageError::Integrity(format!(
                "game {} differs between archive projections",
                detail.game.id
            )));
        }
    }
    if session_games.len() != games.len() {
        return Err(StorageError::Integrity(
            "session and game archive projections are incomplete".into(),
        ));
    }

    let mut summary = ImportSummary {
        schema_version: archive.schema_version,
        players_added: 0,
        players_reused: 0,
        sessions_added: 0,
        games_added: 0,
        throws_added: 0,
        events_added: 0,
        interrupted_sessions: 0,
        interrupted_games: 0,
    };
    for player in &archive.players {
        let existing = transaction
            .query_row(
                "SELECT name, avatar, color FROM players WHERE id=?1",
                [&player.id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        if let Some(existing) = existing {
            if (existing.0, existing.1, existing.2)
                != (
                    player.name.clone(),
                    player.avatar.clone(),
                    player.color.clone(),
                )
            {
                return Err(StorageError::Integrity(format!(
                    "player ID {} conflicts with local data",
                    player.id
                )));
            }
            summary.players_reused += 1;
        } else {
            transaction.execute(
                "INSERT INTO players(id, name, avatar, color, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    player.id,
                    player.name,
                    player.avatar,
                    player.color,
                    player.created_at
                ],
            )?;
            summary.players_added += 1;
        }
    }

    for detail in sessions {
        reject_existing_id(transaction, "sessions", &detail.session.id)?;
        let imported_active = detail.session.status == "active";
        let status = if imported_active {
            "interrupted"
        } else {
            detail.session.status.as_str()
        };
        let ended_at = if imported_active {
            Some(archive.exported_at.as_str())
        } else {
            detail.session.ended_at.as_deref()
        };
        transaction.execute(
            "INSERT INTO sessions(id, status, language, started_at, ended_at)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                detail.session.id,
                status,
                detail.session.language,
                detail.session.started_at,
                ended_at
            ],
        )?;
        for (position, player) in detail.players.iter().enumerate() {
            transaction.execute(
                "INSERT INTO session_players(session_id, player_id, position)
                 VALUES(?1, ?2, ?3)",
                params![
                    detail.session.id,
                    player.id,
                    i64::try_from(position).map_err(|_| StorageError::Integrity(
                        "session player position exceeds SQLite range".into()
                    ))?
                ],
            )?;
        }
        summary.sessions_added += 1;
        summary.interrupted_sessions += u64::from(imported_active);
    }

    for detail in games {
        reject_existing_id(transaction, "games", &detail.game.id)?;
        import_game(transaction, detail, &archive.exported_at, &mut summary)?;
    }
    Ok(summary)
}

fn reject_existing_id(
    transaction: &Transaction<'_>,
    table: &str,
    id: &str,
) -> Result<(), StorageError> {
    let query = match table {
        "sessions" => "SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",
        "games" => "SELECT EXISTS(SELECT 1 FROM games WHERE id=?1)",
        _ => {
            return Err(StorageError::Integrity(
                "unsupported collision table".into(),
            ));
        }
    };
    let exists: bool = transaction.query_row(query, [id], |row| row.get(0))?;
    if exists {
        return Err(StorageError::Integrity(format!(
            "{table} ID {id} already exists"
        )));
    }
    Ok(())
}

fn validate_archive_player(player: &PlayerProfile) -> Result<(), StorageError> {
    let valid_color = player.color.len() == 7
        && player.color.starts_with('#')
        && player.color[1..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit());
    if !valid_archive_id(&player.id)
        || player.name.trim().is_empty()
        || player.name.chars().count() > 32
        || !valid_archive_text(&player.avatar, 32)
        || !valid_color
        || !valid_archive_text(&player.created_at, 64)
    {
        return Err(StorageError::Integrity(format!(
            "invalid archive player {}",
            player.id
        )));
    }
    Ok(())
}

fn validate_archive_session(
    detail: &SessionDetail,
    profiles: &HashMap<&str, &PlayerProfile>,
) -> Result<(), StorageError> {
    let session = &detail.session;
    if !valid_archive_id(&session.id)
        || !matches!(
            session.status.as_str(),
            "active" | "finished" | "interrupted"
        )
        || !matches!(session.language.as_str(), "de" | "en")
        || !valid_archive_text(&session.started_at, 64)
        || session
            .ended_at
            .as_deref()
            .is_some_and(|value| !valid_archive_text(value, 64))
        || detail.players.is_empty()
        || detail.players.len() > 8
        || session.player_count != detail.players.len() as u64
        || session.games != detail.games.len() as u64
        || session.finished_games
            != detail
                .games
                .iter()
                .filter(|game| game.status == "finished")
                .count() as u64
        || detail.statistics.len() != detail.players.len()
    {
        return Err(StorageError::Integrity(format!(
            "invalid archive session {}",
            session.id
        )));
    }
    let mut player_ids = HashSet::new();
    for player in &detail.players {
        if profiles.get(player.id.as_str()).copied() != Some(player)
            || !player_ids.insert(player.id.as_str())
        {
            return Err(StorageError::Integrity(format!(
                "session {} contains an invalid player projection",
                session.id
            )));
        }
    }
    let statistic_ids = detail
        .statistics
        .iter()
        .map(|statistic| statistic.id.as_str())
        .collect::<HashSet<_>>();
    if statistic_ids.len() != detail.statistics.len()
        || statistic_ids != player_ids
        || detail.statistics.iter().any(|statistic| {
            profiles.get(statistic.id.as_str()).is_none_or(|profile| {
                statistic.name != profile.name
                    || statistic.avatar != profile.avatar
                    || statistic.color != profile.color
            })
        })
    {
        return Err(StorageError::Integrity(format!(
            "session {} contains an invalid statistics projection",
            session.id
        )));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Validates every imported game relation before insertion.
fn validate_archive_game(
    detail: &GameDetail,
    replay: Option<&GameReplay>,
    profiles: &HashMap<&str, &PlayerProfile>,
) -> Result<(), StorageError> {
    let game = &detail.game;
    if !valid_archive_id(&game.id)
        || !valid_archive_id(&game.session_id)
        || !valid_archive_text(&game.game_type, 64)
        || !matches!(
            game.status.as_str(),
            "running" | "finished" | "aborted" | "interrupted"
        )
        || !valid_archive_optional_text(&game.result_type, 64)
        || !valid_archive_optional_text(&game.finish_reason, 256)
        || game.ruleset_version == 0
        || game.ruleset_version > u64::from(u16::MAX)
        || !valid_archive_optional_text(&game.app_version, 64)
        || !matches!(game.environment.as_str(), "production" | "test")
        || !valid_archive_text(&game.started_at, 64)
        || game
            .ended_at
            .as_deref()
            .is_some_and(|value| !valid_archive_text(value, 64))
        || game.players.is_empty()
        || game.players.len() > 8
        || game.darts != detail.throws.len() as u64
        || game.winner_count != game.winner_ids.len() as u64
        || detail.throws.len() > 100_000
        || detail.events.len() > 100_000
    {
        return Err(StorageError::Integrity(format!(
            "invalid archive game {}",
            game.id
        )));
    }
    let mut player_ids = HashSet::new();
    for (position, player) in game.players.iter().enumerate() {
        let Some(profile) = profiles.get(player.id.as_str()).copied() else {
            return Err(StorageError::Integrity(format!(
                "game {} references an absent player",
                game.id
            )));
        };
        if player.name != profile.name
            || player.avatar != profile.avatar
            || player.color != profile.color
            || player.position != position as u64
            || !player_ids.insert(player.id.as_str())
        {
            return Err(StorageError::Integrity(format!(
                "game {} contains an invalid player projection",
                game.id
            )));
        }
    }
    if game
        .winner_ids
        .iter()
        .any(|winner| !player_ids.contains(winner.as_str()))
    {
        return Err(StorageError::Integrity(format!(
            "game {} contains an invalid winner",
            game.id
        )));
    }
    let event_ids = detail
        .events
        .iter()
        .map(|event| event.id)
        .collect::<HashSet<_>>();
    if event_ids.len() != detail.events.len() {
        return Err(StorageError::Integrity(format!(
            "game {} contains duplicate event IDs",
            game.id
        )));
    }
    for (index, event) in detail.events.iter().enumerate() {
        if event.ordinal != index as u64 + 1
            || !valid_archive_text(&event.event_type, 64)
            || !valid_archive_text(&event.source, 64)
            || !valid_archive_text(&event.created_at, 64)
            || event
                .player_id
                .as_deref()
                .is_some_and(|id| !player_ids.contains(id))
            || event
                .corrects_event_id
                .is_some_and(|id| !detail.events[..index].iter().any(|prior| prior.id == id))
        {
            return Err(StorageError::Integrity(format!(
                "game {} contains an invalid event",
                game.id
            )));
        }
    }
    for throw in &detail.throws {
        if !valid_archive_text(&throw.source, 64)
            || !valid_archive_text(&throw.outcome, 64)
            || !valid_archive_text(&throw.created_at, 64)
            || throw
                .player_id
                .as_deref()
                .is_some_and(|id| !player_ids.contains(id))
            || throw.event_id.is_some_and(|id| !event_ids.contains(&id))
        {
            return Err(StorageError::Integrity(format!(
                "game {} contains an invalid throw",
                game.id
            )));
        }
    }
    if let Some(replay) = replay
        && (replay.game_id != game.id
            || replay.initial_state != game.initial_state
            || replay.final_state != game.final_state
            || (!detail.events.is_empty() && replay.events != detail.events))
    {
        return Err(StorageError::Integrity(format!(
            "game {} has a mismatched replay",
            game.id
        )));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Keeps one game's relational insert and ID remap atomic and local.
fn import_game(
    transaction: &Transaction<'_>,
    detail: &GameDetail,
    exported_at: &str,
    summary: &mut ImportSummary,
) -> Result<(), StorageError> {
    let game = &detail.game;
    let options = serde_json::to_string(&game.options)
        .map_err(|error| StorageError::Integrity(format!("invalid game options: {error}")))?;
    let initial_state = serialize_optional_json(game.initial_state.as_ref())?;
    let final_state = serialize_optional_json(game.final_state.as_ref())?;
    let imported_running = game.status == "running";
    let status = if imported_running {
        "interrupted"
    } else {
        game.status.as_str()
    };
    let result_type = if imported_running {
        ""
    } else {
        game.result_type.as_str()
    };
    let finish_reason = if imported_running {
        "portable_archive_import"
    } else {
        game.finish_reason.as_str()
    };
    let ended_at = if imported_running {
        Some(exported_at)
    } else {
        game.ended_at.as_deref()
    };
    transaction.execute(
        "INSERT INTO games(
            id, session_id, game_type, status, options_json, winner_id,
            result_type, finish_reason, ruleset_version, app_version, environment,
            initial_state_json, final_state_json, started_at, ended_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            game.id,
            game.session_id,
            game.game_type,
            status,
            options,
            game.winner_ids.first(),
            result_type,
            finish_reason,
            archive_sql_i64(game.ruleset_version, "ruleset version")?,
            game.app_version,
            game.environment,
            initial_state,
            final_state,
            game.started_at,
            ended_at
        ],
    )?;
    for player in &game.players {
        transaction.execute(
            "INSERT INTO game_players(game_id, player_id, position, final_score)
             VALUES(?1, ?2, ?3, ?4)",
            params![
                game.id,
                player.id,
                archive_sql_i64(player.position, "game player position")?,
                player.final_score
            ],
        )?;
    }
    for winner in &game.winner_ids {
        transaction.execute(
            "INSERT INTO game_winners(game_id, player_id) VALUES(?1, ?2)",
            params![game.id, winner],
        )?;
    }
    let mut event_ids = HashMap::<u64, i64>::new();
    for event in &detail.events {
        let payload = serde_json::to_string(&event.payload)
            .map_err(|error| StorageError::Integrity(format!("invalid event payload: {error}")))?;
        let task = serialize_optional_json(event.task.as_ref())?;
        let frame = serialize_optional_json(event.frame.as_ref())?;
        let corrects = event
            .corrects_event_id
            .map(|id| {
                event_ids.get(&id).copied().ok_or_else(|| {
                    StorageError::Integrity("corrected event is not available".into())
                })
            })
            .transpose()?;
        transaction.execute(
            "INSERT INTO game_events(
                game_id, ordinal, event_type, player_id, source, payload_json,
                task_json, frame_json, effective, corrects_event_id, created_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                game.id,
                archive_sql_i64(event.ordinal, "event ordinal")?,
                event.event_type,
                event.player_id,
                event.source,
                payload,
                task,
                frame,
                event.effective,
                corrects,
                event.created_at
            ],
        )?;
        event_ids.insert(event.id, transaction.last_insert_rowid());
        summary.events_added += 1;
    }
    for throw in &detail.throws {
        let event = serde_json::to_string(&throw.event)
            .map_err(|error| StorageError::Integrity(format!("invalid dart event: {error}")))?;
        let task = serialize_optional_json(throw.task.as_ref())?;
        let event_id = throw
            .event_id
            .map(|id| {
                event_ids
                    .get(&id)
                    .copied()
                    .ok_or_else(|| StorageError::Integrity("throw event is not available".into()))
            })
            .transpose()?;
        transaction.execute(
            "INSERT INTO throws(
                game_id, action_id, seq, player_id, event_json, score_after,
                round_number, dart_in_turn, field, ring, multiplier, dart_score,
                mode_points, outcome, source, task_json, event_id, created_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                game.id,
                throw
                    .action_id
                    .map(|value| archive_sql_i64(value, "dart action ID"))
                    .transpose()?,
                archive_sql_i64(throw.seq, "dart sequence")?,
                throw.player_id,
                event,
                throw.score_after,
                archive_sql_i64(throw.round_number, "dart round")?,
                archive_sql_i64(throw.dart_in_turn, "dart in turn")?,
                throw.field.map(|value| archive_sql_i64(value, "dart field")).transpose()?,
                throw.ring,
                throw.multiplier.map(|value| archive_sql_i64(value, "dart multiplier")).transpose()?,
                throw.dart_score,
                throw.mode_points,
                throw.outcome,
                throw.source,
                task,
                event_id,
                throw.created_at
            ],
        )?;
        summary.throws_added += 1;
    }
    summary.games_added += 1;
    summary.interrupted_games += u64::from(imported_running);
    Ok(())
}

fn serialize_optional_json(
    value: Option<&serde_json::Value>,
) -> Result<Option<String>, StorageError> {
    value
        .map(|value| {
            serde_json::to_string(value)
                .map_err(|error| StorageError::Integrity(format!("invalid archive JSON: {error}")))
        })
        .transpose()
}

fn archive_sql_i64(value: u64, label: &str) -> Result<i64, StorageError> {
    to_sql_i64(value, label).map_err(StorageError::Integrity)
}

fn valid_archive_id(value: &str) -> bool {
    valid_archive_text(value, 128)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn valid_archive_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn valid_archive_optional_text(value: &str, maximum: usize) -> bool {
    value.len() <= maximum && !value.chars().any(char::is_control)
}

fn load_session_history(
    connection: &Connection,
    session_id: &str,
) -> Result<Option<SessionHistory>, StorageError> {
    let row = connection
        .query_row(
            "SELECT s.id, s.status, s.language, s.started_at, s.ended_at,
                    COUNT(DISTINCT g.id),
                    COUNT(DISTINCT CASE WHEN g.status='finished' THEN g.id END),
                    COUNT(DISTINCT sp.player_id)
             FROM sessions s
             LEFT JOIN session_players sp ON sp.session_id=s.id
             LEFT JOIN games g ON g.session_id=s.id
             WHERE s.id=?1 GROUP BY s.id",
            [session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )
        .optional()?;
    row.map(
        |(id, status, language, started_at, ended_at, games, finished, players)| {
            Ok(SessionHistory {
                id,
                status,
                language,
                started_at,
                ended_at,
                games: nonnegative(games, "session game count")?,
                finished_games: nonnegative(finished, "finished game count")?,
                player_count: nonnegative(players, "session player count")?,
            })
        },
    )
    .transpose()
}

fn load_session_players(
    connection: &Connection,
    session_id: &str,
) -> Result<Vec<PlayerProfile>, StorageError> {
    let mut statement = connection.prepare(
        "SELECT p.id, p.name, p.avatar, p.color, p.created_at
         FROM session_players sp JOIN players p ON p.id=sp.player_id
         WHERE sp.session_id=?1 ORDER BY sp.position, p.id",
    )?;
    let rows = statement.query_map([session_id], |row| {
        Ok(PlayerProfile {
            id: row.get(0)?,
            name: row.get(1)?,
            avatar: row.get(2)?,
            color: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::from)
}

fn load_session_games(
    connection: &Connection,
    session_id: &str,
) -> Result<Vec<GameHistory>, StorageError> {
    let mut statement =
        connection.prepare("SELECT id FROM games WHERE session_id=?1 ORDER BY started_at, id")?;
    let ids = statement
        .query_map([session_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    ids.into_iter()
        .map(|id| {
            load_game_history(connection, &id)?.ok_or_else(|| {
                StorageError::Integrity(format!("session references missing game {id}"))
            })
        })
        .collect()
}

#[allow(clippy::type_complexity)]
fn load_game_history(
    connection: &Connection,
    game_id: &str,
) -> Result<Option<GameHistory>, StorageError> {
    let row = connection
        .query_row(
            "SELECT id, session_id, game_type, status, options_json, result_type,
                    finish_reason, ruleset_version, app_version, environment,
                    initial_state_json, final_state_json, started_at, ended_at,
                    (SELECT COUNT(*) FROM throws WHERE game_id=games.id),
                    (SELECT COUNT(*) FROM game_winners WHERE game_id=games.id)
             FROM games WHERE id=?1",
            [game_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, i64>(15)?,
                ))
            },
        )
        .optional()?;
    let Some((
        id,
        session_id,
        game_type,
        status,
        options,
        result_type,
        finish_reason,
        ruleset_version,
        app_version,
        environment,
        initial_state,
        final_state,
        started_at,
        ended_at,
        darts,
        winner_count,
    )) = row
    else {
        return Ok(None);
    };
    Ok(Some(GameHistory {
        winner_ids: load_winner_ids(connection, game_id)?,
        players: load_game_players(connection, game_id)?,
        id,
        session_id,
        game_type,
        status,
        options: parse_json(&options, "game options")?,
        result_type,
        finish_reason,
        ruleset_version: nonnegative(ruleset_version, "ruleset version")?,
        app_version,
        environment,
        darts: nonnegative(darts, "game dart count")?,
        winner_count: nonnegative(winner_count, "game winner count")?,
        initial_state: parse_optional_json(initial_state, "initial game state")?,
        final_state: parse_optional_json(final_state, "final game state")?,
        started_at,
        ended_at,
    }))
}

fn load_winner_ids(connection: &Connection, game_id: &str) -> Result<Vec<String>, StorageError> {
    let mut statement = connection
        .prepare("SELECT player_id FROM game_winners WHERE game_id=?1 ORDER BY player_id")?;
    statement
        .query_map([game_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::from)
}

fn load_game_players(
    connection: &Connection,
    game_id: &str,
) -> Result<Vec<GamePlayerHistory>, StorageError> {
    let mut statement = connection.prepare(
        "SELECT p.id, p.name, p.avatar, p.color, gp.position, gp.final_score
         FROM game_players gp JOIN players p ON p.id=gp.player_id
         WHERE gp.game_id=?1 ORDER BY gp.position, p.id",
    )?;
    let rows = statement.query_map([game_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, Option<i64>>(5)?,
        ))
    })?;
    rows.map(|row| {
        let (id, name, avatar, color, position, final_score) = row?;
        Ok(GamePlayerHistory {
            id,
            name,
            avatar,
            color,
            position: nonnegative(position, "game player position")?,
            final_score,
        })
    })
    .collect()
}

#[allow(clippy::type_complexity)]
fn load_throws(connection: &Connection, game_id: &str) -> Result<Vec<ThrowHistory>, StorageError> {
    let mut statement = connection.prepare(
        "SELECT id, action_id, seq, player_id, event_json, score_after,
                round_number, dart_in_turn, field, ring, multiplier, dart_score,
                mode_points, outcome, source, task_json, event_id, created_at
         FROM throws WHERE game_id=?1 ORDER BY id",
    )?;
    let rows = statement.query_map([game_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<i64>>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, Option<i64>>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<i64>>(10)?,
            row.get::<_, i64>(11)?,
            row.get::<_, i64>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, String>(14)?,
            row.get::<_, Option<String>>(15)?,
            row.get::<_, Option<i64>>(16)?,
            row.get::<_, String>(17)?,
        ))
    })?;
    rows.map(|row| {
        let (
            id,
            action_id,
            seq,
            player_id,
            event,
            score,
            round,
            dart,
            field,
            ring,
            multiplier,
            dart_score,
            mode_points,
            outcome,
            source,
            task,
            event_id,
            created_at,
        ) = row?;
        Ok(ThrowHistory {
            id: nonnegative(id, "throw ID")?,
            action_id: optional_nonnegative(action_id, "dart action ID")?,
            seq: nonnegative(seq, "dart sequence")?,
            player_id,
            event: parse_json(&event, "dart event")?,
            score_after: score,
            round_number: nonnegative(round, "dart round")?,
            dart_in_turn: nonnegative(dart, "dart in turn")?,
            field: optional_nonnegative(field, "dart field")?,
            ring,
            multiplier: optional_nonnegative(multiplier, "dart multiplier")?,
            dart_score,
            mode_points,
            outcome,
            source,
            task: parse_optional_json(task, "dart task")?,
            event_id: optional_nonnegative(event_id, "throw event ID")?,
            created_at,
        })
    })
    .collect()
}

#[allow(clippy::type_complexity)]
fn load_game_events(
    connection: &Connection,
    game_id: &str,
) -> Result<Vec<GameEventHistory>, StorageError> {
    let mut statement = connection.prepare(
        "SELECT id, ordinal, event_type, player_id, source, payload_json,
                task_json, frame_json, effective, corrects_event_id, created_at
         FROM game_events WHERE game_id=?1 ORDER BY ordinal",
    )?;
    let rows = statement.query_map([game_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, Option<i64>>(9)?,
            row.get::<_, String>(10)?,
        ))
    })?;
    rows.map(|row| {
        let (
            id,
            ordinal,
            event_type,
            player_id,
            source,
            payload,
            task,
            frame,
            effective,
            corrects,
            created_at,
        ) = row?;
        if !matches!(effective, 0 | 1) {
            return Err(StorageError::Integrity(
                "event effective flag is invalid".into(),
            ));
        }
        Ok(GameEventHistory {
            id: nonnegative(id, "game event ID")?,
            ordinal: nonnegative(ordinal, "game event ordinal")?,
            event_type,
            player_id,
            source,
            payload: parse_json(&payload, "game event payload")?,
            task: parse_optional_json(task, "game event task")?,
            frame: parse_optional_json(frame, "game event frame")?,
            effective: effective == 1,
            corrects_event_id: optional_nonnegative(corrects, "corrected event ID")?,
            created_at,
        })
    })
    .collect()
}

fn legacy_replay_events(
    game: &GameHistory,
    throws: Vec<ThrowHistory>,
) -> Result<Vec<GameEventHistory>, StorageError> {
    let initial_score = if game.game_type == "x01" {
        game.options
            .get("start_score")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(501)
    } else {
        0
    };
    let mut players = game
        .players
        .iter()
        .map(|player| {
            (
                player.id.clone(),
                serde_json::json!({
                    "id": player.id,
                    "name": player.name,
                    "avatar": player.avatar,
                    "color": player.color,
                    "score": initial_score,
                }),
            )
        })
        .collect::<Vec<_>>();
    throws
        .into_iter()
        .enumerate()
        .map(|(index, throw)| {
            if let Some(player_id) = throw.player_id.as_deref()
                && let Some((_, player)) = players.iter_mut().find(|(id, _)| id == player_id)
            {
                player["score"] = serde_json::Value::from(throw.score_after);
            }
            let ordinal = u64::try_from(index + 1)
                .map_err(|_| StorageError::Integrity("replay ordinal is out of range".into()))?;
            Ok(GameEventHistory {
                id: throw.event_id.unwrap_or(throw.id),
                ordinal,
                event_type: "throw".into(),
                player_id: throw.player_id.clone(),
                source: throw.source.clone(),
                payload: throw.event.clone(),
                task: throw.task.clone(),
                frame: Some(serde_json::json!({
                    "game_type": game.game_type,
                    "players": players.iter().map(|(_, player)| player).collect::<Vec<_>>(),
                    "current_player_id": throw.player_id,
                    "round_number": throw.round_number,
                    "darts_in_turn": throw.dart_in_turn,
                    "status": "running",
                    "last_event": throw.event,
                    "overlay": {},
                })),
                effective: true,
                corrects_event_id: None,
                created_at: throw.created_at,
            })
        })
        .collect()
}

fn parse_json(value: &str, label: &str) -> Result<serde_json::Value, StorageError> {
    serde_json::from_str(value)
        .map_err(|error| StorageError::Integrity(format!("invalid {label}: {error}")))
}

fn parse_optional_json(
    value: Option<String>,
    label: &str,
) -> Result<Option<serde_json::Value>, StorageError> {
    value.map(|json| parse_json(&json, label)).transpose()
}

fn optional_nonnegative(value: Option<i64>, label: &str) -> Result<Option<u64>, StorageError> {
    value.map(|number| nonnegative(number, label)).transpose()
}

fn nonnegative(value: i64, label: &str) -> Result<u64, StorageError> {
    u64::try_from(value).map_err(|_| StorageError::Integrity(format!("{label} is negative")))
}

fn round_to(value: f64, places: i32) -> f64 {
    let factor = 10_f64.powi(places);
    (value * factor).round() / factor
}

fn percentage(numerator: u64, denominator: u64, places: i32) -> Result<f64, StorageError> {
    if denominator == 0 {
        return Ok(0.0);
    }
    let numerator = f64::from(u32::try_from(numerator).map_err(|_| {
        StorageError::Integrity("percentage numerator exceeds supported range".into())
    })?);
    let denominator = f64::from(u32::try_from(denominator).map_err(|_| {
        StorageError::Integrity("percentage denominator exceeds supported range".into())
    })?);
    Ok(round_to(numerator / denominator * 100.0, places))
}

#[allow(clippy::trivially_copy_pass_by_ref)] // Serde skip predicates receive a reference.
const fn is_false(value: &bool) -> bool {
    !*value
}

fn legacy_backup_path(path: &Path) -> Result<PathBuf, StorageError> {
    let base = format!(".pre-rust-v{CURRENT_SCHEMA_VERSION}.bak");
    for index in 0..10_000_u16 {
        let suffix = if index == 0 {
            base.clone()
        } else {
            format!("{base}.{index}")
        };
        let candidate = sibling_with_suffix(path, &suffix)?;
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(StorageError::Integrity(
        "too many retained legacy database backups".into(),
    ))
}

fn staging_path(path: &Path) -> Result<PathBuf, StorageError> {
    sibling_with_suffix(path, ".importing")
}

fn sibling_with_suffix(path: &Path, suffix: &str) -> Result<PathBuf, StorageError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| StorageError::Integrity("database path has no file name".into()))?;
    let mut suffixed = file_name.to_os_string();
    suffixed.push(suffix);
    Ok(path.with_file_name(suffixed))
}

fn remove_staging_file(path: &Path) -> Result<(), StorageError> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    remove_sidecar_files(path)
}

fn remove_sidecar_files(path: &Path) -> Result<(), StorageError> {
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let sidecar = PathBuf::from(sidecar);
        if sidecar.exists() {
            fs::remove_file(sidecar)?;
        }
    }
    Ok(())
}

fn online_backup(source_path: &Path, destination_path: &Path) -> Result<(), StorageError> {
    let source = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut destination = Connection::open(destination_path)?;
    {
        let backup = Backup::new(&source, &mut destination)?;
        backup.run_to_completion(128, Duration::from_millis(5), None)?;
    }
    verify_integrity(&destination)
}

fn validate_legacy_database(path: &Path) -> Result<(), StorageError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    verify_integrity(&connection)?;
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > 2 {
        return Err(StorageError::Integrity(format!(
            "legacy Python database schema {version} is not supported"
        )));
    }
    for query in [
        "SELECT id, name, avatar, color, created_at FROM players LIMIT 0",
        "SELECT id, status, language, started_at, ended_at FROM sessions LIMIT 0",
        "SELECT session_id, player_id, position FROM session_players LIMIT 0",
        "SELECT id, session_id, game_type, status, options_json, winner_id, result_type, \
                finish_reason, ruleset_version, app_version, environment, initial_state_json, \
                final_state_json, started_at, ended_at FROM games LIMIT 0",
        "SELECT game_id, player_id, position, final_score FROM game_players LIMIT 0",
        "SELECT game_id, player_id FROM game_winners LIMIT 0",
        "SELECT id, game_id, ordinal, event_type, player_id, source, payload_json, task_json, \
                frame_json, effective, corrects_event_id, created_at FROM game_events LIMIT 0",
        "SELECT id, game_id, seq, player_id, event_json, score_after, round_number, dart_in_turn, \
                field, ring, multiplier, dart_score, mode_points, outcome, source, task_json, \
                event_id, created_at FROM throws LIMIT 0",
        "SELECT key, value_json, updated_at FROM runtime_state LIMIT 0",
    ] {
        connection.prepare(query).map_err(|error| {
            StorageError::Integrity(format!(
                "legacy Python database does not match schema 2: {error}"
            ))
        })?;
    }
    Ok(())
}

fn legacy_runtime_settings(connection: &Connection) -> Result<RuntimeSettings, StorageError> {
    let mut settings = RuntimeSettings::default();
    if let Some(calibration) =
        legacy_runtime_value::<CalibrationSettings>(connection, "calibration")?
        && valid_legacy_calibration(&calibration)
    {
        settings.calibration = calibration;
    }
    if let Some(geometry) =
        legacy_runtime_value::<ProjectorGeometry>(connection, "projector_geometry")?
        && geometry.width > 0
        && geometry.height > 0
    {
        settings.projector_geometry = geometry;
    }
    if let Some(sound) = legacy_runtime_value::<serde_json::Value>(connection, "sound")? {
        settings.sound.enabled = sound["enabled"].as_bool().unwrap_or(false);
        settings.sound.output = match sound["output"].as_str() {
            Some("controller") => SoundOutput::Controller,
            Some("both") => SoundOutput::Both,
            _ => SoundOutput::Projector,
        };
        settings.sound.status = if settings.sound.enabled {
            SoundStatus::Starting
        } else {
            SoundStatus::Disabled
        };
    }
    if let Some(theme) = legacy_runtime_value::<ArtTheme>(connection, "art_theme")? {
        settings.art_theme = theme;
    }
    if let Some(language) = legacy_runtime_value::<UiLanguage>(connection, "ui_language")? {
        settings.ui_language = language;
    }
    Ok(settings)
}

fn legacy_runtime_value<T: serde::de::DeserializeOwned>(
    connection: &Connection,
    key: &str,
) -> Result<Option<T>, StorageError> {
    let value = connection
        .query_row(
            "SELECT value_json FROM runtime_state WHERE key=?1",
            [key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(value.and_then(|json| serde_json::from_str(&json).ok()))
}

fn valid_legacy_calibration(calibration: &CalibrationSettings) -> bool {
    calibration.scale.is_finite()
        && (0.5..=2.0).contains(&calibration.scale)
        && calibration.offset_x.is_finite()
        && (-1.0..=1.0).contains(&calibration.offset_x)
        && calibration.offset_y.is_finite()
        && (-1.0..=1.0).contains(&calibration.offset_y)
        && calibration.corners.iter().all(|point| {
            point.x.is_finite()
                && (0.0..=1.0).contains(&point.x)
                && point.y.is_finite()
                && (0.0..=1.0).contains(&point.y)
        })
}

#[allow(clippy::too_many_lines)] // Sequential SQL is kept readable beside its schema version.
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
    if version < 3 {
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
            CREATE TABLE IF NOT EXISTS runtime_journal (
                revision INTEGER PRIMARY KEY,
                command_id TEXT NOT NULL UNIQUE,
                runtime_instance_id TEXT NOT NULL,
                action_json TEXT NOT NULL,
                snapshot_json TEXT NOT NULL,
                committed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS players (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                avatar TEXT NOT NULL,
                color TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                language TEXT NOT NULL DEFAULT 'de',
                started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                ended_at TEXT
            );
            CREATE TABLE IF NOT EXISTS session_players (
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                player_id TEXT NOT NULL REFERENCES players(id),
                position INTEGER NOT NULL,
                PRIMARY KEY(session_id, player_id)
            );
            CREATE TABLE IF NOT EXISTS games (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                game_type TEXT NOT NULL,
                status TEXT NOT NULL,
                options_json TEXT NOT NULL,
                winner_id TEXT REFERENCES players(id),
                result_type TEXT NOT NULL DEFAULT '',
                finish_reason TEXT NOT NULL DEFAULT '',
                ruleset_version INTEGER NOT NULL DEFAULT 1,
                app_version TEXT NOT NULL DEFAULT '',
                environment TEXT NOT NULL DEFAULT 'production',
                initial_state_json TEXT,
                final_state_json TEXT,
                started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                ended_at TEXT
            );
            CREATE TABLE IF NOT EXISTS game_players (
                game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                player_id TEXT NOT NULL REFERENCES players(id),
                position INTEGER NOT NULL,
                final_score INTEGER,
                PRIMARY KEY(game_id, player_id)
            );
            CREATE TABLE IF NOT EXISTS game_winners (
                game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                player_id TEXT NOT NULL REFERENCES players(id),
                PRIMARY KEY(game_id, player_id)
            );
            CREATE TABLE IF NOT EXISTS game_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                ordinal INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                player_id TEXT REFERENCES players(id),
                source TEXT NOT NULL DEFAULT 'system',
                payload_json TEXT NOT NULL,
                task_json TEXT,
                frame_json TEXT,
                effective INTEGER NOT NULL DEFAULT 1,
                corrects_event_id INTEGER REFERENCES game_events(id),
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(game_id, ordinal)
            );
            CREATE TABLE IF NOT EXISTS throws (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                seq INTEGER NOT NULL,
                player_id TEXT REFERENCES players(id),
                event_json TEXT NOT NULL,
                score_after INTEGER NOT NULL,
                round_number INTEGER NOT NULL,
                dart_in_turn INTEGER NOT NULL,
                field INTEGER,
                ring TEXT,
                multiplier INTEGER,
                dart_score INTEGER NOT NULL,
                mode_points INTEGER NOT NULL DEFAULT 0,
                outcome TEXT NOT NULL DEFAULT 'neutral',
                source TEXT NOT NULL DEFAULT 'unknown',
                task_json TEXT,
                event_id INTEGER REFERENCES game_events(id),
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_games_session ON games(session_id);
            CREATE INDEX IF NOT EXISTS idx_games_type_status ON games(game_type, status);
            CREATE INDEX IF NOT EXISTS idx_game_players_player ON game_players(player_id);
            CREATE INDEX IF NOT EXISTS idx_game_winners_player ON game_winners(player_id);
            CREATE INDEX IF NOT EXISTS idx_game_events_game ON game_events(game_id, ordinal);
            CREATE INDEX IF NOT EXISTS idx_game_events_player ON game_events(player_id);
            CREATE INDEX IF NOT EXISTS idx_throws_game ON throws(game_id);
            CREATE INDEX IF NOT EXISTS idx_throws_player ON throws(player_id);
            CREATE INDEX IF NOT EXISTS idx_throws_heatmap ON throws(player_id, field, ring);
            PRAGMA user_version=3;
            COMMIT;
            ",
        )?;
    }
    if version < 4 {
        connection.execute_batch(
            "
            BEGIN;
            ALTER TABLE throws ADD COLUMN action_id INTEGER;
            CREATE INDEX idx_throws_action ON throws(game_id, action_id);
            PRAGMA user_version=4;
            COMMIT;
            ",
        )?;
    }
    if version < 5 {
        connection.execute_batch(
            "
            BEGIN;
            CREATE TABLE companion_devices (
                device_id TEXT PRIMARY KEY,
                device_name TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role = 'projector'),
                token_hash TEXT NOT NULL CHECK(length(token_hash) = 64),
                paired_at_ms INTEGER NOT NULL,
                revoked_at_ms INTEGER
            );
            CREATE INDEX idx_companion_devices_active
                ON companion_devices(revoked_at_ms, device_id);
            PRAGMA user_version=5;
            COMMIT;
            ",
        )?;
    }
    if version < 6 {
        connection.execute_batch(
            "
            BEGIN;
            CREATE TABLE app_preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL CHECK(length(value) <= 4096)
            );
            PRAGMA user_version=6;
            COMMIT;
            ",
        )?;
    }
    Ok(())
}

fn validate_preference_key(key: &str) -> Result<(), StorageError> {
    if key.is_empty()
        || key.len() > 64
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(StorageError::Integrity("invalid app preference key".into()));
    }
    Ok(())
}

fn verify_integrity(connection: &Connection) -> Result<(), StorageError> {
    let result: String = connection.pragma_query_value(None, "quick_check", |row| row.get(0))?;
    if result != "ok" {
        return Err(StorageError::Integrity(result));
    }
    let violation = connection
        .prepare("PRAGMA foreign_key_check")?
        .query_row([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .optional()?;
    if let Some((table, row_id)) = violation {
        return Err(StorageError::Integrity(format!(
            "foreign key violation in {table} row {row_id}"
        )));
    }
    Ok(())
}

#[derive(Debug)]
struct GameProjection {
    players: Vec<(String, i64)>,
    current_player_index: usize,
    round_number: u16,
    darts_in_turn: u8,
    winner_ids: Vec<String>,
    result_type: String,
    last_bust: bool,
}

fn game_projection(game: &RuntimeGame) -> GameProjection {
    match game {
        RuntimeGame::CountUp(game) => {
            let state = game.state();
            GameProjection {
                players: state
                    .players
                    .iter()
                    .map(|player| (player.id.clone(), i64::from(player.score)))
                    .collect(),
                current_player_index: state.current_player_index,
                round_number: state.round_number,
                darts_in_turn: state.darts_in_turn,
                winner_ids: state.winner_ids.clone(),
                result_type: state.result_type.clone(),
                last_bust: false,
            }
        }
        RuntimeGame::X01(game) => {
            let state = game.state();
            GameProjection {
                players: state
                    .players
                    .iter()
                    .map(|player| (player.id.clone(), i64::from(player.score)))
                    .collect(),
                current_player_index: state.current_player_index,
                round_number: state.round_number,
                darts_in_turn: state.darts_in_turn,
                winner_ids: state.winner_ids.clone(),
                result_type: state.result_type.clone(),
                last_bust: state.last_bust,
            }
        }
        RuntimeGame::Registered(game) => {
            let state = game.state();
            GameProjection {
                players: state
                    .players
                    .iter()
                    .map(|player| (player.id.clone(), player.score))
                    .collect(),
                current_player_index: state.current_player_index,
                round_number: state.round_number,
                darts_in_turn: state.darts_in_turn,
                winner_ids: state.winner_ids.clone(),
                result_type: state.result_type.clone(),
                last_bust: false,
            }
        }
    }
}

#[allow(clippy::too_many_lines)] // One exhaustive action match documents the atomic projection.
fn project_domain(
    transaction: &Transaction<'_>,
    request: &CommitRequest<'_>,
) -> Result<(), String> {
    let action: RuntimeAction =
        serde_json::from_str(request.action_json).map_err(|error| error.to_string())?;
    let previous: RuntimeSnapshot =
        serde_json::from_str(request.previous_snapshot_json).map_err(|error| error.to_string())?;
    let next: RuntimeSnapshot =
        serde_json::from_str(request.snapshot_json).map_err(|error| error.to_string())?;

    match &action {
        RuntimeAction::CreatePlayer { player } => {
            transaction
                .execute(
                    "INSERT INTO players(id, name, avatar, color)
                     VALUES(?1, ?2, ?3, ?4)
                     ON CONFLICT(id) DO UPDATE SET
                       name=excluded.name, avatar=excluded.avatar, color=excluded.color",
                    params![player.id, player.name, player.avatar, player.color],
                )
                .map_err(|error| error.to_string())?;
        }
        RuntimeAction::StartSession {
            session_id,
            players,
            ..
        } => {
            for player in players {
                transaction
                    .execute(
                        "INSERT INTO players(id, name, avatar, color)
                         VALUES(?1, ?2, ?3, ?4)
                         ON CONFLICT(id) DO UPDATE SET
                           name=excluded.name, avatar=excluded.avatar, color=excluded.color",
                        params![player.id, player.name, player.avatar, player.color],
                    )
                    .map_err(|error| error.to_string())?;
            }
            transaction
                .execute(
                    "INSERT INTO sessions(id, status) VALUES(?1, 'active')",
                    [session_id],
                )
                .map_err(|error| error.to_string())?;
            for (position, player) in players.iter().enumerate() {
                let position = i64::try_from(position)
                    .map_err(|_| "session position exceeds SQLite range".to_string())?;
                transaction
                    .execute(
                        "INSERT INTO session_players(session_id, player_id, position)
                         VALUES(?1, ?2, ?3)",
                        params![session_id, player.id, position],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        RuntimeAction::StartPreparedGame { .. } | RuntimeAction::StartRematch { .. } => {
            insert_game(transaction, &next, request.snapshot_json)?;
        }
        RuntimeAction::BoardButton { .. } => {
            if previous.session.state().screen == Screen::GameResult {
                if next.session.state().screen == Screen::Countdown {
                    insert_game(transaction, &next, request.snapshot_json)?;
                }
            } else if previous.session.state().game_id.is_some()
                && let Some(game) = previous.game.as_ref()
            {
                record_simple_game_event(
                    transaction,
                    &previous,
                    if game_is_hold(game) {
                        "continue_turn"
                    } else {
                        "next_player"
                    },
                    "board",
                    request.snapshot_json,
                )?;
            }
        }
        RuntimeAction::Dart { event, source } => {
            if next.session.state().game_id.is_some() {
                record_dart(
                    transaction,
                    &previous,
                    &next,
                    event,
                    *source,
                    request.snapshot_json,
                )?;
            }
        }
        RuntimeAction::CorrectDart {
            action_id,
            replacement,
            source,
        } => {
            if previous.session.state().game_id.is_some() {
                project_dart_edit(
                    transaction,
                    &previous,
                    &next,
                    *action_id,
                    Some(replacement),
                    Some(*source),
                    request.snapshot_json,
                )?;
            }
        }
        RuntimeAction::DeleteDart { action_id } => {
            if previous.session.state().game_id.is_some() {
                project_dart_edit(
                    transaction,
                    &previous,
                    &next,
                    *action_id,
                    None,
                    None,
                    request.snapshot_json,
                )?;
            }
        }
        RuntimeAction::Continue => {
            if previous.session.state().game_id.is_some() {
                record_simple_game_event(
                    transaction,
                    &previous,
                    "continue_turn",
                    "system",
                    request.snapshot_json,
                )?;
            }
        }
        RuntimeAction::NextPlayer => {
            if previous.session.state().game_id.is_some() {
                record_simple_game_event(
                    transaction,
                    &previous,
                    "next_player",
                    "operator",
                    request.snapshot_json,
                )?;
            }
        }
        RuntimeAction::Undo => {
            record_undo(transaction, &previous, request.snapshot_json)?;
        }
        RuntimeAction::AbortGame => {
            if let Some(game_id) = previous.session.state().game_id.as_deref() {
                transaction
                    .execute(
                        "UPDATE games SET status='aborted', winner_id=NULL,
                           result_type='', finish_reason='user_abort',
                           final_state_json=?1, ended_at=CURRENT_TIMESTAMP
                         WHERE id=?2 AND status='running'",
                        params![request.previous_snapshot_json, game_id],
                    )
                    .map_err(|error| error.to_string())?;
                transaction
                    .execute("DELETE FROM game_winners WHERE game_id=?1", [game_id])
                    .map_err(|error| error.to_string())?;
            }
        }
        RuntimeAction::EndSession => {
            if let Some(session_id) = next.session.state().session_id.as_deref() {
                transaction
                    .execute(
                        "UPDATE sessions SET status='finished', ended_at=CURRENT_TIMESTAMP
                         WHERE id=?1 AND status='active'",
                        [session_id],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        RuntimeAction::CloseSession => {
            let state = previous.session.state();
            if state.session_status == Some(SessionStatus::Active)
                && let Some(session_id) = state.session_id.as_deref()
            {
                transaction
                    .execute(
                        "UPDATE sessions SET status='interrupted', ended_at=CURRENT_TIMESTAMP
                         WHERE id=?1 AND status='active'",
                        [session_id],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        RuntimeAction::PrepareGame { .. }
        | RuntimeAction::CancelPreparedGame
        | RuntimeAction::MarkGamePlaying
        | RuntimeAction::SelectStarter { .. }
        | RuntimeAction::NextGame
        | RuntimeAction::StartCountUp { .. }
        | RuntimeAction::StartX01 { .. }
        | RuntimeAction::StartRegistered { .. }
        | RuntimeAction::GameAction { .. }
        | RuntimeAction::UpdateCalibration { .. }
        | RuntimeAction::ResetCalibration
        | RuntimeAction::ReportProjectorGeometry { .. }
        | RuntimeAction::UpdateSoundSettings { .. }
        | RuntimeAction::ReportSoundStatus { .. }
        | RuntimeAction::UpdateArtTheme { .. }
        | RuntimeAction::UpdateUiLanguage { .. }
        | RuntimeAction::SetCorrectionLock { .. }
        | RuntimeAction::SoundTest { .. }
        | RuntimeAction::SetDisplayOverride { .. } => {}
    }

    let before = previous.session.state();
    let after = next.session.state();
    if before.screen != Screen::GameResult && after.screen == Screen::GameResult {
        finish_game(transaction, &next, request.snapshot_json)?;
    } else if before.screen == Screen::GameResult
        && after.screen == Screen::Playing
        && let Some(game_id) = after.game_id.as_deref()
    {
        transaction
            .execute(
                "UPDATE games SET status='running', winner_id=NULL, result_type='',
                       finish_reason='', final_state_json=NULL, ended_at=NULL WHERE id=?1",
                [game_id],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM game_winners WHERE game_id=?1", [game_id])
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn game_is_hold(game: &RuntimeGame) -> bool {
    match game {
        RuntimeGame::CountUp(game) => game.state().status == GameStatus::Hold,
        RuntimeGame::X01(game) => game.state().status == GameStatus::Hold,
        RuntimeGame::Registered(game) => game.state().status == GameStatus::Hold,
    }
}

fn insert_game(
    transaction: &Transaction<'_>,
    snapshot: &RuntimeSnapshot,
    snapshot_json: &str,
) -> Result<(), String> {
    let session = snapshot.session.state();
    let session_id = session
        .session_id
        .as_deref()
        .ok_or_else(|| "started game has no session ID".to_string())?;
    let game_id = session
        .game_id
        .as_deref()
        .ok_or_else(|| "started game has no game ID".to_string())?;
    let prepared = session
        .prepared_game
        .as_ref()
        .ok_or_else(|| "started game has no prepared mode".to_string())?;
    let options_json =
        serde_json::to_string(&prepared.options).map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO games(
                id, session_id, game_type, status, options_json, initial_state_json
             ) VALUES(?1, ?2, ?3, 'running', ?4, ?5)",
            params![
                game_id,
                session_id,
                prepared.game_type,
                options_json,
                snapshot_json
            ],
        )
        .map_err(|error| error.to_string())?;
    for (position, player_id) in session.game_player_ids.iter().enumerate() {
        let position = i64::try_from(position)
            .map_err(|_| "game position exceeds SQLite range".to_string())?;
        transaction
            .execute(
                "INSERT INTO game_players(game_id, player_id, position)
                 VALUES(?1, ?2, ?3)",
                params![game_id, player_id, position],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Keeps one throw projection visibly atomic.
fn record_dart(
    transaction: &Transaction<'_>,
    previous: &RuntimeSnapshot,
    next: &RuntimeSnapshot,
    event: &impl Serialize,
    source: sdb_contracts::DartSource,
    frame_json: &str,
) -> Result<(), String> {
    let game_id = next
        .session
        .state()
        .game_id
        .as_deref()
        .ok_or_else(|| "session dart has no game ID".to_string())?;
    let before = previous
        .game
        .as_ref()
        .map(game_projection)
        .ok_or_else(|| "dart has no previous game state".to_string())?;
    let after = next
        .game
        .as_ref()
        .map(game_projection)
        .ok_or_else(|| "dart has no resulting game state".to_string())?;
    let (player_id, _) = before
        .players
        .get(before.current_player_index)
        .ok_or_else(|| "dart player index is invalid".to_string())?;
    let score_after = after
        .players
        .iter()
        .find(|(id, _)| id == player_id)
        .map(|(_, score)| *score)
        .ok_or_else(|| "dart player is absent from resulting state".to_string())?;
    let event_value = serde_json::to_value(event).map_err(|error| error.to_string())?;
    let event_json = serde_json::to_string(&event_value).map_err(|error| error.to_string())?;
    let action_id = match next.game.as_ref() {
        Some(RuntimeGame::CountUp(game)) => {
            game.dart_records().last().map(|record| record.action_id)
        }
        Some(RuntimeGame::X01(game)) => game.dart_records().last().map(|record| record.action_id),
        Some(RuntimeGame::Registered(game)) => {
            game.dart_records().last().map(|record| record.action_id)
        }
        _ => None,
    };
    let audit_payload = serde_json::json!({
        "action_id": action_id,
        "event": &event_value,
    })
    .to_string();
    let source_name = dart_source_name(source);
    if source == sdb_contracts::DartSource::ProjectorTest {
        transaction
            .execute(
                "UPDATE games SET environment='test' WHERE id=?1 AND status='running'",
                [game_id],
            )
            .map_err(|error| error.to_string())?;
    }
    let event_id = insert_game_event(
        transaction,
        GameEventInsert {
            game_id,
            event_type: "throw",
            player_id: Some(player_id),
            source: source_name,
            payload_json: &audit_payload,
            frame_json: Some(frame_json),
            corrects_event_id: None,
        },
    )?;
    let seq = event_value
        .get("seq")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let field = event_value.get("field").and_then(serde_json::Value::as_u64);
    let ring = event_value.get("ring").and_then(serde_json::Value::as_str);
    let multiplier = event_value
        .get("multiplier")
        .and_then(serde_json::Value::as_u64);
    let dart_score = event_value
        .get("score")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let outcome = if after.last_bust {
        "bust"
    } else if event_value.get("type").and_then(serde_json::Value::as_str) == Some("miss") {
        "miss"
    } else {
        "success"
    };
    transaction
        .execute(
            "INSERT INTO throws(
               game_id, seq, player_id, event_json, score_after, round_number,
               dart_in_turn, field, ring, multiplier, dart_score, outcome, source,
               event_id, action_id
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                game_id,
                to_sql_i64(seq, "dart sequence")?,
                player_id,
                event_json,
                score_after,
                i64::from(after.round_number),
                i64::from(after.darts_in_turn),
                field
                    .map(|value| to_sql_i64(value, "dart field"))
                    .transpose()?,
                ring,
                multiplier
                    .map(|value| to_sql_i64(value, "dart multiplier"))
                    .transpose()?,
                to_sql_i64(dart_score, "dart score")?,
                outcome,
                source_name,
                event_id,
                action_id
                    .map(|value| to_sql_i64(value, "dart action ID"))
                    .transpose()?
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

const fn dart_source_name(source: sdb_contracts::DartSource) -> &'static str {
    match source {
        sdb_contracts::DartSource::Board => "board",
        sdb_contracts::DartSource::ProjectorTest => "projector_test",
        sdb_contracts::DartSource::ManualCorrection => "manual_correction",
    }
}

fn project_dart_edit(
    transaction: &Transaction<'_>,
    previous: &RuntimeSnapshot,
    next: &RuntimeSnapshot,
    action_id: u64,
    replacement: Option<&DartEvent>,
    source: Option<DartSource>,
    frame_json: &str,
) -> Result<(), String> {
    let game_id = previous
        .session
        .state()
        .game_id
        .as_deref()
        .ok_or_else(|| "dart edit has no game ID".to_string())?;
    let sql_action_id = to_sql_i64(action_id, "dart action ID")?;
    let target_event_id = transaction
        .query_row(
            "SELECT event_id FROM throws
             WHERE game_id=?1 AND action_id=?2 LIMIT 1",
            params![game_id, sql_action_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .flatten()
        .ok_or_else(|| "dart edit target has no audit event".to_string())?;
    transaction
        .execute(
            "UPDATE game_events SET effective=0 WHERE id=?1 AND effective=1",
            [target_event_id],
        )
        .map_err(|error| error.to_string())?;

    let event_type = if replacement.is_some() {
        "throw_corrected"
    } else {
        "throw_deleted"
    };
    let source_name = source.map_or("control", dart_source_name);
    let replacement_value = replacement
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| error.to_string())?;
    let payload = serde_json::json!({
        "action_id": action_id,
        "replacement": replacement_value,
    })
    .to_string();
    let edit_event_id = insert_game_event(
        transaction,
        GameEventInsert {
            game_id,
            event_type,
            player_id: None,
            source: source_name,
            payload_json: &payload,
            frame_json: Some(frame_json),
            corrects_event_id: Some(target_event_id),
        },
    )?;
    if source == Some(sdb_contracts::DartSource::ProjectorTest) {
        transaction
            .execute("UPDATE games SET environment='test' WHERE id=?1", [game_id])
            .map_err(|error| error.to_string())?;
    }
    rewrite_dart_throws(
        transaction,
        next,
        game_id,
        action_id,
        replacement
            .is_some()
            .then_some((source_name, edit_event_id)),
    )?;
    if previous.session.state().screen == Screen::GameResult
        && next.session.state().screen == Screen::GameResult
    {
        finish_game(transaction, next, frame_json)?;
    }
    Ok(())
}

#[derive(Debug)]
struct StoredThrowMeta {
    source: String,
    event_id: Option<i64>,
    created_at: String,
}

#[derive(Debug)]
struct ReplayedDartRecord {
    action_id: u64,
    event: DartEvent,
    player_id: String,
    score_after: i64,
    round_number: u16,
    dart_in_turn: u8,
    outcome: String,
}

impl From<sdb_game_core::X01DartRecord> for ReplayedDartRecord {
    fn from(record: sdb_game_core::X01DartRecord) -> Self {
        Self {
            action_id: record.action_id,
            event: record.event,
            player_id: record.player_id,
            score_after: i64::from(record.score_after),
            round_number: record.round_number,
            dart_in_turn: record.dart_in_turn,
            outcome: record.outcome,
        }
    }
}

impl From<sdb_game_core::CountUpDartRecord> for ReplayedDartRecord {
    fn from(record: sdb_game_core::CountUpDartRecord) -> Self {
        Self {
            action_id: record.action_id,
            event: record.event,
            player_id: record.player_id,
            score_after: i64::from(record.score_after),
            round_number: record.round_number,
            dart_in_turn: record.dart_in_turn,
            outcome: record.outcome,
        }
    }
}

impl From<sdb_game_core::RegisteredDartRecord> for ReplayedDartRecord {
    fn from(record: sdb_game_core::RegisteredDartRecord) -> Self {
        Self {
            action_id: record.action_id,
            event: record.event,
            player_id: record.player_id,
            score_after: record.score_after,
            round_number: record.round_number,
            dart_in_turn: record.dart_in_turn,
            outcome: record.outcome,
        }
    }
}

fn rewrite_dart_throws(
    transaction: &Transaction<'_>,
    snapshot: &RuntimeSnapshot,
    game_id: &str,
    edited_action_id: u64,
    edit: Option<(&str, i64)>,
) -> Result<(), String> {
    let records: Vec<ReplayedDartRecord> = match snapshot.game.as_ref() {
        Some(RuntimeGame::CountUp(game)) => {
            game.dart_records().into_iter().map(Into::into).collect()
        }
        Some(RuntimeGame::X01(game)) => game.dart_records().into_iter().map(Into::into).collect(),
        Some(RuntimeGame::Registered(game)) => {
            game.dart_records().into_iter().map(Into::into).collect()
        }
        _ => return Err("dart edits are unsupported for this game".into()),
    };
    let mut statement = transaction
        .prepare(
            "SELECT action_id, source, event_id, created_at FROM throws
             WHERE game_id=?1 AND action_id IS NOT NULL",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([game_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                StoredThrowMeta {
                    source: row.get(1)?,
                    event_id: row.get(2)?,
                    created_at: row.get(3)?,
                },
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut metadata = HashMap::new();
    for row in rows {
        let (action_id, stored) = row.map_err(|error| error.to_string())?;
        let action_id = u64::try_from(action_id)
            .map_err(|_| "stored dart action ID is negative".to_string())?;
        metadata.insert(action_id, stored);
    }
    drop(statement);
    transaction
        .execute("DELETE FROM throws WHERE game_id=?1", [game_id])
        .map_err(|error| error.to_string())?;
    let now: String = transaction
        .query_row("SELECT CURRENT_TIMESTAMP", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    for record in records {
        let stored = metadata.remove(&record.action_id);
        let (source, event_id) = if record.action_id == edited_action_id {
            edit.map_or_else(
                || {
                    stored.as_ref().map_or_else(
                        || ("unknown".to_string(), None),
                        |meta| (meta.source.clone(), meta.event_id),
                    )
                },
                |(source, event_id)| (source.to_string(), Some(event_id)),
            )
        } else {
            stored.as_ref().map_or_else(
                || ("unknown".to_string(), None),
                |meta| (meta.source.clone(), meta.event_id),
            )
        };
        let created_at = stored.map_or_else(|| now.clone(), |meta| meta.created_at);
        insert_replayed_throw(
            transaction,
            game_id,
            &record,
            &source,
            event_id,
            &created_at,
        )?;
    }
    Ok(())
}

fn insert_replayed_throw(
    transaction: &Transaction<'_>,
    game_id: &str,
    record: &ReplayedDartRecord,
    source: &str,
    event_id: Option<i64>,
    created_at: &str,
) -> Result<(), String> {
    let event_value = serde_json::to_value(&record.event).map_err(|error| error.to_string())?;
    let event_json = serde_json::to_string(&event_value).map_err(|error| error.to_string())?;
    let field = event_value.get("field").and_then(serde_json::Value::as_u64);
    let ring = event_value.get("ring").and_then(serde_json::Value::as_str);
    let multiplier = event_value
        .get("multiplier")
        .and_then(serde_json::Value::as_u64);
    transaction
        .execute(
            "INSERT INTO throws(
               game_id, seq, player_id, event_json, score_after, round_number,
               dart_in_turn, field, ring, multiplier, dart_score, outcome,
               source, event_id, action_id, created_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                      ?13, ?14, ?15, ?16)",
            params![
                game_id,
                to_sql_i64(record.event.seq(), "dart sequence")?,
                record.player_id,
                event_json,
                record.score_after,
                i64::from(record.round_number),
                i64::from(record.dart_in_turn),
                field
                    .map(|value| to_sql_i64(value, "dart field"))
                    .transpose()?,
                ring,
                multiplier
                    .map(|value| to_sql_i64(value, "dart multiplier"))
                    .transpose()?,
                i64::from(record.event.score()),
                record.outcome,
                source,
                event_id,
                to_sql_i64(record.action_id, "dart action ID")?,
                created_at
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn record_simple_game_event(
    transaction: &Transaction<'_>,
    previous: &RuntimeSnapshot,
    event_type: &str,
    source: &str,
    frame_json: &str,
) -> Result<(), String> {
    let game_id = previous
        .session
        .state()
        .game_id
        .as_deref()
        .ok_or_else(|| format!("{event_type} has no game ID"))?;
    let player_id = previous.game.as_ref().and_then(|game| {
        let projection = game_projection(game);
        projection
            .players
            .get(projection.current_player_index)
            .map(|player| player.0.clone())
    });
    insert_game_event(
        transaction,
        GameEventInsert {
            game_id,
            event_type,
            player_id: player_id.as_deref(),
            source,
            payload_json: "{}",
            frame_json: Some(frame_json),
            corrects_event_id: None,
        },
    )?;
    Ok(())
}

fn record_undo(
    transaction: &Transaction<'_>,
    previous: &RuntimeSnapshot,
    frame_json: &str,
) -> Result<(), String> {
    let Some(game_id) = previous.session.state().game_id.as_deref() else {
        return Ok(());
    };
    let target = transaction
        .query_row(
            "SELECT id, event_type FROM game_events
             WHERE game_id=?1 AND effective=1
               AND event_type IN ('throw', 'continue_turn', 'next_player', 'game_action')
             ORDER BY ordinal DESC LIMIT 1",
            [game_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some((target_id, event_type)) = target {
        transaction
            .execute(
                "UPDATE game_events SET effective=0 WHERE id=?1",
                [target_id],
            )
            .map_err(|error| error.to_string())?;
        if event_type == "throw" {
            transaction
                .execute("DELETE FROM throws WHERE event_id=?1", [target_id])
                .map_err(|error| error.to_string())?;
        }
        let payload = serde_json::json!({"target_event_id": target_id}).to_string();
        insert_game_event(
            transaction,
            GameEventInsert {
                game_id,
                event_type: "undo",
                player_id: None,
                source: "control",
                payload_json: &payload,
                frame_json: Some(frame_json),
                corrects_event_id: Some(target_id),
            },
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct GameEventInsert<'a> {
    game_id: &'a str,
    event_type: &'a str,
    player_id: Option<&'a str>,
    source: &'a str,
    payload_json: &'a str,
    frame_json: Option<&'a str>,
    corrects_event_id: Option<i64>,
}

fn insert_game_event(
    transaction: &Transaction<'_>,
    event: GameEventInsert<'_>,
) -> Result<i64, String> {
    let ordinal: i64 = transaction
        .query_row(
            "SELECT COALESCE(MAX(ordinal), 0) + 1 FROM game_events WHERE game_id=?1",
            [event.game_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO game_events(
               game_id, ordinal, event_type, player_id, source, payload_json,
               frame_json, corrects_event_id
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.game_id,
                ordinal,
                event.event_type,
                event.player_id,
                event.source,
                event.payload_json,
                event.frame_json,
                event.corrects_event_id
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(transaction.last_insert_rowid())
}

fn finish_game(
    transaction: &Transaction<'_>,
    snapshot: &RuntimeSnapshot,
    final_state_json: &str,
) -> Result<(), String> {
    let session = snapshot.session.state();
    let game_id = session
        .game_id
        .as_deref()
        .ok_or_else(|| "finished game has no game ID".to_string())?;
    let game = snapshot
        .game
        .as_ref()
        .map(game_projection)
        .ok_or_else(|| "finished game has no final state".to_string())?;
    let winner_id = (game.winner_ids.len() == 1).then(|| game.winner_ids[0].as_str());
    transaction
        .execute(
            "UPDATE games SET status='finished', winner_id=?1, result_type=?2,
               finish_reason='rules_complete', final_state_json=?3,
               ended_at=CURRENT_TIMESTAMP WHERE id=?4",
            params![winner_id, game.result_type, final_state_json, game_id],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute("DELETE FROM game_winners WHERE game_id=?1", [game_id])
        .map_err(|error| error.to_string())?;
    for winner in &game.winner_ids {
        transaction
            .execute(
                "INSERT INTO game_winners(game_id, player_id) VALUES(?1, ?2)",
                params![game_id, winner],
            )
            .map_err(|error| error.to_string())?;
    }
    for (player_id, score) in game.players {
        transaction
            .execute(
                "UPDATE game_players SET final_score=?1 WHERE game_id=?2 AND player_id=?3",
                params![score, game_id, player_id],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn to_sql_i64(value: u64, label: &str) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("{label} exceeds SQLite integer range"))
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

    fn load_pending_effects(&self, current_revision: u64) -> Result<Vec<PlatformEffect>, String> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT effect_id, committed_revision, effect_json FROM effect_outbox
                 WHERE status='pending' ORDER BY committed_revision, effect_id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        let mut effects = Vec::new();
        for row in rows {
            let (effect_id, revision, json) = row.map_err(|error| error.to_string())?;
            let effect: PlatformEffect =
                serde_json::from_str(&json).map_err(|error| error.to_string())?;
            let revision = u64::try_from(revision)
                .map_err(|_| "effect outbox contains a negative revision".to_owned())?;
            if !valid_effect_id(&effect_id)
                || effect.effect_id != effect_id
                || effect.revision != revision
            {
                return Err("effect outbox metadata is inconsistent".into());
            }
            if effect.delivery == EffectDelivery::Durable
                || (effect.delivery == EffectDelivery::Recoverable
                    && effect.revision == current_revision)
            {
                effects.push(effect);
            }
        }
        Ok(effects)
    }

    fn acknowledge_effect(&mut self, effect_id: &str) -> Result<bool, String> {
        self.connection
            .execute(
                "UPDATE effect_outbox SET status='delivered'
                 WHERE effect_id=?1 AND status='pending'",
                [effect_id],
            )
            .map(|changed| changed == 1)
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
        expire_stale_effects(&transaction, request.next_revision)?;
        for effect in request.effects {
            if !valid_effect_id(&effect.effect_id) || effect.revision != request.next_revision {
                return Err("effect metadata is invalid".into());
            }
            let effect_json = serde_json::to_string(effect).map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO effect_outbox(
                        effect_id, committed_revision, effect_json, status
                     ) VALUES(?1, ?2, ?3, 'pending')",
                    params![effect.effect_id, next_revision, effect_json],
                )
                .map_err(|error| error.to_string())?;
        }
        project_domain(&transaction, &request)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(CommitOutcome::Committed)
    }
}

fn expire_stale_effects(transaction: &Transaction<'_>, next_revision: u64) -> Result<(), String> {
    let mut statement = transaction
        .prepare(
            "SELECT effect_id, effect_json FROM effect_outbox
             WHERE status='pending' AND committed_revision < ?1",
        )
        .map_err(|error| error.to_string())?;
    let next_revision_sql = to_sql_i64(next_revision, "revision")?;
    let rows = statement
        .query_map([next_revision_sql], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;
    let mut expired = Vec::new();
    for row in rows {
        let (effect_id, json) = row.map_err(|error| error.to_string())?;
        let effect: PlatformEffect =
            serde_json::from_str(&json).map_err(|error| error.to_string())?;
        if effect.delivery != EffectDelivery::Durable {
            expired.push(effect_id);
        }
    }
    drop(statement);
    for effect_id in expired {
        transaction
            .execute(
                "UPDATE effect_outbox SET status='discarded'
                 WHERE effect_id=?1 AND status='pending'",
                [effect_id],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn valid_effect_id(effect_id: &str) -> bool {
    !effect_id.is_empty()
        && effect_id.len() <= 256
        && effect_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdb_contracts::{DartEvent, EffectTarget, PlayerRef, Ring, SoundOutput};
    use sdb_game_core::{GameStatus, seed_from_id};
    use sdb_runtime::{Runtime, RuntimeAction};
    use sdb_session_core::Screen;

    fn portable_x01_archive(finish_game: bool) -> serde_json::Value {
        let repository = SqliteRepository::in_memory().expect("source repository");
        let mut runtime = Runtime::restore("portable-source", repository).expect("runtime");
        for (command_id, action) in [
            (
                "session",
                RuntimeAction::StartSession {
                    session_id: "portable-session".into(),
                    players: vec![PlayerRef {
                        id: "portable-ada".into(),
                        name: "Ada".into(),
                        avatar: "🦊".into(),
                        color: "#ff00aa".into(),
                        team_id: None,
                    }],
                    teams: Vec::new(),
                },
            ),
            (
                "prepare",
                RuntimeAction::PrepareGame {
                    game_type: "x01".into(),
                    options: serde_json::json!({"start_score": 40, "out_rule": "double"}),
                },
            ),
            (
                "start",
                RuntimeAction::StartPreparedGame {
                    game_id: "portable-game".into(),
                },
            ),
            ("playing", RuntimeAction::MarkGamePlaying),
        ] {
            runtime
                .dispatch("portable-source", command_id, None, action)
                .unwrap_or_else(|error| panic!("{command_id}: {error}"));
        }
        if finish_game {
            runtime
                .dispatch(
                    "portable-source",
                    "checkout",
                    None,
                    RuntimeAction::Dart {
                        event: DartEvent::Hit {
                            seq: 1,
                            field: 20,
                            ring: Ring::Double,
                            multiplier: 2,
                            label: "D20".into(),
                            score: 40,
                        },
                        source: DartSource::Board,
                    },
                )
                .expect("checkout");
        }
        runtime
            .into_repository()
            .export_data()
            .expect("portable archive")
    }

    #[test]
    fn portable_archive_import_recomputes_history_and_is_atomic_on_collision() {
        let archive = portable_x01_archive(true);
        let mut repository = SqliteRepository::in_memory().expect("target repository");
        repository
            .connection
            .execute(
                "INSERT INTO players(id, name, avatar, color, created_at)
                 VALUES('portable-ada', 'Ada', '🦊', '#ff00aa', 'different-timestamp')",
                [],
            )
            .expect("preexisting equivalent profile");

        let summary = repository.import_data(archive.clone()).expect("import");
        assert_eq!(summary.players_added, 0);
        assert_eq!(summary.players_reused, 1);
        assert_eq!(summary.sessions_added, 1);
        assert_eq!(summary.games_added, 1);
        assert_eq!(summary.throws_added, 1);
        assert_eq!(summary.events_added, 1);
        assert_eq!(summary.interrupted_sessions, 1);
        assert_eq!(summary.interrupted_games, 0);
        let session = repository
            .session_detail("portable-session")
            .expect("session")
            .expect("imported session");
        assert_eq!(session.session.status, "interrupted");
        assert_eq!(session.statistics[0].wins, 1);
        assert_eq!(session.statistics[0].darts, 1);
        assert_eq!(session.statistics[0].total_points, 40);
        let game = repository
            .game_detail("portable-game")
            .expect("game")
            .expect("imported game");
        assert_eq!(game.game.status, "finished");
        assert_eq!(game.game.winner_ids, ["portable-ada"]);
        assert_eq!(game.throws[0].event_id, Some(game.events[0].id));

        let mut colliding_archive = archive;
        colliding_archive["players"]
            .as_array_mut()
            .expect("players")
            .push(serde_json::json!({
                "id": "must-roll-back",
                "name": "Rollback",
                "avatar": "🎯",
                "color": "#00ffaa",
                "created_at": "2026-08-02T12:00:00Z"
            }));
        let error = repository
            .import_data(colliding_archive)
            .expect_err("session collision must reject archive");
        assert!(error.to_string().contains("already exists"));
        assert!(
            repository
                .players()
                .expect("players after rollback")
                .iter()
                .all(|player| player.id != "must-roll-back")
        );
    }

    #[test]
    fn portable_archive_import_interrupts_unresumable_running_games() {
        let archive = portable_x01_archive(false);
        let mut repository = SqliteRepository::in_memory().expect("target repository");
        let summary = repository
            .import_data(archive)
            .expect("import running archive");
        assert_eq!(summary.interrupted_sessions, 1);
        assert_eq!(summary.interrupted_games, 1);
        let game = repository
            .game_detail("portable-game")
            .expect("game")
            .expect("imported game");
        assert_eq!(game.game.status, "interrupted");
        assert_eq!(game.game.finish_reason, "portable_archive_import");
        assert!(game.game.ended_at.is_some());
    }

    #[test]
    fn portable_archive_import_rejects_unknown_or_inconsistent_data() {
        let mut future = portable_x01_archive(true);
        future["schema_version"] = serde_json::json!(99);
        let mut repository = SqliteRepository::in_memory().expect("repository");
        assert!(repository.import_data(future).is_err());
        assert!(repository.players().expect("unchanged players").is_empty());

        let mut unknown = portable_x01_archive(true);
        unknown["players"][0]["unexpected"] = serde_json::json!(true);
        assert!(repository.import_data(unknown).is_err());
        assert!(
            repository
                .players()
                .expect("unknown fields rejected")
                .is_empty()
        );

        let mut inconsistent = portable_x01_archive(true);
        inconsistent["games"][0]["detail"]["game"]["darts"] = serde_json::json!(2);
        assert!(repository.import_data(inconsistent).is_err());
        assert!(repository.players().expect("still unchanged").is_empty());
    }

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
    #[allow(clippy::too_many_lines)] // One test keeps commit, crash recovery, ack and expiry in order.
    fn sqlite_effect_outbox_is_atomic_recoverable_and_acknowledgeable() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-effects-{}-{}.sqlite",
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
                    "sound-on",
                    None,
                    RuntimeAction::UpdateSoundSettings {
                        enabled: true,
                        output: SoundOutput::Both,
                    },
                )
                .expect("sound");
            runtime
                .dispatch(
                    "first",
                    "start",
                    None,
                    RuntimeAction::StartCountUp {
                        players: vec![("ada".into(), "Ada".into())],
                        rounds: 5,
                    },
                )
                .expect("game");
            runtime
                .dispatch(
                    "first",
                    "dart-1",
                    None,
                    RuntimeAction::Dart {
                        event: DartEvent::Hit {
                            seq: 1,
                            field: 20,
                            ring: Ring::Triple,
                            multiplier: 3,
                            label: "T20".into(),
                            score: 60,
                        },
                        source: DartSource::Board,
                    },
                )
                .expect("dart");
            assert_eq!(runtime.public_snapshot().effects.len(), 3);
        }
        let repository = SqliteRepository::open(&temporary).expect("reopen");
        let mut runtime = Runtime::restore("second", repository).expect("restore");
        let effects = runtime.public_snapshot().effects;
        assert_eq!(effects.len(), 2);
        let projector = effects
            .iter()
            .find(|effect| effect.target == EffectTarget::Projector)
            .expect("projector effect");
        assert!(
            runtime
                .acknowledge_effect(&projector.effect_id, EffectTarget::Projector)
                .expect("ack")
        );
        assert_eq!(runtime.public_snapshot().effects.len(), 1);
        let controller = runtime
            .public_snapshot()
            .effects
            .into_iter()
            .find(|effect| effect.target == EffectTarget::Controller)
            .expect("controller effect");
        assert!(
            runtime
                .acknowledge_effect(&controller.effect_id, EffectTarget::Controller)
                .expect("controller ack")
        );
        runtime
            .dispatch(
                "second",
                "newer",
                None,
                RuntimeAction::UpdateArtTheme {
                    theme: sdb_contracts::ArtTheme::Neon,
                },
            )
            .expect("expire visual effect");
        assert!(runtime.public_snapshot().effects.is_empty());
        let repository = runtime.into_repository();
        let statuses: (i64, i64) = repository
            .connection
            .query_row(
                "SELECT
                    SUM(CASE WHEN status='delivered' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status='discarded' THEN 1 ELSE 0 END)
                 FROM effect_outbox",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("effect statuses");
        assert_eq!(statuses, (2, 1));
        std::fs::remove_file(temporary).expect("remove database");
    }

    #[test]
    fn next_player_is_a_distinct_audit_event_and_recovers_the_partial_visit() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let players = vec![
            PlayerRef {
                id: "ada".into(),
                name: "Ada".into(),
                avatar: "comet".into(),
                color: "#28e7ff".into(),
                team_id: None,
            },
            PlayerRef {
                id: "bob".into(),
                name: "Bob".into(),
                avatar: "nova".into(),
                color: "#ffd166".into(),
                team_id: None,
            },
        ];
        for (command_id, action) in [
            (
                "session",
                RuntimeAction::StartSession {
                    session_id: "session-next-player".into(),
                    players,
                    teams: Vec::new(),
                },
            ),
            (
                "prepare",
                RuntimeAction::PrepareGame {
                    game_type: "countup".into(),
                    options: serde_json::json!({"rounds": 5}),
                },
            ),
            (
                "start",
                RuntimeAction::StartPreparedGame {
                    game_id: "game-next-player".into(),
                },
            ),
            ("playing", RuntimeAction::MarkGamePlaying),
        ] {
            runtime
                .dispatch("runtime", command_id, None, action)
                .unwrap_or_else(|error| panic!("{command_id}: {error}"));
        }
        runtime
            .dispatch(
                "runtime",
                "dart",
                None,
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Triple,
                        multiplier: 3,
                        label: "T20".into(),
                        score: 60,
                    },
                    source: sdb_contracts::DartSource::Board,
                },
            )
            .expect("partial visit");
        runtime
            .dispatch("runtime", "next", None, RuntimeAction::NextPlayer)
            .expect("next player");

        let repository = runtime.into_repository();
        let event_count: i64 = repository
            .connection
            .query_row(
                "SELECT COUNT(*) FROM game_events
                 WHERE game_id='game-next-player' AND event_type='next_player'",
                [],
                |row| row.get(0),
            )
            .expect("next-player event");
        assert_eq!(event_count, 1);
        let restored = Runtime::restore("restored", repository).expect("restore");
        let Some(sdb_runtime::RuntimeGame::CountUp(game)) = &restored.snapshot().game else {
            panic!("wrong restored game");
        };
        assert_eq!(game.state().current_player_index, 1);
        assert_eq!(game.state().players[0].score, 60);
        assert_eq!(game.state().darts_in_turn, 0);
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
                            team_id: None,
                        }],
                        teams: Vec::new(),
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
        assert_eq!(repository.schema_version().expect("version"), 6);
        assert!(repository.journal(10).expect("journal").is_empty());
        std::fs::remove_file(temporary).expect("remove test database");
    }

    #[test]
    fn python_schema_two_database_is_backed_up_and_imported_completely() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-python-import-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_dir_all(&temporary);
        std::fs::create_dir_all(&temporary).expect("temporary directory");
        let legacy = temporary.join("dartboard.db");
        let runtime = temporary.join("runtime.sqlite");
        {
            let connection = Connection::open(&legacy).expect("legacy database");
            connection
                .execute_batch(include_str!("../../../fixtures/databases/python-v2.sql"))
                .expect("legacy Python fixture");
        }

        let (repository, imported) =
            SqliteRepository::open_with_legacy_import(&runtime, &legacy).expect("import database");
        let imported = imported.expect("import result");
        assert_eq!(imported.source, legacy);
        assert_eq!(
            imported.backup.file_name().and_then(|name| name.to_str()),
            Some("dartboard.db.pre-rust-v6.bak")
        );
        assert_eq!(repository.schema_version().expect("version"), 6);
        let profiles = repository.players().expect("profiles");
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].avatar, "fox");
        let finished = repository
            .game_detail("game-finished")
            .expect("game detail")
            .expect("finished game");
        assert_eq!(finished.game.winner_ids, ["ada"]);
        assert_eq!(finished.throws[0].mode_points, -4);
        let interrupted = repository
            .game_detail("game-running")
            .expect("game detail")
            .expect("running game");
        assert_eq!(interrupted.game.status, "interrupted");
        assert_eq!(interrupted.game.finish_reason, "legacy_runtime_migration");
        let active_session = repository
            .session_detail("session-active")
            .expect("session detail")
            .expect("active session");
        assert_eq!(active_session.session.status, "interrupted");
        let snapshot: RuntimeSnapshot = serde_json::from_str(
            &repository
                .load_snapshot()
                .expect("runtime table")
                .expect("imported settings snapshot"),
        )
        .expect("runtime snapshot");
        assert_eq!(snapshot.settings.art_theme, ArtTheme::Neon);
        assert_eq!(snapshot.settings.ui_language, UiLanguage::En);
        assert!(snapshot.settings.sound.enabled);
        assert_eq!(snapshot.settings.sound.output, SoundOutput::Both);
        assert!((snapshot.settings.calibration.scale - 1.1).abs() < f64::EPSILON);
        drop(repository);

        let legacy_connection = Connection::open(&legacy).expect("legacy source");
        let legacy_game_status: String = legacy_connection
            .query_row(
                "SELECT status FROM games WHERE id='game-running'",
                [],
                |row| row.get(0),
            )
            .expect("legacy status");
        assert_eq!(legacy_game_status, "running");
        drop(legacy_connection);
        let backup_connection = Connection::open(&imported.backup).expect("migration backup");
        let backup_version: u32 = backup_connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("backup version");
        assert_eq!(backup_version, 2);
        drop(backup_connection);

        let (_, second_import) =
            SqliteRepository::open_with_legacy_import(&runtime, &legacy).expect("reopen runtime");
        assert!(second_import.is_none());
        std::fs::remove_dir_all(temporary).expect("remove test directory");
    }

    #[test]
    fn incompatible_legacy_database_is_rejected_before_any_copy_is_created() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-python-import-reject-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_dir_all(&temporary);
        std::fs::create_dir_all(&temporary).expect("temporary directory");
        let legacy = temporary.join("dartboard.db");
        let runtime = temporary.join("runtime.sqlite");
        let connection = Connection::open(&legacy).expect("incompatible database");
        connection
            .pragma_update(None, "user_version", 99)
            .expect("future version");
        drop(connection);

        let error = SqliteRepository::open_with_legacy_import(&runtime, &legacy)
            .expect_err("must reject incompatible source");
        assert!(error.to_string().contains("schema 99"));
        assert!(!runtime.exists());
        assert!(!temporary.join("dartboard.db.pre-rust-v6.bak").exists());
        let connection = Connection::open(&legacy).expect("unchanged source");
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("source version");
        assert_eq!(version, 99);
        drop(connection);
        std::fs::remove_dir_all(temporary).expect("remove test directory");
    }

    #[test]
    fn a_later_reimport_uses_the_current_source_and_retains_the_first_backup() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-python-reimport-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_dir_all(&temporary);
        std::fs::create_dir_all(&temporary).expect("temporary directory");
        let legacy = temporary.join("dartboard.db");
        let runtime = temporary.join("runtime.sqlite");
        let connection = Connection::open(&legacy).expect("legacy database");
        connection
            .execute_batch(include_str!("../../../fixtures/databases/python-v2.sql"))
            .expect("legacy Python fixture");
        drop(connection);

        let (repository, first) =
            SqliteRepository::open_with_legacy_import(&runtime, &legacy).expect("first import");
        drop(repository);
        let first = first.expect("first import result");
        std::fs::remove_file(&runtime).expect("archive first runtime outside this test");
        let connection = Connection::open(&legacy).expect("updated legacy database");
        connection
            .execute(
                "INSERT INTO players(id, name, avatar, color, created_at)
                 VALUES('cara', 'Cara', 'star', '#00ffaa', '2026-07-03T18:00:00Z')",
                [],
            )
            .expect("new Python profile");
        drop(connection);

        let (repository, second) =
            SqliteRepository::open_with_legacy_import(&runtime, &legacy).expect("second import");
        let second = second.expect("second import result");
        assert_eq!(repository.players().expect("current profiles").len(), 3);
        assert!(first.backup.exists());
        assert_eq!(
            second.backup.file_name().and_then(|name| name.to_str()),
            Some("dartboard.db.pre-rust-v6.bak.1")
        );
        drop(repository);
        std::fs::remove_dir_all(temporary).expect("remove test directory");
    }

    #[test]
    fn schema_three_adds_dart_action_ids_without_losing_throws() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-action-id-migration-{}-{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&temporary);
        {
            let connection = Connection::open(&temporary).expect("schema three database");
            connection
                .execute_batch(
                    "
                    CREATE TABLE throws (
                        id INTEGER PRIMARY KEY,
                        game_id TEXT NOT NULL,
                        source TEXT NOT NULL,
                        event_id INTEGER,
                        created_at TEXT NOT NULL
                    );
                    INSERT INTO throws(id, game_id, source, event_id, created_at)
                    VALUES(1, 'legacy-game', 'board', 7, '2026-01-01T00:00:00Z');
                    PRAGMA user_version=3;
                    ",
                )
                .expect("schema three slice");
        }
        let repository = SqliteRepository::open(&temporary).expect("migrate action IDs");
        assert_eq!(repository.schema_version().expect("version"), 6);
        let row: (String, Option<i64>) = repository
            .connection
            .query_row(
                "SELECT game_id, action_id FROM throws WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("preserved throw");
        assert_eq!(row, ("legacy-game".into(), None));
        std::fs::remove_file(temporary).expect("remove test database");
    }

    #[test]
    fn schema_four_adds_companion_devices_without_losing_existing_data() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-companion-migration-{}-{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&temporary);
        {
            let connection = Connection::open(&temporary).expect("schema four database");
            connection
                .execute_batch(
                    "
                    CREATE TABLE migration_sentinel(value TEXT NOT NULL);
                    INSERT INTO migration_sentinel(value) VALUES('preserve-me');
                    PRAGMA user_version=4;
                    ",
                )
                .expect("schema four slice");
        }
        let repository = SqliteRepository::open(&temporary).expect("migrate companions");
        assert_eq!(repository.schema_version().expect("version"), 6);
        assert!(repository.companion_devices().expect("devices").is_empty());
        let sentinel: String = repository
            .connection
            .query_row("SELECT value FROM migration_sentinel", [], |row| row.get(0))
            .expect("preserved data");
        assert_eq!(sentinel, "preserve-me");
        std::fs::remove_file(temporary).expect("remove test database");
    }

    #[test]
    fn schema_five_adds_persisted_host_preferences() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-preference-migration-{}-{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&temporary);
        {
            let connection = Connection::open(&temporary).expect("schema five database");
            connection
                .execute_batch(
                    "
                    CREATE TABLE companion_devices (
                        device_id TEXT PRIMARY KEY,
                        device_name TEXT NOT NULL,
                        role TEXT NOT NULL,
                        token_hash TEXT NOT NULL,
                        paired_at_ms INTEGER NOT NULL,
                        revoked_at_ms INTEGER
                    );
                    INSERT INTO companion_devices(
                        device_id, device_name, role, token_hash, paired_at_ms
                    ) VALUES('ipad', 'Arcade iPad', 'projector',
                             'abababababababababababababababababababababababababababababababab', 42);
                    PRAGMA user_version=5;
                    ",
                )
                .expect("schema five slice");
        }
        let mut repository = SqliteRepository::open(&temporary).expect("migrate preferences");
        assert_eq!(repository.schema_version().expect("version"), 6);
        assert_eq!(repository.companion_devices().expect("companions").len(), 1);
        assert!(
            repository
                .preference("projector.output")
                .expect("empty")
                .is_none()
        );
        repository
            .save_preference("projector.output", "external_display")
            .expect("save preference");
        assert_eq!(
            repository
                .preference("projector.output")
                .expect("preference"),
            Some("external_display".into())
        );
        assert!(repository.preference("../invalid").is_err());
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
    #[allow(clippy::too_many_lines)] // One end-to-end projection proves all derived read models.
    fn finished_game_projects_profiles_history_and_statistics_atomically() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        runtime
            .dispatch(
                "runtime",
                "session",
                None,
                RuntimeAction::StartSession {
                    session_id: "session-1".into(),
                    players: vec![PlayerRef {
                        id: "ada".into(),
                        name: "Ada".into(),
                        avatar: "🦊".into(),
                        color: "#ff00aa".into(),
                        team_id: None,
                    }],
                    teams: Vec::new(),
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
        runtime
            .dispatch(
                "runtime",
                "checkout",
                None,
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Double,
                        multiplier: 2,
                        label: "D20".into(),
                        score: 40,
                    },
                    source: sdb_contracts::DartSource::Board,
                },
            )
            .expect("checkout");

        let repository = runtime.into_repository();
        let profiles = repository.players().expect("players");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "ada");
        assert_eq!(profiles[0].name, "Ada");
        assert_eq!(profiles[0].avatar, "🦊");
        assert_eq!(profiles[0].color, "#ff00aa");
        let sessions = repository.sessions(10).expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].games, 1);
        assert_eq!(sessions[0].finished_games, 1);
        assert_eq!(sessions[0].player_count, 1);
        let statistics = repository.player_statistics().expect("statistics");
        assert_eq!(statistics.len(), 1);
        assert_eq!(statistics[0].games, 1);
        assert_eq!(statistics[0].wins, 1);
        assert_eq!(statistics[0].darts, 1);
        assert_eq!(statistics[0].total_points, 40);
        assert_eq!(statistics[0].best_dart, 40);
        assert!((statistics[0].three_dart_average - 120.0).abs() < f64::EPSILON);
        assert!((statistics[0].win_rate - 100.0).abs() < f64::EPSILON);
        let session = repository
            .session_detail("session-1")
            .expect("session detail")
            .expect("stored session");
        assert_eq!(session.statistics[0].wins, 1);
        assert_eq!(session.games[0].darts, 1);
        assert_eq!(session.games[0].winner_count, 1);
        let game = repository
            .game_detail("game-1")
            .expect("game detail")
            .expect("stored game");
        assert_eq!(game.throws[0].field, Some(20));
        assert_eq!(game.throws[0].ring.as_deref(), Some("double"));
        assert_eq!(game.throws[0].dart_score, 40);
        let heatmap = repository
            .heatmap(Some("ada"), None, Some("x01"), false)
            .expect("heatmap");
        assert_eq!(heatmap.total_darts, 1);
        assert_eq!(heatmap.board_hits, 1);
        assert_eq!(heatmap.segments[0].successes, 1);
        let modes = repository.mode_statistics(false).expect("mode statistics");
        assert_eq!(modes[0].starts, 1);
        assert!((modes[0].completion_rate - 100.0).abs() < f64::EPSILON);
        let training = repository
            .training_recommendations("ada")
            .expect("training")
            .expect("known player");
        assert_eq!(training.recommendations.len(), 2);
        assert!(training.recommendations.iter().all(|item| item.starter));
        assert!(
            repository
                .training_recommendations("missing")
                .expect("unknown player query")
                .is_none()
        );
        let export = repository.export_data().expect("history export");
        assert_eq!(export["schema_version"], 2);
        assert_eq!(export["database_schema_version"], CURRENT_SCHEMA_VERSION);
        assert_eq!(export["sessions"].as_array().map(Vec::len), Some(1));
        assert_eq!(export["games"].as_array().map(Vec::len), Some(1));
        let effective_events: i64 = repository
            .connection
            .query_row(
                "SELECT COUNT(*) FROM game_events WHERE game_id='game-1' AND effective=1",
                [],
                |row| row.get(0),
            )
            .expect("events");
        assert_eq!(effective_events, 1);
        repository
            .connection
            .execute("UPDATE throws SET event_id=NULL WHERE game_id='game-1'", [])
            .expect("detach legacy throws from event journal");
        repository
            .connection
            .execute("DELETE FROM game_events WHERE game_id='game-1'", [])
            .expect("simulate legacy throw-only history");
        let legacy_replay = repository
            .game_replay("game-1")
            .expect("legacy replay")
            .expect("stored game");
        assert_eq!(legacy_replay.events.len(), 1);
        assert_eq!(
            legacy_replay.events[0].frame.as_ref().expect("frame")["players"][0]["score"],
            0
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Covers play, correction, scoring, audit and recovery atomically.
    fn registered_cricket_projects_history_and_recovers_like_legacy_modes() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        for (command_id, action) in [
            (
                "cricket-session",
                RuntimeAction::StartSession {
                    session_id: "session-cricket".into(),
                    players: vec![PlayerRef {
                        id: "ada".into(),
                        name: "Ada".into(),
                        avatar: "comet".into(),
                        color: "#28e7ff".into(),
                        team_id: None,
                    }],
                    teams: Vec::new(),
                },
            ),
            (
                "cricket-prepare",
                RuntimeAction::PrepareGame {
                    game_type: "cricket".into(),
                    options: serde_json::json!({}),
                },
            ),
            (
                "cricket-start",
                RuntimeAction::StartPreparedGame {
                    game_id: "game-cricket".into(),
                },
            ),
            ("cricket-playing", RuntimeAction::MarkGamePlaying),
        ] {
            runtime
                .dispatch("runtime", command_id, None, action)
                .unwrap_or_else(|error| panic!("{command_id}: {error}"));
        }

        let darts = [
            (20, Ring::Triple, 3, "T20", 60),
            (19, Ring::Triple, 3, "T19", 57),
            (18, Ring::Triple, 3, "T18", 54),
            (17, Ring::Triple, 3, "T17", 51),
            (16, Ring::Triple, 3, "T16", 48),
            (15, Ring::Triple, 3, "T15", 45),
            (25, Ring::DoubleBull, 2, "DBULL", 50),
            (25, Ring::SingleBull, 1, "BULL", 25),
        ];
        for (index, (field, ring, multiplier, label, score)) in darts.into_iter().enumerate() {
            if matches!(index, 3 | 6) {
                runtime
                    .dispatch(
                        "runtime",
                        &format!("cricket-continue-{index}"),
                        None,
                        RuntimeAction::Continue,
                    )
                    .expect("continue Cricket turn");
            }
            runtime
                .dispatch(
                    "runtime",
                    &format!("cricket-dart-{index}"),
                    None,
                    RuntimeAction::Dart {
                        event: DartEvent::Hit {
                            seq: u64::try_from(index + 1).expect("sequence"),
                            field,
                            ring,
                            multiplier,
                            label: label.into(),
                            score,
                        },
                        source: sdb_contracts::DartSource::Board,
                    },
                )
                .expect("Cricket dart");
        }

        assert_eq!(
            runtime.snapshot().session.state().screen,
            Screen::GameResult
        );
        assert_eq!(
            runtime.snapshot().session.state().standings[0].session_points,
            3
        );
        runtime
            .dispatch(
                "runtime",
                "cricket-correct-win-to-miss",
                None,
                RuntimeAction::CorrectDart {
                    action_id: 10,
                    replacement: DartEvent::Miss {
                        seq: 999,
                        label: "MISS".into(),
                        score: 0,
                    },
                    source: sdb_contracts::DartSource::ManualCorrection,
                },
            )
            .expect("reopen corrected Cricket win");
        assert_eq!(runtime.snapshot().session.state().screen, Screen::Playing);
        assert_eq!(
            runtime.snapshot().session.state().standings[0].session_points,
            0
        );
        runtime
            .dispatch(
                "runtime",
                "cricket-correct-miss-to-win",
                None,
                RuntimeAction::CorrectDart {
                    action_id: 10,
                    replacement: DartEvent::Hit {
                        seq: 1_000,
                        field: 25,
                        ring: Ring::SingleBull,
                        multiplier: 1,
                        label: "BULL".into(),
                        score: 25,
                    },
                    source: sdb_contracts::DartSource::ManualCorrection,
                },
            )
            .expect("restore corrected Cricket win");
        assert_eq!(
            runtime.snapshot().session.state().screen,
            Screen::GameResult
        );
        assert_eq!(
            runtime.snapshot().session.state().standings[0].session_points,
            3
        );
        let repository = runtime.into_repository();
        let detail = repository
            .game_detail("game-cricket")
            .expect("game detail")
            .expect("Cricket game");
        assert_eq!(detail.game.game_type, "cricket");
        assert_eq!(detail.game.status, "finished");
        assert_eq!(detail.game.winner_ids, vec!["ada"]);
        assert_eq!(detail.throws.len(), 8);
        assert_eq!(detail.throws[7].action_id, Some(10));
        assert_eq!(detail.throws[7].seq, 8);
        assert_eq!(detail.throws[7].source, "manual_correction");
        assert_eq!(detail.game.ruleset_version, 1);
        let restored = Runtime::restore("restored", repository).expect("restore Cricket runtime");
        let Some(RuntimeGame::Registered(game)) = restored.snapshot().game.as_ref() else {
            panic!("registered Cricket snapshot missing");
        };
        assert_eq!(game.state().status, GameStatus::Finished);
        assert_eq!(game.state().winner_ids, vec!["ada"]);
        assert_eq!(game.state().random_seed, seed_from_id("game-cricket"));
    }

    #[test]
    fn history_preserves_signed_arcade_scores() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        for (command_id, action) in [
            (
                "signed-session",
                RuntimeAction::StartSession {
                    session_id: "signed-session".into(),
                    players: vec![PlayerRef {
                        id: "ada".into(),
                        name: "Ada".into(),
                        avatar: "comet".into(),
                        color: "#28e7ff".into(),
                        team_id: None,
                    }],
                    teams: Vec::new(),
                },
            ),
            (
                "signed-prepare",
                RuntimeAction::PrepareGame {
                    game_type: "countup".into(),
                    options: serde_json::json!({"rounds": 3}),
                },
            ),
            (
                "signed-start",
                RuntimeAction::StartPreparedGame {
                    game_id: "signed-game".into(),
                },
            ),
            ("signed-playing", RuntimeAction::MarkGamePlaying),
        ] {
            runtime
                .dispatch("runtime", command_id, None, action)
                .unwrap_or_else(|error| panic!("{command_id}: {error}"));
        }
        runtime
            .dispatch(
                "runtime",
                "signed-dart",
                None,
                RuntimeAction::Dart {
                    event: DartEvent::Miss {
                        seq: 1,
                        label: "MISS".into(),
                        score: 0,
                    },
                    source: sdb_contracts::DartSource::Board,
                },
            )
            .expect("record dart");

        let repository = runtime.into_repository();
        repository
            .connection
            .execute(
                "UPDATE game_players SET final_score=-40 WHERE game_id='signed-game'",
                [],
            )
            .expect("set signed final score");
        repository
            .connection
            .execute(
                "UPDATE throws SET score_after=-40 WHERE game_id='signed-game'",
                [],
            )
            .expect("set signed throw score");

        let detail = repository
            .game_detail("signed-game")
            .expect("game detail")
            .expect("signed game");
        assert_eq!(detail.game.players[0].final_score, Some(-40));
        assert_eq!(detail.throws[0].score_after, -40);
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Verifies winner replacement across runtime and SQLite.
    fn count_up_correction_replaces_the_finished_winner_atomically() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let players = [("ada", "Ada"), ("bob", "Bob")]
            .into_iter()
            .map(|(id, name)| PlayerRef {
                id: id.into(),
                name: name.into(),
                avatar: "comet".into(),
                color: "#28e7ff".into(),
                team_id: None,
            })
            .collect();
        for (command_id, action) in [
            (
                "countup-session",
                RuntimeAction::StartSession {
                    session_id: "session-countup-edit".into(),
                    players,
                    teams: Vec::new(),
                },
            ),
            (
                "countup-prepare",
                RuntimeAction::PrepareGame {
                    game_type: "countup".into(),
                    options: serde_json::json!({"rounds": 1}),
                },
            ),
            (
                "countup-start",
                RuntimeAction::StartPreparedGame {
                    game_id: "game-countup-edit".into(),
                },
            ),
            ("countup-playing", RuntimeAction::MarkGamePlaying),
        ] {
            runtime
                .dispatch("runtime", command_id, None, action)
                .unwrap_or_else(|error| panic!("{command_id}: {error}"));
        }
        let darts = [
            DartEvent::Hit {
                seq: 1,
                field: 20,
                ring: Ring::Triple,
                multiplier: 3,
                label: "T20".into(),
                score: 60,
            },
            DartEvent::Miss {
                seq: 2,
                label: "MISS".into(),
                score: 0,
            },
            DartEvent::Miss {
                seq: 3,
                label: "MISS".into(),
                score: 0,
            },
            DartEvent::Hit {
                seq: 4,
                field: 20,
                ring: Ring::SingleInner,
                multiplier: 1,
                label: "S20".into(),
                score: 20,
            },
            DartEvent::Miss {
                seq: 5,
                label: "MISS".into(),
                score: 0,
            },
            DartEvent::Miss {
                seq: 6,
                label: "MISS".into(),
                score: 0,
            },
        ];
        for (index, event) in darts.into_iter().enumerate() {
            if index == 3 {
                runtime
                    .dispatch("runtime", "countup-continue", None, RuntimeAction::Continue)
                    .expect("continue CountUp");
            }
            runtime
                .dispatch(
                    "runtime",
                    &format!("countup-dart-{index}"),
                    None,
                    RuntimeAction::Dart {
                        event,
                        source: sdb_contracts::DartSource::Board,
                    },
                )
                .expect("CountUp dart");
        }
        assert_eq!(
            runtime.snapshot().session.state().standings[0].session_points,
            3
        );
        assert_eq!(
            runtime.snapshot().session.state().standings[1].session_points,
            0
        );

        runtime
            .dispatch(
                "runtime",
                "countup-correct-winner",
                None,
                RuntimeAction::CorrectDart {
                    action_id: 1,
                    replacement: DartEvent::Miss {
                        seq: 999,
                        label: "MISS".into(),
                        score: 0,
                    },
                    source: sdb_contracts::DartSource::ManualCorrection,
                },
            )
            .expect("correct CountUp winner");
        assert_eq!(
            runtime.snapshot().session.state().standings[0].session_points,
            0
        );
        assert_eq!(
            runtime.snapshot().session.state().standings[1].session_points,
            3
        );

        let repository = runtime.into_repository();
        let detail = repository
            .game_detail("game-countup-edit")
            .expect("detail")
            .expect("game");
        assert_eq!(detail.game.winner_ids, vec!["bob"]);
        assert_eq!(detail.throws.len(), 6);
        assert_eq!(detail.throws[0].action_id, Some(1));
        assert_eq!(detail.throws[0].seq, 1);
        assert_eq!(detail.throws[0].source, "manual_correction");
        assert_eq!(detail.throws[0].outcome, "miss");
    }

    #[test]
    fn undo_keeps_audit_event_but_removes_the_win_from_statistics() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let player = PlayerRef {
            id: "ada".into(),
            name: "Ada".into(),
            avatar: "comet".into(),
            color: "#28e7ff".into(),
            team_id: None,
        };
        for (command_id, action) in [
            (
                "session",
                RuntimeAction::StartSession {
                    session_id: "session-undo".into(),
                    players: vec![player],
                    teams: Vec::new(),
                },
            ),
            (
                "prepare",
                RuntimeAction::PrepareGame {
                    game_type: "x01".into(),
                    options: serde_json::json!({"start_score": 40, "out_rule": "double"}),
                },
            ),
            (
                "start",
                RuntimeAction::StartPreparedGame {
                    game_id: "game-undo".into(),
                },
            ),
            ("playing", RuntimeAction::MarkGamePlaying),
            (
                "checkout",
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Double,
                        multiplier: 2,
                        label: "D20".into(),
                        score: 40,
                    },
                    source: sdb_contracts::DartSource::Board,
                },
            ),
            ("undo", RuntimeAction::Undo),
        ] {
            runtime
                .dispatch("runtime", command_id, None, action)
                .unwrap_or_else(|error| panic!("{command_id}: {error}"));
        }
        let repository = runtime.into_repository();
        let statistics = repository.player_statistics().expect("statistics");
        assert_eq!(statistics[0].games, 0);
        assert_eq!(statistics[0].wins, 0);
        assert_eq!(statistics[0].darts, 0);
        let rows: (i64, i64, i64) = repository
            .connection
            .query_row(
                "SELECT
                   COUNT(*),
                   SUM(CASE WHEN event_type='throw' AND effective=0 THEN 1 ELSE 0 END),
                   SUM(CASE WHEN event_type='undo' AND corrects_event_id IS NOT NULL THEN 1 ELSE 0 END)
                 FROM game_events WHERE game_id='game-undo'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("audit events");
        assert_eq!(rows, (2, 1, 1));
        let game_status: String = repository
            .connection
            .query_row("SELECT status FROM games WHERE id='game-undo'", [], |row| {
                row.get(0)
            })
            .expect("game status");
        assert_eq!(game_status, "running");
    }

    #[test]
    #[allow(clippy::too_many_lines)] // End-to-end audit-chain scenario.
    fn correction_and_deletion_rewrite_throws_but_preserve_the_audit_chain() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        for (command_id, action) in [
            (
                "session",
                RuntimeAction::StartSession {
                    session_id: "edit-session".into(),
                    players: vec![PlayerRef {
                        id: "ada".into(),
                        name: "Ada".into(),
                        avatar: "comet".into(),
                        color: "#28e7ff".into(),
                        team_id: None,
                    }],
                    teams: Vec::new(),
                },
            ),
            (
                "prepare",
                RuntimeAction::PrepareGame {
                    game_type: "x01".into(),
                    options: serde_json::json!({"start_score": 301, "out_rule": "straight"}),
                },
            ),
            (
                "start",
                RuntimeAction::StartPreparedGame {
                    game_id: "edit-game".into(),
                },
            ),
            ("playing", RuntimeAction::MarkGamePlaying),
            (
                "dart-1",
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::SingleInner,
                        multiplier: 1,
                        label: "S20".into(),
                        score: 20,
                    },
                    source: DartSource::Board,
                },
            ),
            (
                "dart-2",
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 2,
                        field: 20,
                        ring: Ring::SingleInner,
                        multiplier: 1,
                        label: "S20".into(),
                        score: 20,
                    },
                    source: DartSource::Board,
                },
            ),
            (
                "correct-1",
                RuntimeAction::CorrectDart {
                    action_id: 1,
                    replacement: DartEvent::Hit {
                        seq: 999,
                        field: 20,
                        ring: Ring::Triple,
                        multiplier: 3,
                        label: "T20".into(),
                        score: 60,
                    },
                    source: DartSource::ManualCorrection,
                },
            ),
            ("delete-2", RuntimeAction::DeleteDart { action_id: 2 }),
        ] {
            runtime
                .dispatch("runtime", command_id, None, action)
                .unwrap_or_else(|error| panic!("{command_id}: {error}"));
        }
        let RuntimeGame::X01(game) = runtime.snapshot().game.as_ref().expect("game") else {
            panic!("wrong game");
        };
        assert_eq!(game.state().players[0].score, 241);
        assert_eq!(game.state().darts_in_turn, 1);
        let repository = runtime.into_repository();
        let throws: Vec<(i64, i64, String, i64, String)> = {
            let mut statement = repository
                .connection
                .prepare(
                    "SELECT action_id, seq, json_extract(event_json, '$.label'),
                            score_after, source
                     FROM throws WHERE game_id='edit-game' ORDER BY id",
                )
                .expect("throws query");
            statement
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })
                .expect("throws")
                .collect::<Result<_, _>>()
                .expect("throw rows")
        };
        assert_eq!(
            throws,
            vec![(1, 1, "T20".into(), 241, "manual_correction".into())]
        );
        let audit: (i64, i64, i64, i64) = repository
            .connection
            .query_row(
                "SELECT
                   COUNT(*),
                   SUM(CASE WHEN event_type='throw' AND effective=0 THEN 1 ELSE 0 END),
                   SUM(CASE WHEN event_type='throw_corrected' AND effective=1
                             AND corrects_event_id IS NOT NULL THEN 1 ELSE 0 END),
                   SUM(CASE WHEN event_type='throw_deleted' AND effective=1
                             AND corrects_event_id IS NOT NULL THEN 1 ELSE 0 END)
                 FROM game_events WHERE game_id='edit-game'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("audit");
        assert_eq!(audit, (4, 2, 1, 1));
        let session = repository
            .session_detail("edit-session")
            .expect("session detail")
            .expect("session");
        assert_eq!(session.players[0].id, "ada");
        assert_eq!(session.games.len(), 1);
        assert_eq!(session.games[0].id, "edit-game");
        let detail = repository
            .game_detail("edit-game")
            .expect("game detail")
            .expect("game");
        assert_eq!(detail.game.players[0].id, "ada");
        assert_eq!(detail.throws.len(), 1);
        assert_eq!(detail.throws[0].action_id, Some(1));
        assert_eq!(detail.throws[0].event["label"], "T20");
        assert_eq!(detail.events.len(), 4);
        assert_eq!(
            detail.events.iter().filter(|event| event.effective).count(),
            2
        );
        let replay = repository
            .game_replay("edit-game")
            .expect("replay")
            .expect("game");
        assert!(replay.initial_state.is_some());
        assert!(replay.final_state.is_none());
        assert_eq!(replay.events.len(), 4);
        assert!(
            repository
                .session_detail("missing")
                .expect("missing")
                .is_none()
        );
        assert!(
            repository
                .game_detail("missing")
                .expect("missing")
                .is_none()
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // The complete test/nonproduction projection is one atomic flow.
    fn projector_test_throw_marks_the_whole_game_nonproduction() {
        let repository = SqliteRepository::in_memory().expect("repository");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        for (command_id, action) in [
            (
                "session",
                RuntimeAction::StartSession {
                    session_id: "test-session".into(),
                    players: vec![PlayerRef {
                        id: "ada".into(),
                        name: "Ada".into(),
                        avatar: "comet".into(),
                        color: "#28e7ff".into(),
                        team_id: None,
                    }],
                    teams: Vec::new(),
                },
            ),
            (
                "prepare",
                RuntimeAction::PrepareGame {
                    game_type: "x01".into(),
                    options: serde_json::json!({"start_score": 40, "out_rule": "double"}),
                },
            ),
            (
                "start",
                RuntimeAction::StartPreparedGame {
                    game_id: "test-game".into(),
                },
            ),
            ("playing", RuntimeAction::MarkGamePlaying),
            (
                "test-checkout",
                RuntimeAction::Dart {
                    event: DartEvent::Hit {
                        seq: 1,
                        field: 20,
                        ring: Ring::Double,
                        multiplier: 2,
                        label: "D20".into(),
                        score: 40,
                    },
                    source: sdb_contracts::DartSource::ProjectorTest,
                },
            ),
        ] {
            runtime
                .dispatch("runtime", command_id, None, action)
                .unwrap_or_else(|error| panic!("{command_id}: {error}"));
        }
        let repository = runtime.into_repository();
        let environment: String = repository
            .connection
            .query_row(
                "SELECT environment FROM games WHERE id='test-game'",
                [],
                |row| row.get(0),
            )
            .expect("environment");
        assert_eq!(environment, "test");
        let source: String = repository
            .connection
            .query_row(
                "SELECT source FROM throws WHERE game_id='test-game'",
                [],
                |row| row.get(0),
            )
            .expect("source");
        assert_eq!(source, "projector_test");
        let statistics = repository.player_statistics().expect("statistics");
        assert_eq!(statistics[0].games, 0);
        assert_eq!(statistics[0].darts, 0);
        assert_eq!(statistics[0].wins, 0);
        let with_test = repository
            .player_statistics_including_test(true)
            .expect("statistics with test games");
        assert_eq!(with_test[0].games, 1);
        assert_eq!(with_test[0].darts, 1);
        assert_eq!(
            repository
                .heatmap(None, None, None, false)
                .expect("production heatmap")
                .total_darts,
            0
        );
        assert_eq!(
            repository
                .heatmap(None, None, None, true)
                .expect("test heatmap")
                .total_darts,
            1
        );
        assert!(
            repository
                .mode_statistics(false)
                .expect("production modes")
                .is_empty()
        );
        assert_eq!(
            repository.mode_statistics(true).expect("test modes")[0].finished,
            1
        );
    }

    #[test]
    fn companion_grants_persist_as_hashes_and_remain_revocable() {
        let mut repository = SqliteRepository::in_memory().expect("repository");
        let device = PairedDevice {
            device_id: "ipad-projector".into(),
            device_name: "Arcade iPad".into(),
            role: CompanionRole::Projector,
            token_hash: "ab".repeat(32),
            paired_at_ms: 42,
        };
        repository
            .save_companion_device(&device)
            .expect("save companion");
        assert_eq!(
            repository.companion_devices().expect("devices"),
            vec![device]
        );
        assert!(
            repository
                .revoke_companion_device("ipad-projector", 99)
                .expect("revoke")
        );
        assert!(repository.companion_devices().expect("devices").is_empty());
        assert!(
            !repository
                .revoke_companion_device("ipad-projector", 100)
                .expect("already revoked")
        );
        let stored: (String, i64) = repository
            .connection
            .query_row(
                "SELECT token_hash, revoked_at_ms FROM companion_devices
                 WHERE device_id='ipad-projector'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("audit row");
        assert_eq!(stored, ("ab".repeat(32), 99));
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

    #[test]
    fn failed_history_projection_rolls_back_runtime_and_audit_journal() {
        let repository = SqliteRepository::in_memory().expect("repository");
        repository
            .connection
            .execute_batch(
                "CREATE TRIGGER fail_player_projection
                 BEFORE INSERT ON players
                 BEGIN
                   SELECT RAISE(ABORT, 'injected profile failure');
                 END;",
            )
            .expect("failure trigger");
        let mut runtime = Runtime::restore("runtime", repository).expect("runtime");
        let error = runtime
            .dispatch(
                "runtime",
                "session",
                Some(0),
                RuntimeAction::StartSession {
                    session_id: "failed-session".into(),
                    players: vec![PlayerRef {
                        id: "ada".into(),
                        name: "Ada".into(),
                        avatar: "comet".into(),
                        color: "#28e7ff".into(),
                        team_id: None,
                    }],
                    teams: Vec::new(),
                },
            )
            .expect_err("projection must fail");
        assert!(matches!(error, sdb_runtime::RuntimeError::Persistence(_)));
        assert_eq!(runtime.snapshot().revision, 0);
        let repository = runtime.into_repository();
        assert!(repository.load_snapshot().expect("snapshot").is_none());
        assert!(repository.journal(10).expect("journal").is_empty());
        assert!(repository.players().expect("players").is_empty());
        assert!(repository.sessions(10).expect("sessions").is_empty());
    }
}
