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
                CREATE TABLE IF NOT EXISTS runtime_state (
                    key TEXT PRIMARY KEY,
                    value_json TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                """
            )
            self._connection.execute(
                """
                INSERT OR IGNORE INTO game_winners(game_id, player_id)
                SELECT id, winner_id FROM games WHERE winner_id IS NOT NULL
                """
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

    def start_session(self, player_ids: Iterable[str]) -> Dict[str, Any]:
        ids = list(dict.fromkeys(player_ids))
        if not ids:
            raise ValueError("A session needs at least one player")
        session = {"id": str(uuid4()), "status": "active", "started_at": utc_now()}
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
                "INSERT INTO sessions(id, status, started_at) VALUES(:id, :status, :started_at)",
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

    def start_game(self, session_id: str, game_type: str, options: Dict[str, Any]) -> str:
        game_id = str(uuid4())
        with self._lock, self._connection:
            self._connection.execute(
                """
                INSERT INTO games(id, session_id, game_type, status, options_json, started_at)
                VALUES(?, ?, ?, 'running', ?, ?)
                """,
                (game_id, session_id, game_type, json.dumps(options), utc_now()),
            )
        return game_id

    def record_throw(
        self,
        game_id: str,
        seq: int,
        player_id: Optional[str],
        event: Dict[str, Any],
        score_after: int,
    ) -> None:
        with self._lock, self._connection:
            self._connection.execute(
                """
                INSERT INTO throws(game_id, seq, player_id, event_json, score_after, created_at)
                VALUES(?, ?, ?, ?, ?, ?)
                """,
                (game_id, seq, player_id, json.dumps(event), score_after, utc_now()),
            )

    def finish_game(
        self,
        game_id: str,
        winner_id: Optional[str] = None,
        winner_ids: Optional[Iterable[str]] = None,
    ) -> None:
        winners = list(dict.fromkeys(winner_ids or ([winner_id] if winner_id else [])))
        legacy_winner = winners[0] if len(winners) == 1 else None
        with self._lock, self._connection:
            self._connection.execute(
                "UPDATE games SET status='finished', winner_id=?, ended_at=? WHERE id=?",
                (legacy_winner, utc_now(), game_id),
            )
            self._connection.execute(
                "DELETE FROM game_winners WHERE game_id=?",
                (game_id,),
            )
            self._connection.executemany(
                "INSERT INTO game_winners(game_id, player_id) VALUES(?, ?)",
                [(game_id, player_id) for player_id in winners],
            )

    def abort_game(self, game_id: str) -> None:
        """Keep an audit record while excluding the game from all statistics."""
        with self._lock, self._connection:
            self._connection.execute(
                """
                UPDATE games
                SET status='aborted', winner_id=NULL, ended_at=?
                WHERE id=? AND status='running'
                """,
                (utc_now(), game_id),
            )
            self._connection.execute(
                "DELETE FROM game_winners WHERE game_id=?",
                (game_id,),
            )

    def reopen_game(self, game_id: str) -> None:
        with self._lock, self._connection:
            self._connection.execute(
                "UPDATE games SET status='running', winner_id=NULL, ended_at=NULL WHERE id=?",
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
                INSERT INTO throws(game_id, seq, player_id, event_json, score_after, created_at)
                VALUES(?, ?, ?, ?, ?, ?)
                """,
                [
                    (
                        game_id,
                        int(throw["seq"]),
                        throw.get("player_id"),
                        json.dumps(throw["event"]),
                        int(throw.get("score_after", 0)),
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

    def statistics(self, player_ids: Optional[Iterable[str]] = None) -> List[Dict[str, Any]]:
        selected_ids = set(player_ids or [])
        query = """
            SELECT p.id, p.name, p.avatar, p.color,
                   COUNT(DISTINCT CASE WHEN g.status='finished' THEN g.id END) AS games,
                   COUNT(DISTINCT CASE WHEN gw.player_id IS NOT NULL THEN g.id END) AS wins,
                   COUNT(t.id) AS darts,
                   COALESCE(SUM(CAST(json_extract(t.event_json, '$.score') AS INTEGER)), 0) AS total_points,
                   COALESCE(MAX(CAST(json_extract(t.event_json, '$.score') AS INTEGER)), 0) AS best_dart,
                   COALESCE(SUM(CASE WHEN json_extract(t.event_json, '$.type')='miss' THEN 1 ELSE 0 END), 0) AS misses
            FROM players p
            LEFT JOIN session_players sp ON sp.player_id=p.id
            LEFT JOIN games g ON g.session_id=sp.session_id
            LEFT JOIN throws t
              ON t.game_id=g.id AND t.player_id=p.id AND g.status!='aborted'
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
                   COALESCE(SUM(CAST(json_extract(t.event_json, '$.score') AS INTEGER)), 0) AS total_points,
                   COALESCE(MAX(CAST(json_extract(t.event_json, '$.score') AS INTEGER)), 0) AS best_dart,
                   COALESCE(SUM(CASE WHEN json_extract(t.event_json, '$.type')='miss' THEN 1 ELSE 0 END), 0) AS misses
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
