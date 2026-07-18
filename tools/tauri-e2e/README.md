# W2-B Tauri E2E harness

**Classification:** `desktop-boundary-e2e` (not full WebView GUI E2E)

## What it runs

| Layer | Command | Proves |
|---|---|---|
| Frontend invoke policy | vitest `invoke.policy.test.ts` | Tauri detection; browser mock fallback; **no silent mock downgrade** |
| Desktop boundary journey | `cargo test -p tracer-desktop --test desktop_boundary_journey` | Same `build_control_plane` + `plane_*` handlers Tauri registers; fake ACP; temp SQLite; reopen history |
| GUI probe | report only | Documents WebDriver/tauri-driver blocker; **no false full-GUI claim** |

## Standard CI class

- network: **no**
- credentials: **no**
- live Grok: **no**
- provider: **no**
- fake ACP: **yes**
- temp file SQLite: **yes**

## Run

From repo root:

```powershell
node tools/tauri-e2e/run.mjs
node tools/tauri-e2e/run.mjs --policy-only
node tools/tauri-e2e/run.mjs --boundary-only
node tools/tauri-e2e/run.mjs --gui-probe
```

Or:

```powershell
pnpm --filter @tracer/tauri-e2e test
```

(Requires package workspace wiring; direct `node tools/tauri-e2e/run.mjs` always works.)

## E2E env hooks (desktop app)

| Variable | Purpose |
|---|---|
| `TRACER_DATABASE_PATH` | File SQLite path for persist/reopen journeys |
| `TRACER_FAKE_ACP_JS` | Path to `fake-acp-runtime.js` |
| `TRACER_HELI_PROBE_PATH` | Directory for Heli probe (empty → unavailable non-fatal) |
| `TRACER_NODE_BIN` | Node executable for fake ACP spawn |

## Preferred full GUI path (follow-up)

```text
launch built desktop app (TRACER_* env)
→ frontend loads
→ __TAURI__.core.invoke available
→ snapshot / session / prompt / stream / approval / cancel
→ close → reopen → history
```

Requires: `tauri-driver` (or WebDriver for WebView2) + Playwright/Selenium bindings. Not required for W2-B acceptance.

## Related docs

- `docs/modules/w2-b/W2_B_E2E_ARCHITECTURE.md`
- `docs/modules/w2-b/W2_B_TEST_MATRIX.md`
- `docs/modules/w2-b/W2_B_COMPLETION_REPORT.md`
