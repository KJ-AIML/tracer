# W1-E Storage Design

**Task:** `tracer-w1-storage`  
**Crate:** `crates/tracer-storage`  
**Migrations:** `apps/desktop/src-tauri/migrations/` (and embedded copy under the crate)

## Purpose

Provide the durable SQLite foundation for the first vertical slice:

- Project / Session / Event / RuntimeProcess / Approval / basic Artifact records
- Ordered event replay by monotonic per-session `sequence`
- Crash-safe transactional writes
- Schema migrations and version markers
- Storage error mapping to Tauri command `errorClass` values

## Writer policy (normative)

**The control plane is the sole planned writer of the primary Tracer SQLite database.**

| Component | DB role |
|---|---|
| Control plane (`tracer-control-plane` / Tauri backend) | **Only writer** + reader |
| Runtime sidecar (ACP process) | **Must not** open primary DB for writes (ADR-001, F-S05) |
| Desktop UI (React) | No direct SQL; uses Tauri commands |
| Storage crate | Library used by control plane; not a second process writer |

Schema seed: `storage_meta.writer_policy = control_plane_only`.

Secrets / auth tokens must **not** appear as application table columns. Authentication material stays outside these tables.

## Database path

```text
{platform_app_data_dir}/tracer/tracer.db
```

- `platform_app_data_dir` is supplied by the host (Tauri path API, tests use `tempfile`).
- The storage crate never hardcodes user home directories.

API: `tracer_storage::database_path(app_data_dir)`.

## Journal / crash safety

On open:

- `foreign_keys = ON`
- `journal_mode = WAL`
- `synchronous = NORMAL` (safe with WAL)

Multi-step mutations (e.g. append event + advance `next_sequence`) run in a **single SQL transaction**. Interrupted transactions roll back (F-S03); callers must not claim durable success if `append_event` / commit returns an error (F-S01).

## Schema overview (logical v1)

| Table | Role |
|---|---|
| `projects` | Registered local folders |
| `sessions` | Session status, runtime binding, `next_sequence` |
| `events` | Full normalized envelope + indexed columns; `UNIQUE(session_id, sequence)` |
| `runtime_processes` | Process diagnostics summary |
| `approval_decisions` | Approval audit |
| `artifacts` | Basic artifact/file-change summaries |
| `storage_meta` | Logical schema version, writer policy |

Migrations are sqlx-versioned SQL files. Re-running migrations is idempotent.

## Event ordering

- Control plane assigns `eventId`, `sequence`, and observation `timestamp` (protocol decision).
- Storage `append_event` / `append_events` assign the next monotonic `sequence` from `sessions.next_sequence`.
- Reads: `ORDER BY sequence ASC` with `after_sequence` + `limit` (maps to `tracer_events_list`).
- Unknown `type` values and unknown payload fields are preserved via JSON columns and full `envelope_json`.

## Session reload / F-S04

After app restart:

1. Re-open DB and load session + events (history is durable truth).
2. Call `SessionRepository::reconcile_stale_live_sessions(Disconnected|Stopped|Failed)` so statuses that imply a live process are corrected when no process exists.
3. UI must not show “running” without a live process.

## Error mapping

| Storage condition | `errorClass` |
|---|---|
| IO / SQLite failure | `StorageError` (retryable) |
| Migration failure | `StorageError` (refuse unsafe start) |
| Missing row | `NotFound` |
| Unique conflict | `AlreadyExists` |
| Bad args / sequence conflict | `InvalidArgument` |

## Repository surface

Other modules depend on traits / `SqliteStorage` methods, not raw SQL:

- `ProjectRepository`
- `SessionRepository`
- `EventRepository`
- `RuntimeProcessRepository`
- `ApprovalRepository`
- `ArtifactRepository`
- Plus `append_event` / `append_events` / `begin` for transactional control-plane flows

## Integration notes for W1-F

- Prefer path dependency on `tracer-storage` once root workspace is approved.
- Open DB at app start; fail closed on migration errors (F-S02).
- Run stale-session reconcile before serving session lists.
- Stream live events to UI; use `EventRepository::list` for reload.

## Out of scope (this agent)

- ACP protocol
- UI
- Runtime-as-DB-writer
- Root workspace `Cargo.toml` edits (request coordinator if needed)
- Full artifact store / multi-device sync
