-- Tracer primary SQLite schema v1 (W1-E)
-- Control plane is the sole planned writer of this database.
-- No secrets / auth tokens columns: credentials must not live in application tables.
--
-- Canonical copy also lives under crates/tracer-storage/migrations/.
-- Keep these files in sync; the storage crate embeds its copy for tests.

PRAGMA foreign_keys = ON;

-- ---------------------------------------------------------------------------
-- Projects
-- ---------------------------------------------------------------------------
CREATE TABLE projects (
    project_id   TEXT PRIMARY KEY NOT NULL,
    name         TEXT NOT NULL,
    root_path    TEXT NOT NULL,
    status       TEXT NOT NULL
                 CHECK (status IN ('ready', 'missing', 'invalid')),
    is_git       INTEGER NOT NULL DEFAULT 0
                 CHECK (is_git IN (0, 1)),
    last_opened_at TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_projects_root_path ON projects(root_path);

-- ---------------------------------------------------------------------------
-- Sessions
-- ---------------------------------------------------------------------------
CREATE TABLE sessions (
    session_id           TEXT PRIMARY KEY NOT NULL,
    project_id           TEXT NOT NULL
                         REFERENCES projects(project_id) ON DELETE CASCADE,
    title                TEXT,
    status               TEXT NOT NULL
                         CHECK (status IN (
                             'creating',
                             'starting_runtime',
                             'ready',
                             'running',
                             'awaiting_approval',
                             'cancelling',
                             'completed',
                             'failed',
                             'disconnected',
                             'stopped'
                         )),
    runtime_kind         TEXT,
    runtime_session_id   TEXT,
    capabilities_json    TEXT,
    last_error_json      TEXT,
    active_agent_run_id  TEXT,
    -- Next sequence value to assign (starts at 1). Control plane owns sequencing.
    next_sequence        INTEGER NOT NULL DEFAULT 1
                         CHECK (next_sequence >= 1),
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE INDEX idx_sessions_project ON sessions(project_id);
CREATE INDEX idx_sessions_status ON sessions(status);

-- ---------------------------------------------------------------------------
-- Events (normalized Tracer Event Protocol v1 envelopes)
-- Full envelope_json preserves unknown types and unknown payload fields.
-- ---------------------------------------------------------------------------
CREATE TABLE events (
    event_id       TEXT PRIMARY KEY NOT NULL,
    session_id     TEXT NOT NULL
                   REFERENCES sessions(session_id) ON DELETE CASCADE,
    project_id     TEXT NOT NULL,
    agent_run_id   TEXT,
    sequence       INTEGER NOT NULL
                   CHECK (sequence >= 1),
    event_version  INTEGER NOT NULL
                   CHECK (event_version >= 1),
    event_type     TEXT NOT NULL,
    severity       TEXT NOT NULL DEFAULT 'info'
                   CHECK (severity IN ('info', 'warn', 'error')),
    timestamp      TEXT NOT NULL,
    payload_json   TEXT NOT NULL,
    adapter_json   TEXT,
    envelope_json  TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_events_session_sequence ON events(session_id, sequence);
CREATE INDEX idx_events_session_type ON events(session_id, event_type);

-- ---------------------------------------------------------------------------
-- Runtime process summaries (diagnostics; not a second writer path)
-- ---------------------------------------------------------------------------
CREATE TABLE runtime_processes (
    process_id   TEXT PRIMARY KEY NOT NULL,
    session_id   TEXT NOT NULL
                 REFERENCES sessions(session_id) ON DELETE CASCADE,
    pid          INTEGER,
    executable   TEXT,
    args_json    TEXT,
    cwd          TEXT,
    status       TEXT NOT NULL
                 CHECK (status IN ('starting', 'running', 'exited', 'failed')),
    exit_code    INTEGER,
    exit_signal  TEXT,
    started_at   TEXT NOT NULL,
    ended_at     TEXT
);

CREATE INDEX idx_runtime_processes_session ON runtime_processes(session_id);

-- ---------------------------------------------------------------------------
-- Approval decisions (audit)
-- ---------------------------------------------------------------------------
CREATE TABLE approval_decisions (
    approval_id   TEXT PRIMARY KEY NOT NULL,
    session_id    TEXT NOT NULL
                  REFERENCES sessions(session_id) ON DELETE CASCADE,
    event_id      TEXT,
    decision      TEXT NOT NULL
                  CHECK (decision IN ('allow', 'deny', 'allow_always', 'deny_always')),
    decided_at    TEXT NOT NULL,
    details_json  TEXT
);

CREATE INDEX idx_approvals_session ON approval_decisions(session_id);

-- ---------------------------------------------------------------------------
-- Basic artifacts (file-change summaries etc.; not a full artifact store)
-- ---------------------------------------------------------------------------
CREATE TABLE artifacts (
    artifact_id    TEXT PRIMARY KEY NOT NULL,
    session_id     TEXT NOT NULL
                   REFERENCES sessions(session_id) ON DELETE CASCADE,
    project_id     TEXT NOT NULL,
    kind           TEXT NOT NULL,
    path           TEXT,
    summary        TEXT,
    metadata_json  TEXT,
    created_at     TEXT NOT NULL
);

CREATE INDEX idx_artifacts_session ON artifacts(session_id);

-- ---------------------------------------------------------------------------
-- Schema / app metadata (version markers, non-secret flags)
-- ---------------------------------------------------------------------------
CREATE TABLE storage_meta (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

INSERT INTO storage_meta (key, value) VALUES ('schema_logical_version', '1');
INSERT INTO storage_meta (key, value) VALUES ('writer_policy', 'control_plane_only');
