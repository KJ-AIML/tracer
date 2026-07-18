# W2.2-T Setup Guide — WebView Driver Tooling

**Task:** `tracer-w2-webview-tooling`  
**Operator audience:** Windows GUI agent hosts (primary); Linux WebKit path secondary

## 1. Prerequisites

| Tool | Notes |
|---|---|
| Windows 10/11 x64 with GUI session | Required for L2/L3-I on this product host class |
| Rust (`rustc`/`cargo`) | `cargo install tauri-driver --locked` |
| Node.js ≥ 20 | Harness scripts |
| pnpm 9.15.0 | Workspace package manager |
| Microsoft Edge | Version major must match msedgedriver |
| WebView2 Runtime | Usually co-installed with Edge Evergreen |
| Built desktop app | `pnpm --filter @tracer/desktop build` then `cargo build -p tracer-desktop` |

## 2. Plan mode (safe default)

No install, no download:

```powershell
node tools/tauri-driver/setup.mjs
# or
pnpm test:tauri-e2e:setup
# or doctor (also plan by default)
pnpm test:tauri-e2e:doctor
node tools/tauri-e2e/doctor.mjs --json
```

Plan output lists:

- Edge version / major  
- tauri-driver presence  
- msedgedriver presence + **compatibility evaluation**  
- planned actions with exact reasons (`TAURI_DRIVER_NOT_FOUND`, `EDGE_DRIVER_VERSION_MISMATCH`, …)

## 3. Apply mode (opt-in only)

**Authorization required** (this task explicitly allows apply on the authoring host):

```powershell
$env:TRACER_TAURI_E2E_SETUP = "1"
node tools/tauri-driver/setup.mjs --apply
# or
node tools/tauri-e2e/doctor.mjs --apply
```

Flags:

| Flag / env | Effect |
|---|---|
| `--apply` | Enable apply |
| `TRACER_TAURI_E2E_SETUP=1` | Enable apply |
| `--skip-tauri-driver` | Do not cargo install |
| `--skip-edge-driver` | Do not download msedgedriver |
| `--json` | Machine-readable report |

### What apply does

1. **`cargo install tauri-driver --locked`** → user cargo bin  
2. Optional copy to `tools/tauri-driver/bin/` (gitignored)  
3. Detect Edge major; download matching **msedgedriver** zip from Microsoft endpoints  
4. Extract to `tools/tauri-driver/.cache/msedgedriver/` (gitignored)  
5. Record versions in `tools/tauri-driver/.cache/versions.local.json` (gitignored)

### What apply does **not** do

- Permanently rewrite system PATH  
- Commit binaries  
- Install without explicit opt-in  
- Enable live Grok or network for product tests (driver download is one-time tooling only)

## 4. Build app artifacts

```powershell
pnpm install
pnpm --filter @tracer/desktop build
cargo build -p tracer-desktop
# optional release:
# cargo build -p tracer-desktop --release
```

L2/L3-I resolve binaries under `target/{debug,release}/tracer-desktop.exe` (and `src-tauri/target/…` fallbacks).

## 5. Verify readiness

```powershell
pnpm test:tauri-e2e:doctor
# expect: Doctor classification: READY
# expect: L3-I attemptable=true
```

Optional JSON artifact:

```powershell
node tools/tauri-e2e/doctor.mjs --json --write-report
# writes docs/validation/tauri/WEBVIEW_DRIVER_READINESS_LAST.json (local; may be gitignored / not required)
```

## 6. Run levels

```powershell
# Standard CI (no drivers required)
pnpm test:tauri-e2e

# Platform-gated
pnpm test:tauri-e2e:l2
pnpm test:tauri-e2e:l3i
```

Skip rebuild when artifacts present:

```powershell
node tools/tauri-e2e/l2-smoke.mjs --skip-build
```

## 7. Environment overrides

| Variable | Purpose |
|---|---|
| `TRACER_TAURI_E2E_SETUP` | Authorize apply (`1` / `true`) |
| `TRACER_TAURI_DRIVER` | Absolute path to tauri-driver |
| `TRACER_NATIVE_DRIVER` | Absolute path to msedgedriver / WebKitWebDriver |
| `TRACER_TAURI_DRIVER_PORT` | Default `4444` |
| `TRACER_TAURI_DRIVER_HOST` | Default `127.0.0.1` |
| `TRACER_E2E_APP_BINARY` | Force app binary path |
| `TRACER_E2E_PROFILE` | Prefer debug/release discovery hint |
| `TRACER_EDGE_BINARY` | Override msedge.exe location |
| `TRACER_DATABASE_PATH` | Set by harness (temp SQLite) |
| `TRACER_FAKE_ACP_JS` | Set by harness |
| `TRACER_HELI_PROBE_PATH` | Set by harness |

## 8. Troubleshooting

| Symptom | Class / code | Fix |
|---|---|---|
| Doctor `DRIVER_UNAVAILABLE` | `TAURI_DRIVER_NOT_FOUND` | Apply setup or `cargo install tauri-driver --locked` |
| msedgedriver missing | `EDGE_DRIVER_NOT_FOUND` | Apply setup (project cache) |
| Major mismatch | `EDGE_DRIVER_VERSION_MISMATCH` | Re-apply download; delete stale cache |
| Version unparsed | `EDGE_DRIVER_VERSION_UNVERIFIED` | Corrupt binary — re-download |
| Port busy | `PORT_IN_USE` | Free 4444 or set `TRACER_TAURI_DRIVER_PORT` |
| No binary | `APP_BINARY_NOT_FOUND` | `cargo build -p tracer-desktop` |
| No dist | `FRONTEND_DIST_NOT_FOUND` | `pnpm --filter @tracer/desktop build` |
| L3-I orphans | `ORPHAN_PROCESS` | Investigate leftover processes; harness reaps best-effort |
| L3-I macOS | `UNSUPPORTED_PLATFORM` | External tauri-driver path unsupported; future embedded WDIO |

## 9. Never commit

```text
tools/tauri-driver/.cache/**
tools/tauri-driver/bin/**
*.exe (under tools/tauri-driver)
versions.local.json
```

Covered by `tools/tauri-driver/.gitignore`.
