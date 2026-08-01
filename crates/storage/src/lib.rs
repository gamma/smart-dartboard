//! Transactional `SQLite` implementation of the runtime repository.

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sdb_companion::{CompanionRole, PairedDevice};
use sdb_contracts::{DartEvent, DartSource};
use sdb_runtime::{
    CommitOutcome, CommitRequest, Repository, RuntimeAction, RuntimeGame, RuntimeSnapshot,
};
use sdb_session_core::{Screen, SessionStatus};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use thiserror::Error;

const CURRENT_SCHEMA_VERSION: u32 = 5;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
pub struct PlayerStatistics {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub games: u64,
    pub wins: u64,
    pub darts: u64,
    pub total_points: u64,
    pub best_dart: u64,
    pub misses: u64,
    pub three_dart_average: f64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamePlayerHistory {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub position: u64,
    pub final_score: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub initial_state: Option<serde_json::Value>,
    pub final_state: Option<serde_json::Value>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub players: Vec<GamePlayerHistory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDetail {
    pub session: SessionHistory,
    pub players: Vec<PlayerProfile>,
    pub games: Vec<GameHistory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrowHistory {
    pub id: u64,
    pub action_id: Option<u64>,
    pub seq: u64,
    pub player_id: Option<String>,
    pub event: serde_json::Value,
    pub score_after: u64,
    pub round_number: u64,
    pub dart_in_turn: u64,
    pub outcome: String,
    pub source: String,
    pub event_id: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
pub struct GameDetail {
    pub game: GameHistory,
    pub throws: Vec<ThrowHistory>,
    pub events: Vec<GameEventHistory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameReplay {
    pub game_id: String,
    pub initial_state: Option<serde_json::Value>,
    pub final_state: Option<serde_json::Value>,
    pub events: Vec<GameEventHistory>,
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
        let mut statement = self.connection.prepare(
            "SELECT p.id, p.name, p.avatar, p.color,
                    COUNT(DISTINCT g.id) AS games,
                    COUNT(DISTINCT CASE WHEN gw.player_id IS NOT NULL THEN g.id END) AS wins,
                    COUNT(t.id) AS darts, COALESCE(SUM(t.dart_score), 0) AS total_points,
                    COALESCE(MAX(t.dart_score), 0) AS best_dart,
                    COALESCE(SUM(CASE WHEN t.outcome='miss' THEN 1 ELSE 0 END), 0) AS misses
             FROM players p
             LEFT JOIN game_players gp ON gp.player_id=p.id
             LEFT JOIN games g ON g.id=gp.game_id
                 AND g.status='finished' AND g.environment='production'
             LEFT JOIN throws t ON t.game_id=g.id AND t.player_id=p.id
             LEFT JOIN game_winners gw ON gw.game_id=g.id AND gw.player_id=p.id
             GROUP BY p.id
             ORDER BY wins DESC, total_points DESC, lower(p.name), p.id",
        )?;
        let rows = statement.query_map([], |row| {
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
            ))
        })?;
        rows.map(|row| {
            let (id, name, avatar, color, games, wins, darts, points, best, misses) = row?;
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
                best_dart: nonnegative(best, "statistics best dart")?,
                misses: nonnegative(misses, "statistics misses")?,
                three_dart_average,
                win_rate,
            })
        })
        .collect()
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
        Ok(Some(GameReplay {
            game_id: game.id,
            initial_state: game.initial_state,
            final_state: game.final_state,
            events: load_game_events(&self.connection, game_id)?,
        }))
    }
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
                    initial_state_json, final_state_json, started_at, ended_at
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
            final_score: final_score
                .map(|score| nonnegative(score, "game final score"))
                .transpose()?,
        })
    })
    .collect()
}

