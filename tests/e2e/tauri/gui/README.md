# Tauri GUI product journeys (L3-J / W2.2-B)

Executable entry (serial suite):

```powershell
pnpm test:tauri-e2e:gui
pnpm test:tauri-e2e:gui -- --journey GJ-03
pnpm test:tauri-e2e:gui -- --skip-build
node tools/tauri-e2e/l3j-gui.mjs --json
```

Implementation:

| Path | Role |
|---|---|
| `tools/tauri-e2e/l3j-gui.mjs` | Harness lifecycle + report |
| `tools/tauri-e2e/lib/journeys.mjs` | GJ-01…GJ-12 product flows |
| `tools/tauri-e2e/lib/gui.mjs` | WebDriver DOM helpers |
| `artifacts/tauri-e2e/<run-id>/` | Gitignored failure artifacts |

Selector priority: role+name → form label → `data-testid="tracer-…"` → state markers.

Environment: real Tauri binary + WebDriver; **fake ACP only**; temp SQLite; no live Grok / network / credentials.

See `docs/modules/w2-2-b/` and `docs/validation/tauri/FULL_GUI_JOURNEY_RESULTS.md`.
