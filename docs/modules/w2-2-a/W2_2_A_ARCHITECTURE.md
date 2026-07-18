# W2.2-A Architecture — Tauri E2E Infrastructure

**Task:** `tracer-w2-tauri-e2e-infrastructure`  
**Work item:** W2.2-A  
**Branch:** `agent/tracer-w2-tauri-e2e-infrastructure`  
**Scope (owned):**  
`tools/tauri-e2e/`, `tools/tauri-driver/`, `tests/e2e/tauri/`, `tests/e2e/webview-infrastructure/`,  
`apps/desktop/src-tauri/tauri.conf.*`, `apps/desktop/capabilities/` (minimal),  
`docs/modules/w2-2-a/`, `docs/validation/tauri/`

**Forbidden:** L3-J product journey, control-plane drain redesign (W2.2-C), IDE/ALMS/plugins, live Grok in CI

## 1. Purpose

Build **deterministic, diagnosable infrastructure** for launching and driving the **actual Tauri application**:

| Delivered | Not delivered |
|---|---|
| Environment doctor + matrix | Full product GUI journey (L3-J) |
| Staged pipeline with failure classes | Control-plane redesign |
| L2 packaged/binary launch smoke | Session/prompt/approval DOM journey |
| L3-I WebView **driver infrastructure** interaction | Claiming “full GUI E2E” |
| Process safety (ownership, logs, orphans) | Auto live Grok |

Extends Gate 2.1 `tools/tauri-e2e/` (L0+L1 desktop-boundary) without collapsing levels.

## 2. Level model (do not collapse)

```text
L3-J Full GUI product journey     ← DEFERRED (future W2.2-B) — never claimed here
L3-I WebView driver infrastructure ← W2.2-A (when tauri-driver + native driver present)
L2   Built/packaged app launch smoke ← W2.2-A (GUI host / platform-gated)
L1   Backend command-boundary      ← Gate 2.1 executable (referenced, not re-owned)
L0   Frontend invoke/mock policy   ← Gate 2.1 executable (referenced, not re-owned)
```

**Claim rules**

- Claim **L2** only with executable evidence (process launch + shutdown + orphan check on a real binary).
- Claim **L3-I** only with executable evidence (driver start + WebDriver session + WebView probe + teardown).
- If tooling missing → `BLOCKED_BY_TOOLING` / `DRIVER_UNAVAILABLE` — **not** `PASS`, **not** silent skip-as-pass.

## 3. Pipeline stages

```text
frontend build
  → Tauri backend build
  → packaging / test binary resolve
  → driver startup          (L3-I only; skipped for L2)
  → app launch
  → readiness
  → smoke
  → app shutdown
  → driver shutdown         (L3-I only)
  → orphan verification
```

Each stage emits: `pass | fail | skip | partial | blocked_tooling | blocked_webview | unsupported`  
plus a suite-level result:

`PASS | PARTIAL | BLOCKED_BY_TOOLING | BLOCKED_BY_WEBVIEW | UNSUPPORTED_PLATFORM | FAIL`

## 4. Component layout

```text
┌────────────────────────────────────────────────────────────────────┐
│ tools/tauri-e2e/run.mjs     orchestrator (L0/L1 + --all / flags)   │
│ tools/tauri-e2e/doctor.mjs  environment discovery + classification │
│ tools/tauri-e2e/l2-smoke.mjs                                       │
│ tools/tauri-e2e/l3i-infra.mjs                                      │
└───────────────────────────────┬────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌────────────────────┐
│ lib/discover  │     │ lib/process     │     │ lib/webdriver      │
│ lib/classify  │     │ tree kill       │     │ minimal HTTP WD    │
│ lib/stages    │     │ orphan reap     │     │ (no WDIO dep)      │
└───────────────┘     └─────────────────┘     └────────────────────┘
        │
        ▼
┌────────────────────────────────────────────────────────────────────┐
│ tools/tauri-driver/  print-setup · doctor · start-driver           │
└────────────────────────────────────────────────────────────────────┘
```

### 4.1 Doctor

