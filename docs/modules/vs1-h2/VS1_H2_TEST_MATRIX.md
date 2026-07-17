# VS1-H2 Test Matrix

**Task:** `tracer-vs1-desktop-wiring`  
**Runner:** `pnpm --filter @tracer/desktop test` (vitest + jsdom)  
**Backend under test:** deterministic `MockBackend` (no network, no credentials)

## 1. Required scenarios

| # | Scenario | File / test | Pass criteria |
|---|---|---|---|
| 1 | Initial snapshot | `snapshotStore.test.ts` → initial snapshot | loadPhase ready; projects≥1; status ready; composer enabled |
| 2 | Runtime unavailable | runtime unavailable | status failed; pill unavailable; composer disabled |
| 3 | Authentication required | authentication required | auth unauthenticated; pill sign_in_required; submit → AuthenticationRequired |
| 4 | Prompt streaming | prompt streaming | status running; prompt.submitted + message.delta; no ACP method names |
| 5 | Approval request | approval request | awaiting_approval; pendingApprovals≥1; side tab approvals |
| 6 | Approval accepted | approval accepted | pending cleared; status running; approval.resolved |
| 7 | Approval rejected | approval rejected | pending cleared; decision deny |
| 8 | Cancel pending approval | cancel pending approval | pending cleared; status stopped |
| 9 | Completed run | completed run | status completed; session.completed event |
| 10 | Runtime crash | runtime crash | status disconnected; pill crashed; never running |
| 11 | Session-history restore | session-history restore | events_list length≥4; snapshot restore after refresh |
| 12 | Heli unavailable | Heli unavailable | heli.available=false; loadPhase ready; non-fatal banner |

## 2. Supporting pure tests

| Area | Coverage |
|---|---|
| `mapRuntimeObservation` | control-plane strings → UI catalog; auth override |
| Legacy `mockStore` | compat smoke (composer reasons, disconnect honesty) |

## 3. Explicit non-goals

| Not run | Reason |
|---|---|
| Live Grok invoke | Credentials / network forbidden in standard tests |
| Full Playwright desktop e2e | Scaffold only under `tests/e2e/desktop/` |
| Control-plane Rust VS suite | Owned by W1-F / integration tasks |

## 4. How to run

```powershell
cd repos/worktrees/tracer-vs1-h2
pnpm install
pnpm --filter @tracer/desktop test
pnpm --filter @tracer/desktop typecheck
```

## 5. Evidence (this delivery)

| Check | Result |
|---|---|
| vitest desktop | **18 passed** (14 journey + 4 mockStore compat) |
| tsc desktop | **pass** |
| Network in tests | **none** |
| Credentials in tests | **none** |