# Tauri GUI product journeys (L3-J / W2.2-B + W2.3-C reliability)

Executable entry (serial suite):

```powershell
pnpm test:tauri-e2e:gui
pnpm test:tauri-e2e:gui -- --journey GJ-03
pnpm test:tauri-e2e:gui -- --skip-build
node tools/tauri-e2e/l3j-gui.mjs --json

# Reliability (W2.3-C)
pnpm test:tauri-e2e:reliability
pnpm test:tauri-e2e:inject-fail
pnpm test:tauri-e2e:repeat-gui -- --runs 5 --skip-build
```

Implementation:

| Path | Role |
|---|---|
| `tools/tauri-e2e/l3j-gui.mjs` | Harness lifecycle + report |
| `tools/tauri-e2e/lib/journeys.mjs` | GJ-01…GJ-12 product flows |
| `tools/tauri-e2e/lib/gui.mjs` | WebDriver DOM helpers |
| `tools/tauri-e2e/lib/ports.mjs` | Free port allocation |
| `tools/tauri-e2e/lib/artifacts.mjs` | Sanitized failure artifacts |
| `tools/tauri-e2e/lib/reliability.mjs` | Waits, cleanup, edge probe, inject parse |
| `tools/tauri-e2e/repeat-gui.mjs` | Consecutive fresh-env first-attempt runs |
| `tools/tauri-e2e/inject-fail.mjs` | Harness failure injection |
| `artifacts/tauri-e2e/<run-id>/` | Gitignored failure artifacts |

Selector priority: role+name → form label → `data-testid="tracer-…"` → state markers.

Environment: real Tauri binary + WebDriver; **fake ACP only**; temp SQLite; no live Grok / network / credentials.

See `docs/modules/w2-3-c/` and `docs/validation/tauri/GUI_RELIABILITY_RESULTS.md`.