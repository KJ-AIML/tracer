# tracer-heli fixtures

Deterministic on-disk HeliHarness trees used by `tests/status_fixtures.rs`.

| Fixture | Purpose |
|---|---|
| `minimal_workspace/` | Concurrent mode with 2 tasks, active lease, binding, sessions, target/index |
| `stale_lease_workspace/` | Expired write lease projection |
| `no_workspace/` | Missing-workspace safe path |

Do not point these fixtures at real machine absolute workspace paths. Synthetic `d:/fixture/...` worktree strings are intentional.
