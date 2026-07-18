# Tauri E2E entry

| Level | Command | Notes |
|---|---|---|
| L0+L1 | `pnpm test:tauri-e2e` / `node tools/tauri-e2e/run.mjs` | Standard CI |
| Doctor | `pnpm test:tauri-e2e:doctor` | Host readiness |
| L2 | `pnpm test:tauri-e2e:l2` | Process smoke |
| L3-I | `pnpm test:tauri-e2e:l3i` | WebDriver infra only |
| **L3-J** | **`pnpm test:tauri-e2e:gui`** | Full product GUI journeys (W2.2-B) |

L2 / L3-I / L3-J are **not** part of `pnpm -r test`.

GUI journeys: [`gui/README.md`](./gui/README.md).
