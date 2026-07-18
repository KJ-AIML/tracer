/**
 * Isolated prior-version SQLite helpers for upgrade fixtures (W2.4.1-A).
 * Uses Node built-in node:sqlite (Node >=22). Never touches operator Tracer app-data.
 */

import { DatabaseSync } from "node:sqlite";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  copyFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import path from "node:path";

export const FIXTURE_APP_ID = "dev.tracer.desktop.upgrade-fixture";
export const SCHEMA_V1 = "1";
export const SCHEMA_V2 = "2";

const SCHEMA_V1_DDL = `
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS projects (
    project_id   TEXT PRIMARY KEY NOT NULL,
    name         TEXT NOT NULL,
    root_path    TEXT NOT NULL,
    status       TEXT NOT NULL,
    is_git       INTEGER NOT NULL DEFAULT 0,
    last_opened_at TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_root_path ON projects(root_path);
CREATE TABLE IF NOT EXISTS sessions (
    session_id           TEXT PRIMARY KEY NOT NULL,
    project_id           TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
    title                TEXT,
    status               TEXT NOT NULL,
    runtime_kind         TEXT,
    runtime_session_id   TEXT,
    capabilities_json    TEXT,
    last_error_json      TEXT,
    active_agent_run_id  TEXT,
    next_sequence        INTEGER NOT NULL DEFAULT 1,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id);
CREATE TABLE IF NOT EXISTS events (
    event_id       TEXT PRIMARY KEY NOT NULL,
    session_id     TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
    project_id     TEXT NOT NULL,
    agent_run_id   TEXT,
    sequence       INTEGER NOT NULL,
    event_version  INTEGER NOT NULL,
    event_type     TEXT NOT NULL,
    severity       TEXT NOT NULL DEFAULT 'info',
    timestamp      TEXT NOT NULL,
    payload_json   TEXT NOT NULL,
    adapter_json   TEXT,
    envelope_json  TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_events_session_seq ON events(session_id, sequence);
CREATE TABLE IF NOT EXISTS storage_meta (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    checksum BLOB NOT NULL,
    execution_time BIGINT NOT NULL
);
CREATE TABLE IF NOT EXISTS approval_decisions (
    approval_id   TEXT PRIMARY KEY NOT NULL,
    session_id    TEXT NOT NULL,
    project_id    TEXT NOT NULL,
    decision      TEXT NOT NULL,
    created_at    TEXT NOT NULL
);
`;

function nowIso() {
  return new Date().toISOString();
}

function id(prefix) {
  return `${prefix}_${createHash("sha256")
    .update(`${prefix}-${Date.now()}-${Math.random()}`)
    .digest("hex")
    .slice(0, 16)}`;
}

export function dbFingerprint(dbPath) {
  if (!existsSync(dbPath)) return null;
  const buf = readFileSync(dbPath);
  return {
    pathHint: path.basename(path.dirname(dbPath)) + "/" + path.basename(dbPath),
    sizeBytes: buf.length,
    sha256: createHash("sha256").update(buf).digest("hex"),
  };
}

