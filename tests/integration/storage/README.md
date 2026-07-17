# Storage integration tests

Primary executable tests live in the `tracer-storage` crate so they share one
Cargo target directory and stay green without a root workspace:

```bash
cd crates/tracer-storage
cargo test
```

Notable cases:

| Test | Covers |
|---|---|
| `storage_foundation::*` | Fresh DB, migration rerun, ordering, unknown payloads, interrupted write, reload, F-S04 reconcile, no secrets columns |
| `vs10_persistence_reload::*` | VS-10 persistence + reload evidence |

This package (`tests/integration/storage`) mirrors the VS-10 case for path
ownership under the master plan. Prefer running via the crate above until a
root workspace unifies build graphs:

```bash
cd tests/integration/storage
# optional; reuses crate target when set:
# set CARGO_TARGET_DIR=../../../crates/tracer-storage/target
cargo test
```
