# VS1-H2 Completion Report — Desktop Snapshot Wiring

**Task:** `tracer-vs1-desktop-wiring`  
**Branch:** `agent/tracer-vs1-desktop-wiring`  
**Worktree:** `repos/worktrees/tracer-vs1-h2`  
**Date:** 2026-07-18  
**Host:** grok-build  

## Decision

| Item | Result |
|---|---|
| Goal achieved | **Yes** — core journey owned by typed commands + snapshots |
| Mock store ownership | **Demoted** to browser/test backend + legacy compat helpers |
| Tauri preference | **Yes** (`invoke` auto prefers `__TAURI__`) |
| React ACP/SQLite/process | **None** |
| Required deterministic tests | **12/12 scenarios covered** |
| Wave 2 / push / merge | **Not done** (out of scope) |

## Deliverables

### Code

- `apps/desktop/src/shared/types/snapshot.ts` — snapshot DTOs + mappers
- `apps/desktop/src/shared/commands/invoke.ts` — tauri/mock/auto invoke
- `apps/desktop/src/shared/commands/mockBackend.ts` — deterministic command backend
- `apps/desktop/src/shared/store/snapshotStore.ts` — SnapshotJourney + AppViewState
- `apps/desktop/src/shared/store/snapshotStore.test.ts` — matrix tests
- Wired UI: `App.tsx`, `AppShell`, `GlobalStatusRegion`, `ProjectsHome`, `ProjectWorkspace`, `SessionWorkspacePlaceholder`, `AboutPage`
- Aliases: `@tracer/event-types` in desktop vite/tsconfig

### Docs

- `docs/modules/vs1-h2/VS1_H2_ARCHITECTURE.md`
- `docs/modules/vs1-h2/VS1_H2_TEST_MATRIX.md`
- `docs/modules/vs1-h2/VS1_H2_COMPLETION_REPORT.md`

### E2E scaffold

- `tests/e2e/desktop/README.md` — notes only (no live harness in this slice)

## Tests run

```text
pnpm --filter @tracer/desktop test
  ✓ 18 tests passed

pnpm --filter @tracer/desktop typecheck
  ✓ pass
```

## Assumptions

1. Control-plane `PresentationSnapshot` camelCase JSON matches the TS interface (W1-F already freezes this).
2. Tauri handlers accept structured `args` for multi-field commands and flat ids for single-field commands (matches current Rust signatures).
3. Live `tracer://events` fan-out remains optional; snapshot + events_list is the resilience path (Gate 1.3).
4. Mock backend may keep a demo project for browser UX; Tauri starts from real project_list (may be empty).

## Risks

| Risk | Mitigation |
|---|---|
| CP runtimeObservation strings drift from mapper | Mapper defaults to `not_started`; matrix tests lock known strings |
| Tauri arg wrapping mismatch | `normalizeTauriArgs` mirrors current `commands/mod.rs`; smoke in tauri:dev recommended |
| submit_prompt blocks until terminal on real CP | UI shows running via snapshot; long prompts need event fan-out or polling — document for integration |
| Empty project list on first Tauri launch | EmptyState + register project (host dialog still future) |

## Integration requirements

1. Run desktop under Tauri (`pnpm tauri:dev`) against fake ACP for end-to-end smoke.
2. Optional: poll `tracer_presentation_snapshot` while prompt in flight if event channel not subscribed.
3. Wire native folder picker → `tracer_project_register` (still disabled button with reason).
4. Do not merge until integrator reviews invoke arg shapes against live Tauri.

## Owned path compliance

| Path | Action |
|---|---|
| `apps/desktop/src/**` | Modified |
| `apps/desktop/src-tauri/src/commands/` | Read only (no change required) |
| `packages/ui/` | Read only (existing components sufficient) |
| `tests/e2e/desktop/` | Scaffold README |
| `docs/modules/vs1-h2/` | Added |
| Backend foundation crates | **Not modified** |