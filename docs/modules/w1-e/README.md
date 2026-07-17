# W1-E — Storage and Session Persistence

| Field | Value |
|---|---|
| Task ID | `tracer-w1-storage` |
| Branch | `agent/tracer-w1-storage` |
| Owned paths | `crates/tracer-storage/`, `apps/desktop/src-tauri/migrations/`, `tests/integration/storage/`, `docs/modules/w1-e/` |

## Docs

- [STORAGE_DESIGN.md](./STORAGE_DESIGN.md) — schema, writer policy, ordering, recovery

## Build & test

The crate is standalone until a root workspace manifest is added (root manifests are coordinator-owned).

```bash
cd crates/tracer-storage
cargo test

cd ../../tests/integration/storage
cargo test
```

## Key APIs

```rust
use tracer_storage::{
    database_path, open_database, OpenOptions, SqliteStorage,
    ProjectRepository, SessionRepository, EventRepository,
    SessionStatus,
};

let path = database_path(app_data_dir);
let pool = open_database(&path, OpenOptions::default()).await?;
let store = SqliteStorage::new(pool);

// sole writer: control plane
store.append_event(event).await?;
let events = EventRepository::list(&store, &session_id, 0, 200).await?;

// after restart
SessionRepository::reconcile_stale_live_sessions(&store, SessionStatus::Disconnected).await?;
```