export function seedPriorSchemaV1(dbPath, { projectRoot } = {}) {
  mkdirSync(path.dirname(dbPath), { recursive: true });
  if (existsSync(dbPath)) {
    throw new Error(`refusing to overwrite existing DB: ${dbPath}`);
  }

  const db = new DatabaseSync(dbPath);
  db.exec(SCHEMA_V1_DDL);

  const t = nowIso();
  const projectId = id("proj");
  const sessionCompleted = id("sess");
  const sessionFailed = id("sess");
  const sessionExtra = id("sess");
  const root = projectRoot || path.join(path.dirname(dbPath), "fixture-project");

  db.prepare(`INSERT INTO storage_meta (key, value) VALUES (?, ?), (?, ?)`).run(
    "schema_logical_version",
    SCHEMA_V1,
    "writer_policy",
    "control_plane_only",
  );

  db.prepare(
    `INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
     VALUES (1, 'init', 1, ?, 1)`,
  ).run(Buffer.from("w2-4-1-a-fixture-schema-v1"));

  db.prepare(
    `INSERT INTO projects (project_id, name, root_path, status, is_git, last_opened_at, created_at, updated_at)
     VALUES (?, ?, ?, 'ready', 1, ?, ?, ?)`,
  ).run(projectId, "upgrade-fixture", root, t, t, t);

  const insertSession = db.prepare(
    `INSERT INTO sessions (
      session_id, project_id, title, status, runtime_kind, capabilities_json,
      last_error_json, active_agent_run_id, next_sequence, created_at, updated_at
    ) VALUES (?, ?, ?, ?, 'fake-acp', ?, ?, NULL, ?, ?, ?)`,
  );

  insertSession.run(
    sessionCompleted, projectId, "streamed-complete", "completed",
    JSON.stringify({ promptStreaming: true }), null, 8, t, t,
  );
  insertSession.run(
    sessionFailed, projectId, "cancelled-or-failed", "failed",
    JSON.stringify({ promptStreaming: true }),
    JSON.stringify({ message: "fixture cancelled/failed session" }), 4, t, t,
  );
  insertSession.run(
    sessionExtra, projectId, "history-anchor", "stopped",
    JSON.stringify({ promptStreaming: true }), null, 3, t, t,
  );

  const insertEvent = db.prepare(
    `INSERT INTO events (
      event_id, session_id, project_id, agent_run_id, sequence, event_version,
      event_type, severity, timestamp, payload_json, adapter_json, envelope_json
    ) VALUES (?, ?, ?, NULL, ?, 1, ?, 'info', ?, ?, NULL, ?)`,
  );

  const addEvents = (sessionId, types) => {
    types.forEach((eventType, i) => {
      const eventId = id("evt");
      const seq = i + 1;
      const payload = JSON.stringify({ fixture: true, eventType, seq });
      const envelope = JSON.stringify({
        eventVersion: 1, eventId, sequence: seq, timestamp: t,
        projectId, sessionId, eventType, payload: JSON.parse(payload),
      });
      insertEvent.run(eventId, sessionId, projectId, seq, eventType, t, payload, envelope);
    });
  };

  addEvents(sessionCompleted, [
    "session.created", "runtime.process.started", "runtime.process.ready",
    "session.ready", "session.prompt.submitted", "agent.message.delta",
    "agent.message.delta", "session.completed",
  ]);
  addEvents(sessionFailed, [
    "session.created", "session.ready", "session.prompt.submitted", "session.failed",
  ]);
  addEvents(sessionExtra, ["session.created", "session.ready", "session.stopped"]);

  db.prepare(
    `INSERT INTO approval_decisions (approval_id, session_id, project_id, decision, created_at)
     VALUES (?, ?, ?, 'approved', ?)`,
  ).run(id("appr"), sessionCompleted, projectId, t);

  db.close();
  return captureState(dbPath);
}

export function captureState(dbPath) {
  if (!existsSync(dbPath)) {
    return { ok: false, detail: "db missing", dbPath };
  }
  const db = new DatabaseSync(dbPath, { readOnly: true });
  const schema = db
    .prepare(`SELECT value FROM storage_meta WHERE key = 'schema_logical_version'`)
    .get()?.value;
  const sessions = db
    .prepare(
      `SELECT session_id, title, status, next_sequence FROM sessions ORDER BY created_at, session_id`,
    )
    .all();
  const eventCounts = db
    .prepare(
      `SELECT session_id, COUNT(*) AS n FROM events GROUP BY session_id`,
    )
    .all();
  const orderedSummaries = sessions.map((s) => {
    const ev = eventCounts.find((e) => e.session_id === s.session_id);
    const types = db
      .prepare(
        `SELECT event_type FROM events WHERE session_id = ? ORDER BY sequence`,
      )
      .all(s.session_id)
      .map((r) => r.event_type);
    return {
      sessionId: s.session_id,
      title: s.title,
      status: s.status,
      nextSequence: s.next_sequence,
      eventCount: ev ? Number(ev.n) : 0,
      eventTypesOrdered: types,
    };
  });
  let approvalCount = 0;
  try {
    approvalCount = Number(
      db.prepare(`SELECT COUNT(*) AS n FROM approval_decisions`).get()?.n || 0,
    );
  } catch {
    approvalCount = 0;
  }
  db.close();

  return {
    ok: true,
    schemaLogicalVersion: schema,
    sessionCount: sessions.length,
    sessionIds: sessions.map((s) => s.session_id),
    terminalStates: sessions.map((s) => ({
      sessionId: s.session_id,
      status: s.status,
    })),
    orderedSummaries,
    approvalCount,
    fingerprint: dbFingerprint(dbPath),
  };
}

export function setSchemaLogicalVersion(dbPath, version) {
  const db = new DatabaseSync(dbPath);
  db.prepare(
    `INSERT INTO storage_meta (key, value) VALUES ('schema_logical_version', ?)
     ON CONFLICT(key) DO UPDATE SET value = excluded.value`,
  ).run(String(version));
  db.close();
}

export function corruptDatabase(dbPath) {
  writeFileSync(dbPath, Buffer.from("NOT_A_SQLITE_DATABASE_CORRUPT_FIXTURE"));
}

export function backupDatabase(dbPath, destPath) {
  mkdirSync(path.dirname(destPath), { recursive: true });
  copyFileSync(dbPath, destPath);
  return destPath;
}

