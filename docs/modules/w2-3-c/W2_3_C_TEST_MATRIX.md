# W2.3-C Test Matrix — GUI Reliability

## Suite isolation

| Suite | Command | In `pnpm -r test`? |
|---|---|---|
| Reliability self-test | `pnpm test:tauri-e2e:reliability` | no (explicit) |
| Failure injection | `pnpm test:tauri-e2e:inject-fail` | no |
| L3-J single | `pnpm test:tauri-e2e:gui` | **no** |
| L3-J repeat (5+) | `pnpm test:tauri-e2e:repeat-gui` | **no** |
| Doctor | `pnpm test:tauri-e2e:doctor` | no |

## Reliability self-test matrix

| ID | Check | Assert |
|---|---|---|
| R-01 | sanitize bearer/api_key/password | redacted |
| R-02 | sanitize user paths | `[USER]` |
| R-03 | artifact audit clean | ok after sanitize |
| R-04 | artifact audit detects leak | not ok on raw |
| R-05 | port allocate preferred | port > 0 |
| R-06 | port probe in-use | available=false |
| R-07 | port allocate avoids collision | different free port |
| R-08 | inject parse | safe defaults |
| R-09 | inject classification (app launch / stale edge) | exact FailureCode + retries=0 |
| R-10 | wait policy documented | timeouts + mechanisms present |
| R-11 | edge probe | runs; remediation when mismatch |
| R-12 | temp cleanup | dir removed |
| R-13 | product assert counter | FAIL + PRODUCT_GAP only |

## Failure injection matrix (C6)

| Mode | Expected result | FailureCode | Cleanup |
|---|---|---|---|
| `artifact_secret` | PASS (harness) | — | sanitize + audit |
| `port_hold` | PASS (avoid) | PORT_IN_USE avoided | release holder |
| `orphan_leak` | PARTIAL / reap | ORPHAN_PROCESS | tree kill |
| `mid_journey_kill` | FAIL | DRIVER_STARTUP_FAILED | yes |
| `app_launch_failure` | FAIL | APP_LAUNCH_FAILED | yes |
| `tauri_driver_startup_failure` | BLOCKED_BY_TOOLING | DRIVER_STARTUP_FAILED | yes |
| `msedgedriver_startup_failure` | BLOCKED_BY_TOOLING | MSEDGEDRIVER_STARTUP_FAILED | yes |
| `root_marker_missing` | FAIL | ROOT_MARKER_MISSING | yes |
| `fake_runtime_crash` | FAIL | FAKE_RUNTIME_CRASH | yes |
| `sqlite_unavailable` | FAIL | SQLITE_UNAVAILABLE | yes |
| `forced_gui_assertion_failure` | FAIL | GUI_ASSERTION_FAILED | yes |
| `shutdown_timeout` | FAIL | SHUTDOWN_TIMEOUT | yes |
| `stale_edge_driver` | BLOCKED_BY_TOOLING | EDGE_DRIVER_VERSION_MISMATCH | n/a |

All modes: `retries=0`, sanitized artifacts, next fresh-env run allowed (no unlimited product retry).

## Timing / wait policy (C5)

| Wait | Expected state | Mechanism | Timeout | Failure |
|---|---|---|---|---|
| Driver ready | tauri-driver `/status` | `waitDriverReady` poll | 30s | DRIVER_STARTUP_FAILED |
| App ready | `[data-testid=tracer-app-ready]` | `waitAppReady` | 60s | ROOT_MARKER_MISSING |
| Desktop exit | no desktop orphans | `waitUntil` + reap | 15s | ORPHAN_PROCESS |
| Session status | `data-session-status` match | poll + refresh | 60s | GUI_ASSERTION_FAILED |
| Shutdown | session deleted; driver PID dead | `waitUntil(!processAlive)` | 30s | SHUTDOWN_TIMEOUT |

Fixed delays are backoff slices between polls only (≤5s via `backoff`).

## L3-J repeatability matrix

| Run | Fresh env | First attempt | Journeys | Orphans | Ports | Temp |
|---|---|---|---|---|---|---|
| 1…N | new workDir + SQLite + port preference | yes | GJ-01…12 | 0 | 0 failed collisions | cleaned on PASS |

Per-run timing recorded: driverStartupMs, appReadinessMs, suiteMs, shutdownMs.

## Regression (must stay green)

```text
pnpm test:tauri-e2e:reliability
pnpm test:tauri-e2e:inject-fail
pnpm test:tauri-e2e:doctor
pnpm test:tauri-e2e:gui -- --skip-build   # when binary+drivers ready
pnpm test:tauri-e2e:repeat-gui -- --runs 5 --skip-build
```

Standard CI (L0/L1, cargo workspace) remains independent and must not pull L3-J.
