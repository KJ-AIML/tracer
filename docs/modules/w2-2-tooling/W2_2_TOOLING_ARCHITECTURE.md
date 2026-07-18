# W2.2-T Architecture — WebView Tooling Enablement

**Task:** `tracer-w2-webview-tooling`  
**Work item:** W2.2-T (Gate 2.2.2)  
**Branch:** `agent/tracer-w2-webview-tooling`  
**Scope (owned):**  
`tools/tauri-driver/`, `tools/tauri-e2e/`, `tests/e2e/webview-infrastructure/`,  
`docs/modules/w2-2-tooling/`, `docs/validation/tauri/`,  
`docs/integration/WAVE_2_2_2_TOOLING_REPORT.md`,  
`docs/integration/W2_2_B_LAUNCH_AUTHORIZATION.md`,  
root `package.json` (minimal script aliases only)

**Forbidden:** control-plane / domain / storage redesign; desktop product redesign;  
W2.2-B full GUI product journey; live Grok in CI; committing driver binaries

## 1. Purpose

Close the Gate 2.2.1 gap: **provision and prove** the external WebView driver stack so L3-I can go green on an authorized Windows GUI host, without claiming L3-J product journeys.

| Delivered | Not delivered |
|---|---|
| Opt-in plan/apply driver setup | Auto-install in product CI without policy |
| Edge major ↔ msedgedriver compatibility rule | Committing binaries |
| Doctor component matrix + READY when stack present | Desktop UX redesign |
| L2 PASS + L3-I PASS on authoring host | L3-J session/prompt/approval DOM journey |
| Precise failure classes + CI isolation | Creating/claiming W2.2-B task |

Extends W2.2-A harness; does not replace L0/L1.

## 2. Level model (do not collapse)

```text
L3-J Full GUI product journey     ← NOT_STARTED (future W2.2-B only after launch auth)
L3-I WebView driver infrastructure ← W2.2-T goal: PASS when drivers + build present
L2   Built/packaged app launch smoke ← retained; must stay PASS
L1   Backend command-boundary      ← Gate 2.1 / standard CI
L0   Frontend invoke/mock policy   ← Gate 2.1 / standard CI
```

**Claim rules**

- Claim **Doctor READY** only when critical components OK (optional tauri-cli excluded).
- Claim **L3-I PASS** only with executable evidence: driver start → session → WebView probes → teardown → no orphans.
- Missing tooling → `BLOCKED_BY_TOOLING` / `DRIVER_UNAVAILABLE` — never false PASS.

## 3. Component architecture

```text
┌────────────────────────────────────────────────────────────────────┐
│ tools/tauri-driver/setup.mjs     plan (default) | apply (opt-in)   │
│ tools/tauri-driver/lib/edge.mjs  Edge detect + msedgedriver download│
│ tools/tauri-driver/lib/install.mjs cargo install tauri-driver      │
│ tools/tauri-driver/lib/paths.mjs project cache + path redaction    │
└───────────────────────────────┬────────────────────────────────────┘
                                │ used by
┌───────────────────────────────▼────────────────────────────────────┐
│ tools/tauri-e2e/doctor.mjs     discovery + classification + apply  │
│ tools/tauri-e2e/l2-smoke.mjs   process launch smoke                │
│ tools/tauri-e2e/l3i-infra.mjs  WebDriver infrastructure smoke      │
│ tools/tauri-e2e/lib/*          classify · discover · process · WD  │
└────────────────────────────────────────────────────────────────────┘
```

### 3.1 Setup modes

| Mode | Trigger | Behavior |
|---|---|---|
| **plan** | default | Inventory + planned actions; no install/download |
| **apply** | `--apply` or `TRACER_TAURI_E2E_SETUP=1` | `cargo install tauri-driver --locked`; download matching msedgedriver to **gitignored** project cache |

Apply never permanently rewrites system PATH. Prefer:

- project `tools/tauri-driver/bin/` + `.cache/`
- user cargo bin
- process-local PATH prepend for harness (discovery already searches cargo bin + project paths)

### 3.2 Compatibility rule (explicit)

```text
major(msedgedriver) MUST equal major(installed Microsoft Edge)
```

- Prefer exact full-version match when the Microsoft endpoint serves it.
- File existence alone is **insufficient** — `--version` must parse.
- Codes: `EDGE_DRIVER_COMPATIBLE` | `EDGE_DRIVER_NOT_FOUND` | `EDGE_DRIVER_VERSION_MISMATCH` | `EDGE_DRIVER_VERSION_UNVERIFIED` | `EDGE_BROWSER_VERSION_UNKNOWN`

