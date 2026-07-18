# W2.2-A Environment Matrix

**Task:** `tracer-w2-tauri-e2e-infrastructure`  
**Doctor command:** `node tools/tauri-e2e/doctor.mjs`  
**Detail guide:** `docs/validation/tauri/TAURI_E2E_DOCTOR.md`

## 1. Discovery fields

| Field | Source |
|---|---|
| OS / arch / release | `process.platform`, `os.*` |
| Rust / Cargo | `rustc --version`, `cargo --version` |
| Node / pnpm | `node --version`, `pnpm --version` |
| Tauri CLI | local `@tauri-apps/cli` or `cargo-tauri` / `tauri` on PATH |
| WebView2 (Windows) | registry Evergreen client `pv` |
| WKWebView (macOS) | system (always “available” for Tauri host) |
| WebKitGTK (Linux) | heuristic / WebKitWebDriver presence |
| tauri-driver | `where` / `which` |
| msedgedriver | `where` / `which` / `TRACER_NATIVE_DRIVER` |
| WebKitWebDriver | `which` |
| App binary | `target/{debug,release}/tracer-desktop[.exe]`, `TRACER_E2E_APP_BINARY` |
| Frontend dist | `apps/desktop/dist/index.html` |
| Fake ACP | `tools/fake-acp-runtime/bin/fake-acp-runtime.js` |
| Ports | vite `1420`, driver `TRACER_TAURI_DRIVER_PORT` (default 4444) |
| Build profile | `TRACER_E2E_PROFILE` or discovered binary profile |

## 2. Host matrix (capability)

| Host | L0 | L1 | L2 | L3-I (external driver) | L3-J |
|---|---|---|---|---|---|
| Windows 10/11 + WebView2 + Rust/Node | ✓ | ✓ | ✓* | ✓** | deferred |
| Windows without WebView2 | ✓ | ✓ | blocked webview | blocked | deferred |
| Linux + WebKitGTK + Rust/Node | ✓ | ✓ | ✓* | ✓** | deferred |
| macOS + Rust/Node | ✓ | ✓ | ✓* (app process) | **unsupported** external driver | deferred (embedded WDIO future) |
| Headless standard CI (no GUI) | ✓ | ✓ | typically blocked / manual | blocked | deferred |

\* Requires successful binary build and GUI session for real window.  
\** Requires `tauri-driver` + matching native WebDriver on PATH.

## 3. Doctor classification → action

| Classification | Meaning | Strongest fallback |
|---|---|---|
| `READY` | L2 attemptable; L3-I if drivers present | Run full matrix as desired |
| `BUILD_REQUIRED` | Tools OK; artifacts missing | Build then re-doctor; L0/L1 still run |
| `DRIVER_UNAVAILABLE` | App path OK-ish; no WebDriver stack | L0+L1+L2 only |
| `MISSING_TOOL` | rust/node/pnpm/fake-acp gap | L0 only if node; else fix tools |
| `INCOMPATIBLE_VERSION` | e.g. Node &lt; 20 | Upgrade; unsupported otherwise |
| `WEBVIEW_UNAVAILABLE` | Windows WebView2 missing | L0+L1 only |
| `UNSUPPORTED_PLATFORM` | OS cannot host path | L0+L1; document |

## 4. Observed agent host (this worktree run)

Captured via doctor at implementation time (values may drift):

| Item | Value |
|---|---|
| OS | Windows NT 10 / AMD64 |
| Rust | rustc/cargo 1.96.0 |
| Node | v24.x |
| pnpm | 9.15.0 |
| WebView2 | present (Evergreen registry) |
| Tauri CLI | not installed in desktop package (optional) |
| tauri-driver | missing (DRIVER_UNAVAILABLE for L3-I) |
| msedgedriver | missing |
| App binary | build required until cargo build |
| Frontend dist | build required until Vite build |

**Expected doctor posture on clean checkout of this branch:**  
`BUILD_REQUIRED` + `DRIVER_UNAVAILABLE` (+ optional `MISSING_TOOL` for tauri CLI) — L0/L1 still attemptable.

## 5. Env hooks

| Variable | Levels |
|---|---|
| `TRACER_DATABASE_PATH` | L1, L2, L3-I |
| `TRACER_FAKE_ACP_JS` | L1, L2, L3-I |
| `TRACER_HELI_PROBE_PATH` | L1, L2, L3-I |
| `TRACER_NODE_BIN` | L1, L2, L3-I |
| `TRACER_E2E_PROFILE` | L2 |
| `TRACER_E2E_APP_BINARY` | L2, L3-I |
| `TRACER_TAURI_DRIVER_PORT` | L3-I |
| `TRACER_TAURI_DRIVER_HOST` | L3-I |
| `TRACER_NATIVE_DRIVER` | L3-I |

## 6. Network / secrets policy

| Concern | Policy |
|---|---|
| network | **no** for standard smoke |
| credentials | **no** |
| live Grok | **no** |
| fake ACP | optional pure process smoke; yes for L1 realism |
| temp SQLite | yes when needed |
