# WebView infrastructure E2E (L3-I)

**Task:** `tracer-w2-tauri-e2e-infrastructure` (W2.2-A)  
**Level:** **L3-I** — driver infrastructure interaction  
**Not:** L3-J full GUI product journey

## What this proves

When `tauri-driver` + native WebDriver are installed:

1. Driver process starts and answers `/status`
2. WebDriver `new session` launches the built Tracer binary
3. Basic WebView probes: title, `#root` presence, `__TAURI__.core.invoke` detect
4. Optional `tracer_app_info` invoke (infrastructure only)
5. Session delete + driver stop + orphan verification

When tooling is missing → **`BLOCKED_BY_TOOLING`** (honest; not PASS, not FAIL).

## Run

```powershell
node tools/tauri-e2e/doctor.mjs
node tools/tauri-e2e/l3i-infra.mjs
node tools/tauri-e2e/l3i-infra.mjs --json
```

Implementation lives in `tools/tauri-e2e/l3i-infra.mjs` (process safety + WebDriver client).  
Driver helpers: `tools/tauri-driver/`.

## CI class

`windows_gui_runner` | `platform_gated_ci` | `manual_local`  
Not standard headless CI unless the runner image has WebView2 + Edge Driver + tauri-driver.