export function assertDataPreserved(pre, post) {
  const errors = [];
  if (!pre?.ok || !post?.ok) {
    errors.push("pre/post state not ok");
    return { ok: false, errors };
  }
  if (post.sessionCount < pre.sessionCount) {
    errors.push(`session count dropped ${pre.sessionCount} → ${post.sessionCount}`);
  }
  for (const sid of pre.sessionIds) {
    if (!post.sessionIds.includes(sid)) {
      errors.push(`missing session after upgrade: ${sid}`);
    }
  }
  const dupes = post.sessionIds.filter((sid, i, arr) => arr.indexOf(sid) !== i);
  if (dupes.length) errors.push(`duplicate session ids: ${dupes.join(",")}`);
  for (const preS of pre.orderedSummaries) {
    const postS = post.orderedSummaries.find((s) => s.sessionId === preS.sessionId);
    if (!postS) continue;
    if (postS.eventCount < preS.eventCount) {
      errors.push(
        `event count dropped for ${preS.sessionId}: ${preS.eventCount} → ${postS.eventCount}`,
      );
    }
    if (postS.status !== preS.status) {
      errors.push(
        `status changed for ${preS.sessionId}: ${preS.status} → ${postS.status}`,
      );
    }
  }
  return { ok: errors.length === 0, errors };
}


export function seedDataIntoExisting(dbPath, { projectRoot } = {}) {
  if (!existsSync(dbPath)) {
    throw new Error("seedDataIntoExisting requires existing DB: " + dbPath);
  }
  const db = new DatabaseSync(dbPath);
  const t = nowIso();
  const projectId = id("proj");
  const sessionCompleted = id("sess");
  const sessionFailed = id("sess");
  const sessionExtra = id("sess");
  const root = projectRoot || path.join(path.dirname(dbPath), "fixture-project");

  // Do NOT touch _sqlx_migrations — preserve product checksums.

  db.prepare(
    `INSERT INTO projects (project_id, name, root_path, status, is_git, last_opened_at, created_at, updated_at)
     VALUES (?, ?, ?, 'ready', 1, ?, ?, ?)`,
  ).run(projectId, "upgrade-fixture", root, t, t, t);

  const insertSession = db.prepare(
    `INSERT INTO sessions (
      session_id, project_id, title, status, runtime_kind, capabilities_json,
      last_error_json, active_agent_run_id, next_sequence, created_at, updated_at
    ) VALUES (?, ?, ?, ?, 'fake-acp', ?, ?, NULL, ?, ?, ?)`,
  );

  insertSession.run(sessionCompleted, projectId, "streamed-complete", "completed", JSON.stringify({ promptStreaming: true }), null, 8, t, t);
  insertSession.run(sessionFailed, projectId, "cancelled-or-failed", "failed", JSON.stringify({ promptStreaming: true }), JSON.stringify({ message: "fixture cancelled/failed session" }), 4, t, t);
  insertSession.run(sessionExtra, projectId, "history-anchor", "stopped", JSON.stringify({ promptStreaming: true }), null, 3, t, t);

  const insertEvent = db.prepare(
    `INSERT INTO events (
      event_id, session_id, project_id, agent_run_id, sequence, event_version,
      event_type, severity, timestamp, payload_json, adapter_json, envelope_json
    ) VALUES (?, ?, ?, NULL, ?, 1, ?, 'info', ?, ?, NULL, ?)`,
  );

  const addEvents = (sessionId, types) => {
    types.forEach((eventType, i) => {
      const eventId = id("evt");
      const seq = i + 1;
      const payload = JSON.stringify({ fixture: true, eventType, seq });
      const envelope = JSON.stringify({
        eventVersion: 1, eventId, sequence: seq, timestamp: t,
        projectId, sessionId, eventType, payload: JSON.parse(payload),
      });
      insertEvent.run(eventId, sessionId, projectId, seq, eventType, t, payload, envelope);
    });
  };

  addEvents(sessionCompleted, [
    "session.created", "runtime.process.started", "runtime.process.ready",
    "session.ready", "session.prompt.submitted", "agent.message.delta",
    "agent.message.delta", "session.completed",
  ]);
  addEvents(sessionFailed, [
    "session.created", "session.ready", "session.prompt.submitted", "session.failed",
  ]);
  addEvents(sessionExtra, ["session.created", "session.ready", "session.stopped"]);

  // Approvals table (001): decision allow/deny + decided_at
  try {
    db.prepare(
      "INSERT INTO approval_decisions (approval_id, session_id, event_id, decision, decided_at, details_json) VALUES (?, ?, NULL, 'allow', ?, '{}')"
    ).run(id("appr"), sessionCompleted, t);
  } catch {
    /* optional */
  }

  db.close();
  return captureState(dbPath);
}