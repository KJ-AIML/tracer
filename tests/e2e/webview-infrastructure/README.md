# WebView infrastructure E2E (L3-I)

**Tasks:** `tracer-w2-tauri-e2e-infrastructure` (W2.2-A) · `tracer-w2-webview-tooling` (W2.2-T)  
**Level:** **L3-I** — driver infrastructure interaction  
**Not:** L3-J full GUI product journey

## What this proves

When `tauri-driver` + compatible native WebDriver are provisioned (opt-in):

1. Driver process starts and answers `/status`
2. WebDriver `new session` launches the built Tracer binary
3. Basic WebView probes: title, `#root` presence, Tauri IPC surface
   (`__TAURI_INTERNALS__` and/or public `__TAURI__.core.invoke`)
4. Optional public-API shape check (not product journey invoke)
5. Session delete + driver stop + orphan verification

When tooling is missing → **`BLOCKED_BY_TOOLING`** (honest; not PASS, not FAIL).

## Setup (W2.2-T opt-in)

```powershell
# Plan only (no download/install)
node tools/tauri-driver/setup.mjs
pnpm test:tauri-e2e:doctor

# Apply (authorized this host): cargo install tauri-driver + project-local msedgedriver
$env:TRACER_TAURI_E2E_SETUP = "1"
node tools/tauri-driver/setup.mjs --apply
# or: pnpm test:tauri-e2e:doctor -- --apply
```

Compatibility rule: **`major(msedgedriver) == major(Microsoft Edge)`**.  
Binaries land under gitignored `tools/tauri-driver/.cache/` / `bin/` — **never commit**.

## Run

```powershell
pnpm test:tauri-e2e:doctor
pnpm test:tauri-e2e:l2
pnpm test:tauri-e2e:l3i
# equivalents:
node tools/tauri-e2e/doctor.mjs
node tools/tauri-e2e/l2-smoke.mjs
node tools/tauri-e2e/l3i-infra.mjs
node tools/tauri-e2e/l3i-infra.mjs --json
```

Implementation: `tools/tauri-e2e/l3i-infra.mjs`  
Driver helpers: `tools/tauri-driver/` (setup plan/apply, edge compat, install)

## CI class

`windows_gui_runner` | `platform_gated_ci` | `manual_local`  
**Not** part of `pnpm -r test` / standard CI. Use explicit `pnpm test:tauri-e2e:l3i`.

## L3-J

**NOT_STARTED** — full product GUI journey is future W2.2-B only after launch authorization.
