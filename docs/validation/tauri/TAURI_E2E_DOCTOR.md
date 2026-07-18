# Tauri E2E Doctor

**Module:** W2.2-A  
**Command:**

```powershell
node tools/tauri-e2e/doctor.mjs
node tools/tauri-e2e/doctor.mjs --json
pnpm --filter @tracer/tauri-e2e doctor
```

**Root script name (integrator):** `pnpm test:tauri-e2e:doctor` → `node tools/tauri-e2e/doctor.mjs`

Driver-only: `node tools/tauri-driver/doctor.mjs`  
Setup print: `node tools/tauri-driver/print-setup.mjs`

## What it checks

1. OS / arch / platform support for L2 and L3-I  
2. Rust toolchain (`rustc`, `cargo`)  
3. Node ≥ 20 and pnpm  
4. Optional Tauri CLI  
5. WebView2 (Windows registry) / WebKit notes  
6. `tauri-driver` and native WebDriver (`msedgedriver` / `WebKitWebDriver`)  
7. Frontend dist presence  
8. `tracer-desktop` binary under `target/{debug,release}`  
9. Fake ACP runtime path  
10. Ports and E2E env hook names  

## Classifications

| Class | Meaning |
|---|---|
| `READY` | Host can attempt L2 (and L3-I if drivers present) |
| `MISSING_TOOL` | Required CLI/runtime missing |
| `INCOMPATIBLE_VERSION` | Version too old (e.g. Node &lt; 20) |
| `WEBVIEW_UNAVAILABLE` | WebView2/runtime missing |
| `DRIVER_UNAVAILABLE` | tauri-driver or native driver missing |
| `BUILD_REQUIRED` | Tools OK; need Vite/cargo artifacts |
| `UNSUPPORTED_PLATFORM` | Platform cannot host the requested path |

## Suite results (runners)

| Result | Meaning |
|---|---|
| `PASS` | Level completed |
| `PARTIAL` | Core path OK; some probes incomplete |
| `BLOCKED_BY_TOOLING` | Missing tools — not a product regression |
| `BLOCKED_BY_WEBVIEW` | WebView runtime blocked |
| `UNSUPPORTED_PLATFORM` | OS path unsupported |
| `FAIL` | Unexpected failure with tools present |

**Never treat BLOCKED_* as PASS.**

## Setup recipes

### Windows L2

```powershell
# WebView2 Evergreen if missing
# https://developer.microsoft.com/microsoft-edge/webview2/

pnpm --filter @tracer/desktop build
cargo build -p tracer-desktop
node tools/tauri-e2e/l2-smoke.mjs --skip-build
```

### Windows L3-I

```powershell
cargo install tauri-driver --locked
# Install msedgedriver matching Edge version; add to PATH
# or: setx TRACER_NATIVE_DRIVER "C:\path\msedgedriver.exe"

node tools/tauri-e2e/doctor.mjs
node tools/tauri-e2e/l3i-infra.mjs
```

### Linux L3-I

```powershell
cargo install tauri-driver --locked
# install WebKitWebDriver (distro package)
node tools/tauri-e2e/l3i-infra.mjs
```

### macOS

- L0/L1/L2 process smoke: OK when tools present  
- L3-I **external** `tauri-driver`: `UNSUPPORTED_PLATFORM`  
- Future: embedded WebdriverIO Tauri service (not wired in W2.2-A)

## Exit codes (doctor)

| Code | Meaning |
|---|---|
| 0 | READY, or advisory BUILD_REQUIRED/DRIVER_UNAVAILABLE with L0/L1 still attemptable |
| 2 | Hard block (unsupported / critical missing tools / webview when nothing runnable) |
| 1 | Unexpected doctor error |

## CI guidance

| Class | Doctor expectation |
|---|---|
| standard_ci | L0/L1 tools; ignore DRIVER_UNAVAILABLE |
| windows_gui_runner | WebView2 + optional drivers |
| platform_gated_ci | Same as GUI runner when labeled |
| manual_local | Full doctor before L2/L3-I |

## Related

- `docs/modules/w2-2-a/W2_2_A_ARCHITECTURE.md`
- `docs/modules/w2-2-a/W2_2_A_ENVIRONMENT_MATRIX.md`
- `docs/modules/w2-2-a/W2_2_A_TEST_MATRIX.md`