### 3.3 Doctor components

| Component id | Meaning |
|---|---|
| `TAURI_DRIVER` | tauri-driver binary resolve + version |
| `EDGE_BROWSER` | Microsoft Edge (Windows) |
| `WEBVIEW2_RUNTIME` | WebView2 Evergreen runtime |
| `EDGE_DRIVER` | msedgedriver (Windows) / WebKitWebDriver (Linux) |
| `APPLICATION_BINARY` | `tracer-desktop` cargo artifact |
| `FRONTEND_DIST` | `apps/desktop/dist` |
| `PORT_AVAILABILITY` | default `127.0.0.1:4444` |
| `PROCESS_CLEANUP_CAPABILITY` | taskkill/tasklist or kill |

Statuses: `OK | MISSING | MISMATCH | UNVERIFIED | IN_USE | UNKNOWN | N/A`

### 3.4 L3-I probes (infrastructure only)

1. Driver `/status` ready  
2. WebDriver new session with `tauri:options.application`  
3. Title + `document.readyState`  
4. Root marker `#root` or `body`  
5. Tauri IPC surface: `__TAURI_INTERNALS__` (Tauri 2 default) and/or public `__TAURI__.core.invoke`  
6. Session delete, driver stop, orphan verify  

**Does not** walk product UX (projects → session → prompt → approval). That is L3-J.

Note: product may not set `withGlobalTauri`; public `__TAURI__` can be absent while `__TAURI_INTERNALS__` proves the bridge. L3-I accepts either surface.

## 4. Pipeline stages

Shared stage vocabulary with W2.2-A:

```text
frontend_build → backend_build → packaging_test_binary
  → driver_startup → app_launch → readiness → smoke
  → app_shutdown → driver_shutdown → orphan_verification
```

Suite results:  
`PASS | PARTIAL | BLOCKED_BY_TOOLING | BLOCKED_BY_WEBVIEW | UNSUPPORTED_PLATFORM | FAIL`

## 5. Process safety

| Concern | Mechanism |
|---|---|
| Ownership | `spawnOwned` registry + exit hooks |
| Logs | Per-run temp dir stdout/stderr |
| Tree kill | Windows `taskkill /T /F` |
| Orphans | `findOrphans` / `reapOrphans` for app + drivers |
| Never leave app running | `stopAllOwned` + process exit hook |
| Path redaction | `%USERPROFILE%` / `<user>` in doctor reports |

## 6. CI isolation

| Surface | Runs |
|---|---|
| `pnpm -r test` / package `test` | L0+L1 via `tools/tauri-e2e/run.mjs` only |
| `pnpm test:tauri-e2e:doctor` | Doctor (explicit) |
| `pnpm test:tauri-e2e:l2` | L2 (platform-gated) |
| `pnpm test:tauri-e2e:l3i` | L3-I (platform-gated; **not** recursive test) |
| `pnpm test:tauri-e2e:setup` | Setup plan (apply only with env flag) |

L3-I is **not** part of cargo workspace tests.

## 7. Out of scope

- L3-J full GUI product journey (see `W2_2_B_LAUNCH_AUTHORIZATION.md`)  
- Enabling `withGlobalTauri` / product shell redesign  
- Control-plane drain, multi-session, storage redesign  
- Live Grok credentials in CI  
- Committing `.exe` / driver zips  

## 8. Key files

| Path | Role |
|---|---|
| `tools/tauri-driver/setup.mjs` | Plan/apply provisioning |
| `tools/tauri-driver/lib/edge.mjs` | Edge + msedgedriver |
| `tools/tauri-driver/lib/install.mjs` | tauri-driver cargo install |
| `tools/tauri-driver/lib/paths.mjs` | Cache paths + redaction |
| `tools/tauri-e2e/doctor.mjs` | Doctor CLI |
| `tools/tauri-e2e/l2-smoke.mjs` | L2 |
| `tools/tauri-e2e/l3i-infra.mjs` | L3-I |
| `tools/tauri-e2e/lib/discover.mjs` | Env + component matrix |
| `tools/tauri-e2e/lib/classify.mjs` | Vocabularies |
| Root `package.json` scripts | `test:tauri-e2e:{doctor,l2,l3i,setup}` |
