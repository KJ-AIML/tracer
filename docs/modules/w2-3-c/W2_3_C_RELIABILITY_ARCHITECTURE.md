# W2.3-C Reliability Architecture — GUI Harness + Windows Runner

**Task:** `tracer-w2-gui-reliability`  
**Work item:** W2.3-C  
**Branch:** `agent/tracer-w2-gui-reliability`  
**Owned surfaces:** `tools/tauri-e2e/` (except `live/`), `tests/e2e/tauri/gui/`, `tests/e2e/webview-journey/`, `docs/modules/w2-3-c/`, `docs/validation/tauri/`

## 1. Purpose

Make full L3-J product journeys **GJ-01…GJ-12** repeatable on a Windows GUI host without:

- product behavior changes that hide flakiness  
- unlimited retries masking flakes  
- live provider / credentials  
- port collisions, orphan processes, or unclean temp dirs  

Objective bar:

| Metric | Target |
|---|---|
| Consecutive fresh-env full suites | **≥ 5** first-attempt PASS |
| Product assertion failures | **0** |
| Orphans after teardown | **0** |
| Port collisions (failed bind / double-bind) | **0** |
| Temp cleanup failures on PASS | **0** |

## 2. Layering (unchanged product path)

```text
pnpm test:tauri-e2e:gui / repeat-gui
        │
        ▼
tools/tauri-e2e/l3j-gui.mjs
  free port → drivers → isolated env → WebDriver → GJ-01…12
        │
        ▼
apps/desktop product GUI (no harness plane_* for prompt/approval)
```

W2.3-C **does not** own product UI or live GUI (W2.3-B). It hardens the **harness**.

## 3. Reliability controls

### 3.1 Free port allocation (`lib/ports.mjs`)

1. Prefer `TRACER_TAURI_DRIVER_PORT` / default `4444` when free  
2. Scan upward on `EADDRINUSE`  
3. Ephemeral OS port last resort  
4. Never silently reuse a busy port  

`portCollisions` in reports = **failed collision outcomes** (should be 0). Avoidance events are tracked separately.

### 3.2 State-based waits (`lib/reliability.mjs` + `lib/gui.mjs`)

- Prefer `waitUntil` / DOM status / event markers with **timeouts**  
- Fixed delays only as backoff slices between polls (`backoff`, ≤5s)  
- Relaunch waits for desktop process exit before new session (SQLite lock)  
- Documented wait policy (`WAIT_POLICY`): expected state, mechanism, timeout, failure code  
- Suite records `driverStartupMs`, `appReadinessMs`, `suiteMs`, `shutdownMs`

### 3.3 Edge-update doctor resilience

- Compatibility rule: `major(msedgedriver) == major(Edge)`  
- Doctor reports mismatch + remediation (`doctor --apply` / setup apply)  
- No silent PASS when driver incompatible  
- Apply re-downloads project-local driver (opt-in network)

### 3.4 Failure injection (`inject-fail.mjs`, `TRACER_E2E_INJECT`)

Modes:

```text
none | orphan_leak | port_hold | artifact_secret | mid_journey_kill
app_launch_failure | tauri_driver_startup_failure | msedgedriver_startup_failure
root_marker_missing | fake_runtime_crash | sqlite_unavailable
forced_gui_assertion_failure | shutdown_timeout | stale_edge_driver
```

- Proves harness detect/reap/sanitize/honest-fail with **exact FailureCode**  
- **Never** re-runs product asserts with unlimited retries  
- Mid-journey kill / forced GUI assert → honest `FAIL`, `retries=0`  
- Stale Edge / driver startup → `BLOCKED_BY_TOOLING` (not product green)  
- Next suite always uses a fresh env (no state leak from inject)

### 3.5 Artifact sanitization (`lib/artifacts.mjs`)

Redacts Authorization/Bearer, api keys/tokens/passwords, user home paths.  
Audit helper fails if unsanitized secrets remain.

### 3.6 Temp cleanup + orphans

- `cleanupTempDir` reports cleaned/kept/error  
- PASS runs remove workDir (unless `TRACER_E2E_KEEP_TEMP=1`)  
- Fail runs keep workDir for diagnosis  
- Orphan verify uses shared `verifyNoOrphans` (detect + single reap + recheck)

### 3.7 First-attempt recording

- Suite meta: `attempt: "first"`, `retries: 0`  
- `repeat-gui.mjs` runs N independent fresh-env suites; no product retry loop  

## 4. Commands

```powershell
pnpm test:tauri-e2e:doctor
pnpm test:tauri-e2e:reliability      # unit self-test (no GUI)
pnpm test:tauri-e2e:inject-fail      # harness failure injection
pnpm test:tauri-e2e:gui              # single L3-J suite
pnpm test:tauri-e2e:repeat-gui       # default 5 consecutive first-attempt
pnpm test:tauri-e2e:repeat-gui -- --runs 5 --skip-build
```

## 5. CI class

`windows_gui_runner | platform_gated_ci | manual_local`

- Not in `pnpm -r test` / `cargo test --workspace`  
- No live provider, no credentials, fake ACP only  
- Prefer runner contract doc if platform-gated CI cannot be committed  

## 6. Forbidden

- Product assertion softening to green CI  
- Unlimited retries  
- Touching `tools/tauri-e2e/live/` (W2.3-B)  
- Packaging (W2.3-A) / CP redesign  

## 7. Related docs

- `W2_3_C_TEST_MATRIX.md`  
- `W2_3_C_COMPLETION_REPORT.md`  
- `docs/validation/tauri/GUI_RELIABILITY_RESULTS.md`  
- `docs/validation/tauri/WINDOWS_GUI_RUNNER_CONTRACT.md`  