Detects and reports: OS, arch, Rust, Node, pnpm, Tauri CLI (optional), WebView2 / WebKit,  
`tauri-driver`, native WebDriver (`msedgedriver` / `WebKitWebDriver`), app binary path(s),  
frontend dist, build profile, ports, process-related readiness.

Doctor classes:  
`READY | MISSING_TOOL | INCOMPATIBLE_VERSION | WEBVIEW_UNAVAILABLE | DRIVER_UNAVAILABLE | BUILD_REQUIRED | UNSUPPORTED_PLATFORM`

### 4.2 L2 smoke (minimum checklist)

When tooling supports:

1. Build frontend + backend binary  
2. Launch app with `TRACER_*` env (temp SQLite, fake ACP path, empty Heli probe)  
3. WebView init best-effort (Windows main window handle)  
4. Frontend root / Tauri API / app info / snapshot — **null without driver** (honest PARTIAL)  
5. Clean exit + no orphans  

`bundle.active` remains `false` in `tauri.conf.json` — L2 uses the **cargo test/release binary artifact**, not an MSI installer. That is intentional (lighter, diagnosable).

### 4.3 L3-I infrastructure smoke

When `tauri-driver` + native driver exist:

1. Start driver on unique/local port (default 4444)  
2. WebDriver New Session with `tauri:options.application`  
3. Probes: title, `#root`, `__TAURI__.core.invoke`, optional `tracer_app_info`  
4. Delete session, stop driver, orphan verify  

**Does not** walk product UX (projects → session → prompt → approval). That is L3-J.

### 4.4 Driver safety

| Concern | Mechanism |
|---|---|
| Process ownership | `spawnOwned` registry + exit hooks |
| Logs | Per-run temp dir stdout/stderr capture |
| Timeouts | waitFor / WebDriver request timeouts |
| Tree kill | Windows `taskkill /T /F`; Unix group/SIGKILL |
| Unique temps/ports | `uniqueTempDir`, `TRACER_TAURI_DRIVER_PORT` |
| Orphans | `findOrphans` / `reapOrphans` for app + drivers |
| Never leave app running | `stopAllOwned` in finally + process `exit` hook |

## 5. Packaging / config posture

| Item | W2.2-A choice |
|---|---|
| `tauri.conf.json` `bundle.active` | left `false` — binary artifact smoke |
| Capabilities | no expansion required for external driver path |
| Root `package.json` | prefer tools-local scripts; document root alias `test:tauri-e2e:doctor` |
| Fake ACP | optional for pure process smoke; present for L1 / realistic L2 env |

## 6. CI classification

| Class | What runs |
|---|---|
| `standard_ci` | L0 + L1 only |
| `windows_gui_runner` | L2 ± L3-I |
| `platform_gated_ci` | L2/L3-I when image has WebView + drivers |
| `manual_local` | doctor + selective levels |
| `future_cross_platform` | embedded WDIO path (macOS), multi-OS matrix |

## 7. Relationship to Gate 2.1 (W2-B)

W2-B delivered executable **desktop-boundary** E2E (L0+L1) and documented L3 blocker.  
W2.2-A **does not replace** L0/L1; it adds the missing launch/driver infrastructure layers and keeps L3-J deferred.

## 8. Out of scope

- L3-J full GUI product journey  
- `session_runtime` drain lifecycle (W2.2-C)  
- IDE / editor / terminal / explorer / ALMS / plugins / collab / marketplace  
- Live Grok in standard CI  
- Root Cargo workspace redesign  

## 9. Key files

| Path | Role |
|---|---|
| `tools/tauri-e2e/doctor.mjs` | Doctor |
| `tools/tauri-e2e/l2-smoke.mjs` | L2 |
| `tools/tauri-e2e/l3i-infra.mjs` | L3-I |
| `tools/tauri-e2e/lib/*` | classify, discover, process, stages, webdriver |
| `tools/tauri-driver/*` | driver setup helpers |
| `tests/e2e/tauri/README.md` | entry |
| `tests/e2e/webview-infrastructure/README.md` | L3-I entry |
| `docs/validation/tauri/TAURI_E2E_DOCTOR.md` | operator doctor guide |