#[allow(clippy::type_complexity)]
fn load_throws(connection: &Connection, game_id: &str) -> Result<Vec<ThrowHistory>, StorageError> {
    let mut statement = connection.prepare(
        "SELECT id, action_id, seq, player_id, event_json, score_after,
                round_number, dart_in_turn, outcome, source, event_id, created_at
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
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<i64>>(10)?,
            row.get::<_, String>(11)?,
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
            outcome,
            source,
            event_id,
            created_at,
        ) = row?;
        Ok(ThrowHistory {
            id: nonnegative(id, "throw ID")?,
            action_id: optional_nonnegative(action_id, "dart action ID")?,
            seq: nonnegative(seq, "dart sequence")?,
            player_id,
            event: parse_json(&event, "dart event")?,
            score_after: nonnegative(score, "score after dart")?,
            round_number: nonnegative(round, "dart round")?,
            dart_in_turn: nonnegative(dart, "dart in turn")?,
            outcome,
            source,
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

#[derive(Debug)]
struct GameProjection {
    players: Vec<(String, u32)>,
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
        RuntimeGame::X01(game) => {
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
                last_bust: state.last_bust,
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
        RuntimeAction::StartSession {
            session_id,
            players,
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
        | RuntimeAction::MarkGamePlaying
        | RuntimeAction::SelectStarter { .. }
        | RuntimeAction::NextGame
        | RuntimeAction::StartCountUp { .. }
        | RuntimeAction::StartX01 { .. } => {}
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
        Some(RuntimeGame::X01(game)) => game.dart_records().last().map(|record| record.action_id),
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
                i64::from(score_after),
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
    rewrite_x01_throws(
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

fn rewrite_x01_throws(
    transaction: &Transaction<'_>,
    snapshot: &RuntimeSnapshot,
    game_id: &str,
    edited_action_id: u64,
    edit: Option<(&str, i64)>,
) -> Result<(), String> {
    let records = match snapshot.game.as_ref() {
        Some(RuntimeGame::X01(game)) => game.dart_records(),
        _ => return Err("dart edits are only supported for X01".into()),
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
    record: &sdb_game_core::X01DartRecord,
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
                i64::from(record.score_after),
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
                params![i64::from(score), game_id, player_id],
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
        project_domain(&transaction, &request)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(CommitOutcome::Committed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdb_contracts::{DartEvent, PlayerRef, Ring};
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
        assert_eq!(repository.schema_version().expect("version"), 5);
        assert!(repository.journal(10).expect("journal").is_empty());
        std::fs::remove_file(temporary).expect("remove test database");
    }

    #[test]
    fn python_schema_two_database_is_extended_without_losing_profiles() {
        let temporary = std::env::temp_dir().join(format!(
            "sdb-python-migration-{}-{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&temporary);
        {
            let connection = Connection::open(&temporary).expect("legacy database");
            connection
                .execute_batch(
                    "
                    CREATE TABLE players (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL,
                        avatar TEXT NOT NULL DEFAULT 'comet',
                        color TEXT NOT NULL DEFAULT '#28e7ff',
                        created_at TEXT NOT NULL
                    );
                    INSERT INTO players(id, name, avatar, color, created_at)
                    VALUES('legacy-ada', 'Ada', 'comet', '#28e7ff', '2026-01-01T00:00:00Z');
                    PRAGMA user_version=2;
                    ",
                )
                .expect("legacy Python schema slice");
        }
        let repository = SqliteRepository::open(&temporary).expect("migrate Python database");
        assert_eq!(repository.schema_version().expect("version"), 5);
        let profiles = repository.players().expect("profiles");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "legacy-ada");
        assert!(repository.load_snapshot().expect("runtime table").is_none());
        std::fs::remove_file(temporary).expect("remove test database");
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
        assert_eq!(repository.schema_version().expect("version"), 5);
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
        assert_eq!(repository.schema_version().expect("version"), 5);
        assert!(repository.companion_devices().expect("devices").is_empty());
        let sentinel: String = repository
            .connection
            .query_row("SELECT value FROM migration_sentinel", [], |row| row.get(0))
            .expect("preserved data");
        assert_eq!(sentinel, "preserve-me");
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
                    }],
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
        let effective_events: i64 = repository
            .connection
            .query_row(
                "SELECT COUNT(*) FROM game_events WHERE game_id='game-1' AND effective=1",
                [],
                |row| row.get(0),
            )
            .expect("events");
        assert_eq!(effective_events, 1);
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
        };
        for (command_id, action) in [
            (
                "session",
                RuntimeAction::StartSession {
                    session_id: "session-undo".into(),
                    players: vec![player],
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
                    }],
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
                    }],
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
                    }],
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
