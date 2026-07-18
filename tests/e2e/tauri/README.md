# Tauri E2E (W2-B)

**Task:** `tracer-w2-tauri-gui-e2e`  
**Classification:** desktop-boundary E2E (+ frontend invoke policy). **Not** full WebView GUI E2E.

## Location of executable suites

| Suite | Path |
|---|---|
| Desktop boundary journey (Rust) | `apps/desktop/src-tauri/tests/desktop_boundary_journey.rs` |
| Invoke policy (TypeScript) | `apps/desktop/src/shared/commands/invoke.policy.test.ts` |
| Orchestrator | `tools/tauri-e2e/run.mjs` |

## Run

```powershell
# Full W2-B harness (policy + boundary + classification report)
node tools/tauri-e2e/run.mjs

# Policy only
pnpm --filter @tracer/desktop exec vitest run src/shared/commands/invoke.policy.test.ts

# Boundary only
cargo test -p tracer-desktop --test desktop_boundary_journey -- --test-threads=1
```

## Assertions covered (where technically supported)

See `docs/modules/w2-b/W2_B_TEST_MATRIX.md`.

## Explicit non-claim

This folder does **not** currently host Playwright/WebDriver scripts that click the real WebView. That path is documented as a follow-up in `docs/modules/w2-b/W2_B_E2E_ARCHITECTURE.md`.

Gate 2.1 integration must re-validate presentation snapshot fields after W2-A presentation delivery lands.
