# W1-F Test Matrix

**CI class:** standard — network no, credentials no, live Grok no, provider no.  
**Evidence:** fake-ACP + temp SQLite.

| ID | Scenario | Fake scenario | Status |
|---|---|---|---|
| VS-01 | Successful run | `happy_prompt_stream` | PASS |
| VS-02 | Auth required | `auth_required_session_new` | PASS |
| VS-03 | Auth failure distinct | class mapping + auth path | PASS |
| VS-04 | Unsupported capability | `capability_minimal` + `cancel_unsupported` | PASS |
| VS-05 | Cancel before approval | `cancel_while_permission_pending` | PASS |
| VS-06 | Approval allow once | `permission_allow` | PASS |
| VS-07 | Approval deny once | `permission_deny` | PASS |
| VS-08 | Runtime EOF | `eof_mid_prompt` | PASS |
| VS-09 | Runtime crash | `crash_nonzero_exit` | PASS |
| VS-10 | Malformed protocol | `malformed_frame` | PASS |
| VS-11 | Unknown vendor preserved | `unknown_vendor_notification` | PASS |
| VS-12 | Restart restores history | file SQLite reopen | PASS |
| VS-13 | Interrupted recovery | stale Running → reconcile | PASS |
| VS-14 | Heli unavailable usable | probe empty path | PASS |

Additional: binary missing spawn class; command error serde; app_info; storage open.

**Location:** `crates/tracer-control-plane/tests/vs_scenarios.rs`

## Platform

- Exercised: Windows (this worktree)
- Live Grok smoke: **not run** (standard CI; classification: optional / not required for Gate)
