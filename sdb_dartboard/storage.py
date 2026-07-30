from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from threading import RLock
from typing import Any, Dict, Iterable, List, Optional
from uuid import uuid4


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


class DartboardStore:
    """Small durable event store for kiosk operation.

    SQLite keeps deployment self-contained while transactions make every
    completed throw and session transition recoverable after a restart.
    """

    def __init__(self, path: Path | str) -> None:
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = RLock()
        self._connection = sqlite3.connect(self.path, check_same_thread=False)
        self._connection.row_factory = sqlite3.Row
        self._connection.execute("PRAGMA journal_mode=WAL")
        self._connection.execute("PRAGMA foreign_keys=ON")
        self._migrate()

    def close(self) -> None:
        with self._lock:
            self._connection.close()

    def ping(self) -> bool:
        with self._lock:
            return self._connection.execute("SELECT 1").fetchone()[0] == 1

    def _migrate(self) -> None:
        with self._lock, self._connection:
            self._connection.executescript(
                """
                CREATE TABLE IF NOT EXISTS players (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    avatar TEXT NOT NULL DEFAULT 'comet',
                    color TEXT NOT NULL DEFAULT '#28e7ff',
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    status TEXT NOT NULL,
                    language TEXT NOT NULL DEFAULT 'de',
                    started_at TEXT NOT NULL,
                    ended_at TEXT
                );
                CREATE TABLE IF NOT EXISTS session_players (
                    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                    player_id TEXT NOT NULL REFERENCES players(id),
                    position INTEGER NOT NULL,
                    PRIMARY KEY (session_id, player_id)
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
                    started_at TEXT NOT NULL,
                    ended_at TEXT
                );
                CREATE TABLE IF NOT EXISTS throws (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                    seq INTEGER NOT NULL,
                    player_id TEXT REFERENCES players(id),
                    event_json TEXT NOT NULL,
                    score_after INTEGER NOT NULL,
                    round_number INTEGER NOT NULL DEFAULT 1,
                    dart_in_turn INTEGER NOT NULL DEFAULT 1,
                    field INTEGER,
                    ring TEXT,
                    multiplier INTEGER,
                    dart_score INTEGER NOT NULL DEFAULT 0,
                    mode_points INTEGER NOT NULL DEFAULT 0,
                    outcome TEXT NOT NULL DEFAULT 'neutral',
                    source TEXT NOT NULL DEFAULT 'unknown',
                    task_json TEXT,
                    event_id INTEGER,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_games_session ON games(session_id);
                CREATE INDEX IF NOT EXISTS idx_throws_game ON throws(game_id);
                CREATE INDEX IF NOT EXISTS idx_throws_player ON throws(player_id);
                CREATE TABLE IF NOT EXISTS game_winners (
                    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                    player_id TEXT NOT NULL REFERENCES players(id),
                    PRIMARY KEY (game_id, player_id)
                );
                CREATE INDEX IF NOT EXISTS idx_game_winners_player
                    ON game_winners(player_id);
                CREATE TABLE IF NOT EXISTS game_players (
                    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
                    player_id TEXT NOT NULL REFERENCES players(id),
                    position INTEGER NOT NULL,
                    final_score INTEGER,
                    PRIMARY KEY (game_id, player_id)
                );
                CREATE INDEX IF NOT EXISTS idx_game_players_player
                    ON game_players(player_id);
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
                    created_at TEXT NOT NULL,
                    UNIQUE(game_id, ordinal)
                );
                CREATE INDEX IF NOT EXISTS idx_game_events_game
                    ON game_events(game_id, ordinal);
                CREATE INDEX IF NOT EXISTS idx_game_events_player
                    ON game_events(player_id);
                CREATE TABLE IF NOT EXISTS runtime_state (
                    key TEXT PRIMARY KEY,
                    value_json TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                """
            )
            self._ensure_column("sessions", "language", "TEXT NOT NULL DEFAULT 'de'")
            self._ensure_column("games", "result_type", "TEXT NOT NULL DEFAULT ''")
            self._ensure_column("games", "finish_reason", "TEXT NOT NULL DEFAULT ''")
            self._ensure_column("games", "ruleset_version", "INTEGER NOT NULL DEFAULT 1")
            self._ensure_column("games", "app_version", "TEXT NOT NULL DEFAULT ''")
            self._ensure_column("games", "environment", "TEXT NOT NULL DEFAULT 'production'")
            self._ensure_column("games", "initial_state_json", "TEXT")
            self._ensure_column("games", "final_state_json", "TEXT")
            self._ensure_column("throws", "round_number", "INTEGER NOT NULL DEFAULT 1")
            self._ensure_column("throws", "dart_in_turn", "INTEGER NOT NULL DEFAULT 1")
            self._ensure_column("throws", "field", "INTEGER")
            self._ensure_column("throws", "ring", "TEXT")
            self._ensure_column("throws", "multiplier", "INTEGER")
            self._ensure_column("throws", "dart_score", "INTEGER NOT NULL DEFAULT 0")
            self._ensure_column("throws", "mode_points", "INTEGER NOT NULL DEFAULT 0")
            self._ensure_column("throws", "outcome", "TEXT NOT NULL DEFAULT 'neutral'")
            self._ensure_column("throws", "source", "TEXT NOT NULL DEFAULT 'unknown'")
            self._ensure_column("throws", "task_json", "TEXT")
            self._ensure_column("throws", "event_id", "INTEGER")
            self._connection.execute(
                "CREATE INDEX IF NOT EXISTS idx_games_type_status ON games(game_type, status)"
            )
            self._connection.execute(
                "CREATE INDEX IF NOT EXISTS idx_throws_heatmap ON throws(player_id, field, ring)"
            )
            # BLE sequence numbers are useful for packet de-duplication but are
            # not event identities: they wrap, and simulated throws may reuse
            # them. The event journal's ordinal is the stable ordering key.
            self._connection.execute("DROP INDEX IF EXISTS idx_throws_game_seq")
            self._connection.execute(
                "CREATE INDEX IF NOT EXISTS idx_throws_game_seq ON throws(game_id, seq)"
            )
            self._connection.execute(
                """
                UPDATE throws SET
                    field=CAST(json_extract(event_json, '$.field') AS INTEGER),
                    ring=json_extract(event_json, '$.ring'),
                    multiplier=CAST(json_extract(event_json, '$.multiplier') AS INTEGER),
                    dart_score=COALESCE(CAST(json_extract(event_json, '$.score') AS INTEGER), 0)
                WHERE field IS NULL OR ring IS NULL
                """
            )
            self._connection.execute(
                """
                UPDATE throws SET outcome=
                    CASE
                        WHEN json_extract(event_json, '$.type')='miss' THEN 'miss'
                        WHEN json_extract(event_json, '$.type')='hit' THEN 'success'
                        ELSE outcome
                    END
                WHERE outcome='neutral'
                """
            )
            self._connection.execute(
                """
                INSERT OR IGNORE INTO game_winners(game_id, player_id)
                SELECT id, winner_id FROM games WHERE winner_id IS NOT NULL
                """
            )
            self._connection.execute(
                """
                INSERT OR IGNORE INTO game_players(game_id, player_id, position)
                SELECT g.id, sp.player_id, sp.position
                FROM games g
                JOIN session_players sp ON sp.session_id=g.session_id
                """
            )
            self._connection.execute("PRAGMA user_version=2")

    def _ensure_column(self, table: str, column: str, definition: str) -> None:
        columns = {
            row["name"]
            for row in self._connection.execute(f"PRAGMA table_info({table})").fetchall()
        }
        if column not in columns:
            self._connection.execute(
                f"ALTER TABLE {table} ADD COLUMN {column} {definition}"
            )

    def create_player(self, name: str, avatar: str = "comet", color: str = "#28e7ff") -> Dict[str, Any]:
        player = {
            "id": str(uuid4()),
            "name": name.strip(),
            "avatar": avatar,
            "color": color,
            "created_at": utc_now(),
        }
        if not player["name"]:
            raise ValueError("Player name must not be empty")
        with self._lock, self._connection:
            self._connection.execute(
                "INSERT INTO players(id, name, avatar, color, created_at) VALUES(:id, :name, :avatar, :color, :created_at)",
                player,
            )
        return player

    def list_players(self) -> List[Dict[str, Any]]:
        with self._lock:
            rows = self._connection.execute(
                "SELECT * FROM players ORDER BY lower(name), created_at"
            ).fetchall()
        return [dict(row) for row in rows]

    def start_session(
        self,
        player_ids: Iterable[str],
        language: str = "de",
    ) -> Dict[str, Any]:
        ids = list(dict.fromkeys(player_ids))
        if not ids:
            raise ValueError("A session needs at least one player")
        if language not in {"de", "en"}:
            raise ValueError("Session language must be de or en")
        session = {
            "id": str(uuid4()),
            "status": "active",
            "language": language,
            "started_at": utc_now(),
        }
        with self._lock, self._connection:
            known = {
                player_id
                for player_id in ids
                if self._connection.execute(
                    "SELECT 1 FROM players WHERE id=?",
                    (player_id,),
                ).fetchone()
            }
            missing = [player_id for player_id in ids if player_id not in known]
            if missing:
                raise ValueError(f"Unknown player ids: {', '.join(missing)}")
            self._connection.execute(
                """
                INSERT INTO sessions(id, status, language, started_at)
                VALUES(:id, :status, :language, :started_at)
                """,
                session,
            )
            self._connection.executemany(
                "INSERT INTO session_players(session_id, player_id, position) VALUES(?, ?, ?)",
                [(session["id"], player_id, position) for position, player_id in enumerate(ids)],
            )
        return self.get_session(session["id"])

    def get_session(self, session_id: str) -> Optional[Dict[str, Any]]:
        with self._lock:
            row = self._connection.execute("SELECT * FROM sessions WHERE id=?", (session_id,)).fetchone()
            if row is None:
                return None
            players = self._connection.execute(
                """
                SELECT p.* FROM players p
                JOIN session_players sp ON sp.player_id=p.id
                WHERE sp.session_id=? ORDER BY sp.position
                """,
                (session_id,),
            ).fetchall()
        result = dict(row)
        result["players"] = [dict(player) for player in players]
        return result

    def list_sessions(self, limit: int = 50) -> List[Dict[str, Any]]:
        safe_limit = max(1, min(int(limit), 500))
        with self._lock:
            rows = self._connection.execute(
                """
                SELECT s.*,
                       COUNT(DISTINCT g.id) AS games,
                       COUNT(DISTINCT CASE WHEN g.status='finished' THEN g.id END)
                           AS finished_games,
                       COUNT(DISTINCT sp.player_id) AS player_count
                FROM sessions s
                LEFT JOIN session_players sp ON sp.session_id=s.id
                LEFT JOIN games g ON g.session_id=s.id
                GROUP BY s.id
                ORDER BY s.started_at DESC
                LIMIT ?
                """,
                (safe_limit,),
            ).fetchall()
        return [dict(row) for row in rows]

    def active_session(self) -> Optional[Dict[str, Any]]:
        with self._lock:
            row = self._connection.execute(
                "SELECT id FROM sessions WHERE status='active' ORDER BY started_at DESC LIMIT 1"
            ).fetchone()
        return self.get_session(row["id"]) if row else None

    def end_session(self, session_id: str) -> None:
        with self._lock, self._connection:
            self._connection.execute(
                "UPDATE sessions SET status='finished', ended_at=? WHERE id=?",
                (utc_now(), session_id),
            )

    def start_game(
        self,
        session_id: str,
        game_type: str,
        options: Dict[str, Any],
        *,
        players: Iterable[Dict[str, Any]] = (),
        ruleset_version: int = 1,
        app_version: str = "",
        environment: str = "production",
        initial_state: Optional[Dict[str, Any]] = None,
    ) -> str:
        game_id = str(uuid4())
        lineup = list(players)
        with self._lock, self._connection:
            self._connection.execute(
                """
                INSERT INTO games(
                    id, session_id, game_type, status, options_json,
                    ruleset_version, app_version, environment,
                    initial_state_json, started_at
                )
                VALUES(?, ?, ?, 'running', ?, ?, ?, ?, ?, ?)
                """,
                (
                    game_id,
                    session_id,
                    game_type,
                    json.dumps(options),
                    int(ruleset_version),
                    app_version,
                    environment,
                    json.dumps(initial_state) if initial_state is not None else None,
                    utc_now(),
                ),
            )
            self._connection.executemany(
                """
                INSERT INTO game_players(game_id, player_id, position)
                VALUES(?, ?, ?)
                """,
                [
                    (game_id, str(player["id"]), position)
                    for position, player in enumerate(lineup)
                ],
            )
        return game_id

    def record_game_event(
        self,
        game_id: str,
        event_type: str,
        *,
        player_id: Optional[str] = None,
        source: str = "system",
        payload: Optional[Dict[str, Any]] = None,
        task: Optional[Dict[str, Any]] = None,
        frame: Optional[Dict[str, Any]] = None,
        effective: bool = True,
        corrects_event_id: Optional[int] = None,
        created_at: Optional[str] = None,
    ) -> int:
        with self._lock, self._connection:
            ordinal = int(
                self._connection.execute(
                    """
                    SELECT COALESCE(MAX(ordinal), 0) + 1
                    FROM game_events WHERE game_id=?
                    """,
                    (game_id,),
                ).fetchone()[0]
            )
            cursor = self._connection.execute(
                """
                INSERT INTO game_events(
                    game_id, ordinal, event_type, player_id, source,
                    payload_json, task_json, frame_json, effective,
                    corrects_event_id, created_at
                )
                VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    game_id,
                    ordinal,
                    event_type,
                    player_id,
                    source,
                    json.dumps(payload or {}),
                    json.dumps(task) if task is not None else None,
                    json.dumps(frame) if frame is not None else None,
                    1 if effective else 0,
                    corrects_event_id,
                    created_at or utc_now(),
                ),
            )
        return int(cursor.lastrowid)

    def set_game_environment(self, game_id: str, environment: str) -> None:
        if environment not in {"production", "test"}:
            raise ValueError("Game environment must be production or test")
        with self._lock, self._connection:
            self._connection.execute(
                "UPDATE games SET environment=? WHERE id=? AND status='running'",
                (environment, game_id),
            )

    def invalidate_last_throw_event(self, game_id: str) -> Optional[int]:
        """Keep the original event for audit but remove it from canonical replay."""
        with self._lock, self._connection:
            row = self._connection.execute(
                """
                SELECT id FROM game_events
                WHERE game_id=? AND event_type='throw' AND effective=1
                ORDER BY ordinal DESC LIMIT 1
                """,
                (game_id,),
            ).fetchone()
            if not row:
                return None
            event_id = int(row["id"])
            self._connection.execute(
                "UPDATE game_events SET effective=0 WHERE id=?",
                (event_id,),
            )
        return event_id

    def invalidate_gameplay_event(
        self,
        game_id: str,
        *,
        action_id: Optional[int] = None,
    ) -> Optional[int]:
        """Invalidate one effective gameplay event while retaining its audit row."""
        gameplay_types = {
            "throw",
            "continue_turn",
            "next_player",
            "game_action",
        }
        with self._lock, self._connection:
            rows = self._connection.execute(
                """
                SELECT id, event_type, payload_json
                FROM game_events
                WHERE game_id=? AND effective=1
                ORDER BY ordinal DESC
                """,
                (game_id,),
            ).fetchall()
            event_id = None
            for row in rows:
                if row["event_type"] not in gameplay_types:
                    continue
                payload = json.loads(row["payload_json"] or "{}")
                if action_id is not None and int(
                    payload.get("_action_id", 0) or 0
                ) != int(action_id):
                    continue
                event_id = int(row["id"])
                break
            if event_id is not None:
                self._connection.execute(
                    "UPDATE game_events SET effective=0 WHERE id=?",
                    (event_id,),
                )
        return event_id

    def record_throw(
        self,
        game_id: str,
        seq: int,
        player_id: Optional[str],
        event: Dict[str, Any],
        score_after: int,
        *,
        round_number: int = 1,
        dart_in_turn: int = 1,
        mode_points: int = 0,
        outcome: str = "neutral",
        source: str = "unknown",
        task: Optional[Dict[str, Any]] = None,
        event_id: Optional[int] = None,
    ) -> None:
        with self._lock, self._connection:
            self._connection.execute(
                """
                INSERT INTO throws(
                    game_id, seq, player_id, event_json, score_after,
                    round_number, dart_in_turn, field, ring, multiplier,
                    dart_score, mode_points, outcome, source, task_json,
                    event_id, created_at
                )
                VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    game_id,
                    seq,
                    player_id,
                    json.dumps(event),
                    score_after,
                    int(round_number),
                    int(dart_in_turn),
                    int(event["field"]) if event.get("field") is not None else None,
                    event.get("ring"),
                    int(event["multiplier"])
                    if event.get("multiplier") is not None
                    else None,
                    int(event.get("score", 0) or 0),
                    int(mode_points),
                    outcome,
                    source,
                    json.dumps(task) if task is not None else None,
                    event_id,
                    utc_now(),
                ),
            )

    def finish_game(
        self,
        game_id: str,
        winner_id: Optional[str] = None,
        winner_ids: Optional[Iterable[str]] = None,
        *,
        result_type: str = "",
        finish_reason: str = "",
        final_state: Optional[Dict[str, Any]] = None,
        final_scores: Optional[Dict[str, int]] = None,
    ) -> None:
        winners = list(dict.fromkeys(winner_ids or ([winner_id] if winner_id else [])))
        legacy_winner = winners[0] if len(winners) == 1 else None
        with self._lock, self._connection:
            self._connection.execute(
                """
                UPDATE games SET
                    status='finished',
                    winner_id=?,
                    result_type=?,
                    finish_reason=?,
                    final_state_json=?,
                    ended_at=?
                WHERE id=?
                """,
                (
                    legacy_winner,
                    result_type,
                    finish_reason,
                    json.dumps(final_state) if final_state is not None else None,
                    utc_now(),
                    game_id,
                ),
            )
            self._connection.execute(
                "DELETE FROM game_winners WHERE game_id=?",
                (game_id,),
            )
            self._connection.executemany(
                "INSERT INTO game_winners(game_id, player_id) VALUES(?, ?)",
                [(game_id, player_id) for player_id in winners],
            )
            if final_scores:
                self._connection.executemany(
                    """
                    UPDATE game_players SET final_score=?
                    WHERE game_id=? AND player_id=?
                    """,
                    [
                        (int(score), game_id, player_id)
                        for player_id, score in final_scores.items()
                    ],
                )

    def abort_game(
        self,
        game_id: str,
        *,
        reason: str = "user_abort",
        final_state: Optional[Dict[str, Any]] = None,
    ) -> None:
        """Keep an audit record while excluding the game from all statistics."""
        with self._lock, self._connection:
            self._connection.execute(
                """
                UPDATE games
                SET status='aborted', winner_id=NULL, finish_reason=?,
                    final_state_json=?, ended_at=?
                WHERE id=? AND status='running'
                """,
                (
                    reason,
                    json.dumps(final_state) if final_state is not None else None,
                    utc_now(),
                    game_id,
                ),
            )
            self._connection.execute(
                "DELETE FROM game_winners WHERE game_id=?",
                (game_id,),
            )

    def reopen_game(self, game_id: str) -> None:
        with self._lock, self._connection:
            self._connection.execute(
                """
                UPDATE games SET status='running', winner_id=NULL,
                    result_type='', finish_reason='', final_state_json=NULL,
                    ended_at=NULL
                WHERE id=?
                """,
                (game_id,),
            )
            self._connection.execute(
                "DELETE FROM game_winners WHERE game_id=?",
                (game_id,),
            )

    def delete_last_throw(self, game_id: str) -> None:
        with self._lock, self._connection:
            self._connection.execute(
                """
                DELETE FROM throws WHERE id=(
                    SELECT id FROM throws WHERE game_id=? ORDER BY id DESC LIMIT 1
                )
                """,
                (game_id,),
            )

    def replace_game_throws(
        self,
        game_id: str,
        throws: Iterable[Dict[str, Any]],
    ) -> None:
        """Atomically rewrite a game's event journal after a correction."""
        rows = list(throws)
        with self._lock, self._connection:
            self._connection.execute("DELETE FROM throws WHERE game_id=?", (game_id,))
            self._connection.executemany(
                """
                INSERT INTO throws(
                    game_id, seq, player_id, event_json, score_after,
                    round_number, dart_in_turn, field, ring, multiplier,
                    dart_score, mode_points, outcome, source, task_json,
                    event_id, created_at
                )
                VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                [
                    (
                        game_id,
                        int(throw["seq"]),
                        throw.get("player_id"),
                        json.dumps(throw["event"]),
                        int(throw.get("score_after", 0)),
                        int(throw.get("round_number", 1)),
                        int(throw.get("dart_in_turn", 1)),
                        throw.get("field"),
                        throw.get("ring"),
                        throw.get("multiplier"),
                        int(throw.get("dart_score", 0)),
                        int(throw.get("mode_points", 0)),
                        str(throw.get("outcome", "neutral")),
                        str(throw.get("source", "correction")),
                        json.dumps(throw.get("task"))
                        if throw.get("task") is not None
                        else None,
                        throw.get("event_id"),
                        utc_now(),
                    )
                    for throw in rows
                ],
            )

    def set_runtime_value(self, key: str, value: Any) -> None:
        with self._lock, self._connection:
            self._connection.execute(
                """
                INSERT INTO runtime_state(key, value_json, updated_at)
                VALUES(?, ?, ?)
                ON CONFLICT(key) DO UPDATE SET
                    value_json=excluded.value_json,
                    updated_at=excluded.updated_at
                """,
                (key, json.dumps(value), utc_now()),
            )

    def get_runtime_value(self, key: str, default: Any = None) -> Any:
        with self._lock:
            row = self._connection.execute(
                "SELECT value_json FROM runtime_state WHERE key=?", (key,)
            ).fetchone()
        return json.loads(row["value_json"]) if row else default

    def get_game(self, game_id: str) -> Optional[Dict[str, Any]]:
        with self._lock:
            row = self._connection.execute(
                "SELECT * FROM games WHERE id=?", (game_id,)
            ).fetchone()
        if row is None:
            return None
        result = dict(row)
        result["options"] = json.loads(result.pop("options_json"))
        return result

    def statistics(
        self,
        player_ids: Optional[Iterable[str]] = None,
        *,
        completed_only: bool = False,
        include_nonproduction: bool = False,
    ) -> List[Dict[str, Any]]:
        selected_ids = set(player_ids or [])
        statuses = "g.status='finished'" if completed_only else "g.status IN ('running', 'finished')"
        environment = "" if include_nonproduction else "AND g.environment='production'"
        query = f"""
            SELECT p.id, p.name, p.avatar, p.color,
                   COUNT(DISTINCT g.id) AS games,
                   COUNT(DISTINCT CASE WHEN gw.player_id IS NOT NULL THEN g.id END) AS wins,
                   COUNT(t.id) AS darts,
                   COALESCE(SUM(t.dart_score), 0) AS total_points,
                   COALESCE(SUM(t.mode_points), 0) AS total_mode_points,
                   COALESCE(MAX(t.dart_score), 0) AS best_dart,
                   COALESCE(SUM(CASE WHEN t.outcome='miss' THEN 1 ELSE 0 END), 0) AS misses
            FROM players p
            LEFT JOIN session_players sp ON sp.player_id=p.id
            LEFT JOIN games g
              ON g.session_id=sp.session_id
             AND {statuses}
             {environment}
            LEFT JOIN throws t ON t.game_id=g.id AND t.player_id=p.id
            LEFT JOIN game_winners gw
              ON gw.game_id=g.id AND gw.player_id=p.id
            GROUP BY p.id
            ORDER BY wins DESC, total_points DESC, lower(p.name)
        """
        with self._lock:
            rows = self._connection.execute(query).fetchall()
        result = []
        for row in rows:
            item = dict(row)
            if selected_ids and item["id"] not in selected_ids:
                continue
            item["three_dart_average"] = round(
                item["total_points"] / item["darts"] * 3, 2
            ) if item["darts"] else 0
            item["win_rate"] = round(item["wins"] / item["games"] * 100, 1) if item["games"] else 0
            result.append(item)
        return result

    def session_statistics(self, session_id: str) -> List[Dict[str, Any]]:
        """Return a clean leaderboard for one session.

        Only completed games count. A win awards three session points; aborted
        and currently running games contribute neither games, darts nor points.
        """
        query = """
            SELECT p.id, p.name, p.avatar, p.color,
                   COUNT(DISTINCT g.id) AS games,
                   COUNT(DISTINCT CASE WHEN gw.player_id IS NOT NULL THEN g.id END) AS wins,
                   COUNT(t.id) AS darts,
                   COALESCE(SUM(t.dart_score), 0) AS total_points,
                   COALESCE(SUM(t.mode_points), 0) AS total_mode_points,
                   COALESCE(MAX(t.dart_score), 0) AS best_dart,
                   COALESCE(SUM(CASE WHEN t.outcome='miss' THEN 1 ELSE 0 END), 0) AS misses
            FROM session_players sp
            JOIN players p ON p.id=sp.player_id
            LEFT JOIN games g
              ON g.session_id=sp.session_id AND g.status='finished'
            LEFT JOIN throws t ON t.game_id=g.id AND t.player_id=p.id
            LEFT JOIN game_winners gw
              ON gw.game_id=g.id AND gw.player_id=p.id
            WHERE sp.session_id=?
            GROUP BY p.id
            ORDER BY wins DESC, total_points DESC, lower(p.name)
        """
        with self._lock:
            rows = self._connection.execute(query, (session_id,)).fetchall()
        result = []
        for row in rows:
            item = dict(row)
            item["session_points"] = int(item["wins"]) * 3
            item["three_dart_average"] = round(
                item["total_points"] / item["darts"] * 3, 2
            ) if item["darts"] else 0
            item["win_rate"] = round(
                item["wins"] / item["games"] * 100, 1
            ) if item["games"] else 0
            result.append(item)
        return result

    def reconcile_running_games(self, active_game_id: Optional[str]) -> int:
        """Mark stale running records without deleting their audit history."""
        with self._lock, self._connection:
            if active_game_id:
                cursor = self._connection.execute(
                    """
                    UPDATE games SET status='interrupted',
                        finish_reason='orphaned_runtime', ended_at=?
                    WHERE status='running' AND id<>?
                    """,
                    (utc_now(), active_game_id),
                )
            else:
                cursor = self._connection.execute(
                    """
                    UPDATE games SET status='interrupted',
                        finish_reason='orphaned_runtime', ended_at=?
                    WHERE status='running'
                    """,
                    (utc_now(),),
                )
        return int(cursor.rowcount)

    def session_detail(self, session_id: str) -> Optional[Dict[str, Any]]:
        session = self.get_session(session_id)
        if not session:
            return None
        with self._lock:
            games = self._connection.execute(
                """
                SELECT g.*,
                       COUNT(DISTINCT t.id) AS darts,
                       COUNT(DISTINCT gw.player_id) AS winner_count
                FROM games g
                LEFT JOIN throws t ON t.game_id=g.id
                LEFT JOIN game_winners gw ON gw.game_id=g.id
                WHERE g.session_id=?
                GROUP BY g.id
                ORDER BY g.started_at
                """,
                (session_id,),
            ).fetchall()
        session["games"] = [
            {
                **dict(row),
                "options": json.loads(row["options_json"]),
            }
            for row in games
        ]
        for game in session["games"]:
            game.pop("options_json", None)
            game.pop("initial_state_json", None)
            game.pop("final_state_json", None)
        session["statistics"] = self.session_statistics(session_id)
        return session

    def game_detail(self, game_id: str) -> Optional[Dict[str, Any]]:
        game = self.get_game(game_id)
        if not game:
            return None
        with self._lock:
            players = self._connection.execute(
                """
                SELECT p.id, p.name, p.avatar, p.color, gp.position,
                       gp.final_score,
                       CASE WHEN gw.player_id IS NULL THEN 0 ELSE 1 END AS winner,
                       COUNT(t.id) AS darts,
                       COALESCE(SUM(t.dart_score), 0) AS dart_points,
                       COALESCE(SUM(t.mode_points), 0) AS mode_points,
                       COALESCE(SUM(CASE WHEN t.outcome='miss' THEN 1 ELSE 0 END), 0)
                           AS misses,
                       COALESCE(SUM(CASE WHEN t.outcome='success' THEN 1 ELSE 0 END), 0)
                           AS successes,
                       COALESCE(SUM(CASE WHEN t.outcome='partial' THEN 1 ELSE 0 END), 0)
                           AS partials,
                       COALESCE(SUM(CASE WHEN t.outcome='danger' THEN 1 ELSE 0 END), 0)
                           AS dangers
                FROM game_players gp
                JOIN players p ON p.id=gp.player_id
                LEFT JOIN game_winners gw
                  ON gw.game_id=gp.game_id AND gw.player_id=gp.player_id
                LEFT JOIN throws t
                  ON t.game_id=gp.game_id AND t.player_id=gp.player_id
                WHERE gp.game_id=?
                GROUP BY p.id
                ORDER BY gp.position
                """,
                (game_id,),
            ).fetchall()
            throws = self._connection.execute(
                """
                SELECT id, seq, player_id, score_after, round_number,
                       dart_in_turn, field, ring, multiplier, dart_score,
                       mode_points, outcome, source, task_json, event_json,
                       created_at
                FROM throws WHERE game_id=? ORDER BY id
                """,
                (game_id,),
            ).fetchall()
        game["players"] = [dict(row) for row in players]
        game["throws"] = []
        for row in throws:
            item = dict(row)
            item["event"] = json.loads(item.pop("event_json"))
            task_json = item.pop("task_json")
            item["task"] = json.loads(task_json) if task_json else None
            game["throws"].append(item)
        if game.get("initial_state_json"):
            game["initial_state"] = json.loads(game.pop("initial_state_json"))
        if game.get("final_state_json"):
            game["final_state"] = json.loads(game.pop("final_state_json"))
        return game

    def game_replay(self, game_id: str) -> Optional[Dict[str, Any]]:
        game = self.get_game(game_id)
        if not game:
            return None
        with self._lock:
            rows = self._connection.execute(
                """
                SELECT id, ordinal, event_type, player_id, source,
                       payload_json, task_json, frame_json, effective,
                       corrects_event_id, created_at
                FROM game_events
                WHERE game_id=?
                ORDER BY ordinal
                """,
                (game_id,),
            ).fetchall()
        events = []
        for row in rows:
            item = dict(row)
            item["payload"] = json.loads(item.pop("payload_json"))
            task_json = item.pop("task_json")
            frame_json = item.pop("frame_json")
            item["task"] = json.loads(task_json) if task_json else None
            item["frame"] = json.loads(frame_json) if frame_json else None
            item["effective"] = bool(item["effective"])
            events.append(item)
        if not events:
            detail = self.game_detail(game_id)
            players = {
                player["id"]: {
                    "id": player["id"],
                    "name": player["name"],
                    "avatar": player["avatar"],
                    "color": player["color"],
                    "score": (
                        int(game["options"].get("start_score", 501))
                        if game["game_type"] == "x01"
                        else 0
                    ),
                }
                for player in (detail["players"] if detail else [])
            }
            events = []
            for index, throw in enumerate(detail["throws"] if detail else []):
                if throw["player_id"] in players:
                    players[throw["player_id"]]["score"] = int(
                        throw["score_after"]
                    )
                events.append({
                    "id": throw["id"],
                    "ordinal": index + 1,
                    "event_type": "throw",
                    "player_id": throw["player_id"],
                    "source": throw["source"],
                    "payload": throw["event"],
                    "task": throw["task"],
                    "frame": {
                        "game_type": game["game_type"],
                        "players": [dict(player) for player in players.values()],
                        "current_player_id": throw["player_id"],
                        "round_number": throw["round_number"],
                        "darts_in_turn": throw["dart_in_turn"],
                        "status": "running",
                        "last_event": throw["event"],
                        "overlay": {},
                    },
                    "effective": True,
                    "created_at": throw["created_at"],
                })
        return {
            "game": {
                "id": game["id"],
                "game_type": game["game_type"],
                "status": game["status"],
                "options": game["options"],
                "ruleset_version": game["ruleset_version"],
                "environment": game["environment"],
            },
            "events": events,
        }

    def heatmap(
        self,
        *,
        player_id: Optional[str] = None,
        session_id: Optional[str] = None,
        game_type: Optional[str] = None,
        include_nonproduction: bool = False,
    ) -> Dict[str, Any]:
        base_conditions = ["g.status='finished'"]
        params: List[Any] = []
        if not include_nonproduction:
            base_conditions.append("g.environment='production'")
        if player_id:
            base_conditions.append("t.player_id=?")
            params.append(player_id)
        if session_id:
            base_conditions.append("g.session_id=?")
            params.append(session_id)
        if game_type:
            base_conditions.append("g.game_type=?")
            params.append(game_type)
        conditions = ["t.field IS NOT NULL", *base_conditions]
        query = f"""
            SELECT t.field, t.ring,
                   COUNT(*) AS darts,
                   SUM(CASE WHEN t.outcome='success' THEN 1 ELSE 0 END) AS successes,
                   SUM(CASE WHEN t.outcome='danger' THEN 1 ELSE 0 END) AS dangers,
                   SUM(CASE WHEN t.outcome='neutral' THEN 1 ELSE 0 END) AS neutral,
                   SUM(t.dart_score) AS dart_points,
                   SUM(t.mode_points) AS mode_points
            FROM throws t
            JOIN games g ON g.id=t.game_id
            WHERE {" AND ".join(conditions)}
            GROUP BY t.field, t.ring
            ORDER BY t.field, t.ring
        """
        with self._lock:
            rows = self._connection.execute(query, params).fetchall()
            totals = self._connection.execute(
                f"""
                SELECT COUNT(*) AS recorded_darts,
                       SUM(CASE WHEN t.field IS NOT NULL THEN 1 ELSE 0 END)
                           AS board_hits,
                       SUM(CASE WHEN t.outcome='miss' THEN 1 ELSE 0 END)
                           AS misses
                FROM throws t
                JOIN games g ON g.id=t.game_id
                WHERE {" AND ".join(base_conditions)}
                """,
                params,
            ).fetchone()
        segments = [dict(row) for row in rows]
        return {
            "resolution": "dartboard_segment",
            "segments": segments,
            "total_darts": int(totals["recorded_darts"] or 0),
            "board_hits": int(totals["board_hits"] or 0),
            "misses": int(totals["misses"] or 0),
        }

    def mode_statistics(self, include_nonproduction: bool = False) -> List[Dict[str, Any]]:
        environment_clause = "" if include_nonproduction else "WHERE g.environment='production'"
        with self._lock:
            rows = self._connection.execute(
                f"""
                SELECT g.game_type, g.ruleset_version, g.options_json,
                       COUNT(DISTINCT g.id) AS starts,
                       COUNT(DISTINCT CASE WHEN g.status='finished' THEN g.id END)
                           AS finished,
                       COUNT(DISTINCT CASE WHEN g.status='aborted' THEN g.id END)
                           AS aborted,
                       COUNT(DISTINCT CASE WHEN g.status='interrupted' THEN g.id END)
                           AS interrupted,
                       ROUND(AVG(CASE WHEN g.ended_at IS NOT NULL
                           THEN (julianday(g.ended_at)-julianday(g.started_at))*86400
                           END), 1) AS average_seconds,
                       SUM((SELECT COUNT(*) FROM throws t WHERE t.game_id=g.id))
                           AS darts,
                       SUM((SELECT COUNT(*) FROM throws t
                            WHERE t.game_id=g.id AND t.outcome='success')) AS successes,
                       SUM((SELECT COUNT(*) FROM throws t
                            WHERE t.game_id=g.id AND t.outcome='partial')) AS partials,
                       SUM((SELECT COUNT(*) FROM throws t
                            WHERE t.game_id=g.id AND t.outcome='danger')) AS dangers,
                       SUM((SELECT COUNT(*) FROM throws t
                            WHERE t.game_id=g.id AND t.outcome='miss')) AS misses
                FROM games g
                {environment_clause}
                GROUP BY g.game_type, g.ruleset_version, g.options_json
                ORDER BY g.game_type, g.ruleset_version, g.options_json
                """
            ).fetchall()
        result = []
        for row in rows:
            item = dict(row)
            item["options"] = json.loads(item.pop("options_json"))
            attempts = int(item["darts"] or 0)
            item["success_rate"] = round(
                int(item["successes"] or 0) / attempts * 100, 1
            ) if attempts else 0
            item["completion_rate"] = round(
                int(item["finished"] or 0) / int(item["starts"] or 1) * 100, 1
            )
            result.append(item)
        return result

    def training_recommendations(self, player_id: str) -> Dict[str, Any]:
        with self._lock:
            rows = self._connection.execute(
                """
                SELECT t.field, t.ring, t.outcome, t.task_json
                FROM throws t
                JOIN games g ON g.id=t.game_id
                WHERE t.player_id=? AND g.status='finished'
                  AND g.environment='production'
                  AND t.task_json IS NOT NULL
                ORDER BY t.id DESC LIMIT 2000
                """,
                (player_id,),
            ).fetchall()
        zones: Dict[str, Dict[str, Any]] = {}
        for row in rows:
            task = json.loads(row["task_json"])
            targets = task.get("targets", [])
            if not targets:
                continue
            actual = f"{row['ring']}:{row['field']}"
            for target in targets:
                field = target.get("field")
                rings = target.get("rings") or [target.get("ring")]
                for ring in [item for item in rings if item]:
                    key = f"{ring}:{field}"
                    item = zones.setdefault(
                        key,
                        {"field": field, "ring": ring, "attempts": 0, "successes": 0},
                    )
                    item["attempts"] += 1
                    if actual == key and row["outcome"] == "success":
                        item["successes"] += 1
        recommendations = []
        for item in zones.values():
            attempts = int(item["attempts"])
            item["success_rate"] = round(item["successes"] / attempts * 100, 1)
            if attempts >= 3:
                recommendations.append(item)
        recommendations.sort(key=lambda item: (item["success_rate"], -item["attempts"]))
        if not recommendations:
            recommendations = [
                {
                    "field": 20,
                    "ring": "double",
                    "attempts": 0,
                    "successes": 0,
                    "success_rate": 0,
                    "starter": True,
                },
                {
                    "field": 25,
                    "ring": "single_bull",
                    "attempts": 0,
                    "successes": 0,
                    "success_rate": 0,
                    "starter": True,
                },
            ]
        return {
            "player_id": player_id,
            "recommendations": recommendations[:8],
        }

    def export_data(self) -> Dict[str, Any]:
        """Return a portable, runtime-secret-free archive of historical data."""
        with self._lock:
            session_ids = [
                row["id"]
                for row in self._connection.execute(
                    "SELECT id FROM sessions ORDER BY started_at"
                ).fetchall()
            ]
            game_ids = [
                row["id"]
                for row in self._connection.execute(
                    "SELECT id FROM games ORDER BY started_at"
                ).fetchall()
            ]
        return {
            "schema_version": 2,
            "exported_at": utc_now(),
            "players": self.list_players(),
            "sessions": [
                self.session_detail(session_id) for session_id in session_ids
            ],
            "games": [
                {
                    "detail": self.game_detail(game_id),
                    "replay": self.game_replay(game_id),
                }
                for game_id in game_ids
            ],
        }
