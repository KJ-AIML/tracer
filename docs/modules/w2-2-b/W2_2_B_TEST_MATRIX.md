# W2.2-B Test Matrix

## Suite isolation

| Suite | Command | In `pnpm -r test`? |
|---|---|---|
| Unit/package JS | `pnpm -r test` | yes |
| L0 invoke policy | via desktop vitest / `test:tauri-e2e --policy-only` | yes (package test) |
| L1 boundary | `cargo test -p tracer-desktop --test desktop_boundary_journey` | no GUI drivers |
| L2 | `pnpm test:tauri-e2e:l2` | **no** |
| L3-I | `pnpm test:tauri-e2e:l3i` | **no** |
| **L3-J** | **`pnpm test:tauri-e2e:gui`** | **no** |

## L3-J matrix

| ID | Scenario / fixture | Primary asserts | CI class |
|---|---|---|---|
| GJ-01 | cold start | Tauri backend marker | windows_gui_runner |
| GJ-02 | happy_prompt_stream create | session ready | windows_gui_runner |
| GJ-03 | happy_prompt_stream prompt | event list types | windows_gui_runner |
| GJ-04 | permission_allow | Allow clears card | windows_gui_runner |
| GJ-05 | permission_deny | Deny clears card | windows_gui_runner |
| GJ-06 | cancel_while_permission_pending | cancel completes | windows_gui_runner |
| GJ-07 | two sessions | focus switch DOM | windows_gui_runner |
| GJ-08 | crash_nonzero_exit | disconnect UX | windows_gui_runner |
| GJ-09 | reopen same DB | history events | windows_gui_runner |
| GJ-10 | empty heli probe | non-fatal | windows_gui_runner |
| GJ-11 | invalid register path | fail-closed tauri | windows_gui_runner |
| GJ-12 | teardown | orphan free | windows_gui_runner |

## Regression (must stay green)

```text
pnpm install --frozen-lockfile
pnpm -r test          # must NOT run L2/L3-I/L3-J
pnpm -r build
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
pnpm test:tauri-e2e:doctor
pnpm test:tauri-e2e:l2
pnpm test:tauri-e2e:l3i
pnpm test:tauri-e2e:gui
cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1
cargo test -p tracer-control-plane --test drain_lifecycle -- --test-threads=1
cargo test -p tracer-control-plane --test multi_session -- --test-threads=1
cargo test -p tracer-control-plane --test presentation_delivery
```

## Artifacts

Gitignored: `artifacts/tauri-e2e/<run-id>/` (page source, probe JSON on failure).
