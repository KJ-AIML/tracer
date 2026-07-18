# W2-C Test Matrix — Multi-Session Isolation

**Task:** `tracer-w2-multi-session`  
**CI class:** standard — network no, credentials no, live Grok no  
**Evidence backend:** fake ACP (`tools/fake-acp-runtime`) + temp/file SQLite  
**Platform exercised:** Windows (this worktree)

## 1. Isolation suite (MS-01…MS-16)

**Runner:**

```powershell
cargo test -p tracer-control-plane --test multi_session -- --test-threads=1
```

| ID | Scenario | Fake / storage | Pass criteria |
|---|---|---|---|
| MS-01 | Sequential many sessions | file SQLite, `happy_prompt_stream` ×6 | all stop cleanly; per-id history; monotonic sequences; consistent `sessionId` |
| MS-02 | Presentation focus switch | in-memory, 2 live | focus A then B; both remain alive/Ready; `shutdown_all` empties registry |
| MS-03 | History while peer ingests | file SQLite | concurrent `events_list` on hist never mixes ids; hist `latest_sequence` does not grow from peer |
| MS-04 | Cancel isolation | `cancel_mid_stream` + bystander | cancel victim accepted; bystander not Failed/poisoned |
| MS-05 | Cross-session approval reject | dual `permission_allow` | resolve A’s approval on B → `ApprovalUnknown`; A still pending; A resolve ok |
| MS-06 | Runtime vs Tracer ids | 2 live | unique Tracer ids; runtime id ≠ Tracer id; `runtime_status` filter by Tracer id; no event id leak |
| MS-07 | Sequences session-local | file SQLite | both sessions have local seq starting at 1; independent latest |
| MS-08 | Failed spawn non-poison | bad executable then good | spawn class error; later create+prompt succeed |
| MS-09 | Peer crash non-poison | `crash_nonzero_exit` + stable | stable stays Ready and accepts prompt |
| MS-10 | Restart restores completed | file SQLite reopen | both sessions listable; history; `presentation_focus` history-only |
| MS-11 | Interrupted recovery | stale Running / AwaitingApproval | after reopen: not live process; list + history ok |
| MS-12 | Stale approvals per session | dual `cancel_while_permission_pending` | cancel A clears A only; cancel B clears B |
| MS-13 | No live registry leaks | 3 create, stop one, shutdown | counts 3→2→0; empty runtime_status; clean snapshot focus |
| MS-14 | Deterministic shutdown_all | 4 live, double shutdown | registry empty twice; history still readable |
| MS-15 | One prompt per session | `cancel_mid_stream` double submit | second → `InvalidState` |
| MS-16 | Parallel prompts across sessions | dual happy | both prompts ok; histories isolated + monotonic |

**Serialization:** tests take a process-wide `ms_lock` (Windows node spawn contention).

## 2. Stress suite

**Runner:**

```powershell
cargo test -p tracer-vs1-stress --test stress_multi_session -- --test-threads=1 --nocapture
```

| ID | Scenario | Budget | Pass criteria |
|---|---|---|---|
| ST-MS-01 | Overlapping live sessions | ≤180s, ≤8 sessions | ≥2 live; focus switch all; prompts; `shutdown_all`; session-scoped histories |
| ST-MS-02 | Create/stop cycles with peer | ≤180s, 10 cycles | ≥3 cycles; long-lived peer remains healthy; final shutdown clean |

No production throughput SLAs invented — time-capped only.

## 3. Regression (must stay green)

```powershell
cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1
```

| Suite | Expectation |
|---|---|
| VS-01…VS-14 + helpers | **All pass** after W2-C plane changes |

## 4. Explicit non-goals

| Not run | Reason |
|---|---|
| Live Grok multi-session | Credentials / network; Wave 2 live path separate |
| Desktop multi-tab UI | W2 product polish; not W2-C ownership |
| Cloud collab / multi-user | Forbidden scope |
| Infinite soak | Use bounded stress only |

## 5. Evidence (this delivery)

| Check | Result |
|---|---|
| `multi_session` | **16 passed** (3× flake check green) |
| `vs_scenarios` | **23 passed** |
| `stress_multi_session` | **2 passed** (8 live / 10 cycles observed) |
| Network / credentials | **none** |
