PRAGMA user_version = 9;

CREATE TABLE conversations (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO conversations (id) VALUES (1);

CREATE TABLE approvals (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    turn_id    INTEGER,
    member_id  TEXT,
    action     TEXT NOT NULL,
    body       TEXT NOT NULL,
    decision   TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO approvals (id, action, body, decision)
VALUES (3, 'shell', 'cargo test', 'pending');

-- A v9 database opened and used by an unversioned v0.2 build can contain both
-- the renamed table and its old workflow counterpart. Their ids may collide.
CREATE TABLE runs (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id      INTEGER NOT NULL DEFAULT 0,
    goal                 TEXT NOT NULL,
    status               TEXT NOT NULL,
    coordinator          TEXT,
    verification_command TEXT,
    verification_ok      INTEGER,
    verification_summary TEXT,
    created_at           TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    attempt              INTEGER NOT NULL DEFAULT 1,
    mode                 TEXT,
    mode_state           TEXT
);
INSERT INTO runs (id, conversation_id, goal, status)
VALUES (7, 1, 'new run retained', 'done');

CREATE TABLE workflow_runs (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    goal                 TEXT NOT NULL,
    status               TEXT NOT NULL,
    coordinator          TEXT,
    verification_command TEXT,
    verification_ok      INTEGER,
    verification_summary TEXT,
    created_at           TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    attempt              INTEGER NOT NULL DEFAULT 1,
    mode                 TEXT,
    mode_state           TEXT
);
INSERT INTO workflow_runs (
    id, goal, status, coordinator, verification_command,
    verification_ok, verification_summary, attempt, mode, mode_state
) VALUES (
    7, 'legacy run retained', 'blocked', 'planner', 'cargo test',
    0, 'one test failed', 2, 'lead', '{"phase":"revision","iteration":2}'
);

CREATE TABLE workflow_run_events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id     INTEGER NOT NULL,
    attempt    INTEGER NOT NULL,
    kind       TEXT NOT NULL,
    title      TEXT NOT NULL,
    detail     TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO workflow_run_events (id, run_id, attempt, kind, title, detail)
VALUES (11, 7, 2, 'blocked', 'Legacy event', 'waiting');

CREATE TABLE workflow_run_steps (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id     INTEGER NOT NULL,
    position   INTEGER NOT NULL,
    status     TEXT NOT NULL,
    owner      TEXT,
    title      TEXT NOT NULL,
    note       TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO workflow_run_steps (
    id, run_id, position, status, owner, title, note
) VALUES (13, 7, 1, 'blocked', 'builder', 'Legacy step', 'needs revision');
