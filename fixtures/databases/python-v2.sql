PRAGMA foreign_keys=ON;

CREATE TABLE players (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    avatar TEXT NOT NULL DEFAULT 'comet',
    color TEXT NOT NULL DEFAULT '#28e7ff',
    created_at TEXT NOT NULL
);
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    language TEXT NOT NULL DEFAULT 'de',
    started_at TEXT NOT NULL,
    ended_at TEXT
);
CREATE TABLE session_players (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    player_id TEXT NOT NULL REFERENCES players(id),
    position INTEGER NOT NULL,
    PRIMARY KEY (session_id, player_id)
);
CREATE TABLE games (
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
CREATE TABLE throws (
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
CREATE TABLE game_winners (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    player_id TEXT NOT NULL REFERENCES players(id),
    PRIMARY KEY (game_id, player_id)
);
CREATE TABLE game_players (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    player_id TEXT NOT NULL REFERENCES players(id),
    position INTEGER NOT NULL,
    final_score INTEGER,
    PRIMARY KEY (game_id, player_id)
);
CREATE TABLE game_events (
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
CREATE TABLE runtime_state (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO players(id, name, avatar, color, created_at) VALUES
    ('ada', 'Ada', 'fox', '#ff00aa', '2026-07-01T18:00:00Z'),
    ('bob', 'Bob', 'comet', '#28e7ff', '2026-07-01T18:01:00Z');
INSERT INTO sessions(id, status, language, started_at, ended_at) VALUES
    ('session-finished', 'finished', 'de', '2026-07-01T19:00:00Z', '2026-07-01T19:20:00Z'),
    ('session-active', 'active', 'en', '2026-07-02T19:00:00Z', NULL);
INSERT INTO session_players(session_id, player_id, position) VALUES
    ('session-finished', 'ada', 0),
    ('session-finished', 'bob', 1),
    ('session-active', 'ada', 0),
    ('session-active', 'bob', 1);
INSERT INTO games(
    id, session_id, game_type, status, options_json, winner_id, result_type,
    finish_reason, ruleset_version, app_version, environment,
    initial_state_json, final_state_json, started_at, ended_at
) VALUES
    ('game-finished', 'session-finished', 'countup', 'finished', '{"rounds":8}',
     'ada', 'high_score', 'rules_complete', 1, '0.0.2', 'production',
     '{"players":[]}', '{"players":[]}', '2026-07-01T19:01:00Z', '2026-07-01T19:15:00Z'),
    ('game-running', 'session-active', 'x01', 'running', '{"start_score":301,"out_rule":"double_out"}',
     NULL, '', '', 1, '0.0.2', 'production',
     '{"players":[]}', NULL, '2026-07-02T19:01:00Z', NULL);
INSERT INTO game_players(game_id, player_id, position, final_score) VALUES
    ('game-finished', 'ada', 0, 60),
    ('game-finished', 'bob', 1, 20),
    ('game-running', 'ada', 0, NULL),
    ('game-running', 'bob', 1, NULL);
INSERT INTO game_winners(game_id, player_id) VALUES('game-finished', 'ada');
INSERT INTO game_events(
    id, game_id, ordinal, event_type, player_id, source, payload_json,
    task_json, frame_json, effective, corrects_event_id, created_at
) VALUES(
    1, 'game-finished', 1, 'throw', 'ada', 'board',
    '{"event":{"type":"hit","seq":7,"field":20,"ring":"triple","multiplier":3,"label":"T20","score":60}}',
    NULL, '{"players":[{"id":"ada","score":60}]}', 1, NULL, '2026-07-01T19:02:00Z'
);
INSERT INTO throws(
    id, game_id, seq, player_id, event_json, score_after, round_number,
    dart_in_turn, field, ring, multiplier, dart_score, mode_points, outcome,
    source, task_json, event_id, created_at
) VALUES(
    1, 'game-finished', 7, 'ada',
    '{"type":"hit","seq":7,"field":20,"ring":"triple","multiplier":3,"label":"T20","score":60}',
    60, 1, 1, 20, 'triple', 3, 60, -4, 'success', 'board', NULL, 1,
    '2026-07-01T19:02:00Z'
);
INSERT INTO runtime_state(key, value_json, updated_at) VALUES
    ('calibration', '{"corners":[{"x":0.2,"y":0.1},{"x":0.8,"y":0.1},{"x":0.8,"y":0.9},{"x":0.2,"y":0.9}],"scale":1.1,"offset_x":0.02,"offset_y":-0.03}', '2026-07-02T19:05:00Z'),
    ('sound', '{"enabled":true,"output":"both","status":"ready"}', '2026-07-02T19:05:00Z'),
    ('art_theme', '"neon"', '2026-07-02T19:05:00Z'),
    ('ui_language', '"en"', '2026-07-02T19:05:00Z');

PRAGMA user_version=2;
