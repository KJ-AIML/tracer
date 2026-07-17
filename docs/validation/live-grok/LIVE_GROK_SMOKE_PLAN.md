# Live Grok Smoke Plan (VS1-H1)

**Work item:** VS1-H1  
**Task:** `tracer-vs1-live-smoke`  
**Harness:** `tools/live-grok-smoke`  
**Classification:** manual local / live authenticated smoke  
**Standard CI:** **excluded** (opt-in only)

## 1. Purpose

Validate the stock Grok runtime path used by Tracer:

```text
grok agent --no-leader stdio
```

against W0-B research evidence and the W1-D adapter handoff contract, without redesigning adapters or the control plane.

## 2. Sources of truth

| Source | Path / API |
|---|---|
| W0-B lifecycle | `docs/research/grok-build/PROCESS_LIFECYCLE.md` |
| W0-B completion | `docs/research/grok-build/W0-B_COMPLETION_REPORT.md` |
| W0-B capabilities | `docs/research/grok-build/CAPABILITY_MATRIX.md` |
| W1-D spawn helper | `tracer_runtime_adapter::grok_stdio_spawn_config` |
| W1-D public API | `docs/modules/w1-d/W1_D_PUBLIC_INTERFACE.md` |
| W1-F handoff | `docs/integration/W1_F_HANDOFF_CONTRACT.md` |
| CI policy | `tests/specifications/ci/matrix.yaml` (`live-smoke` job, `standardCi: false`) |

## 3. Safety constraints

1. Never store credentials; never print auth tokens; never commit private prompts.
2. Require **explicit operator intent** before consuming provider usage:
   - `TRACER_LIVE_GROK=1` (alias `TRACER_LIVE_SMOKE=1`)
   - CLI subcommand `run`
3. Support **dry-run** that validates command construction without launching agent stdio.
4. Write **sanitized** structured evidence only.
5. Record platform and runtime version.
6. Do not fail the fake-runtime vertical slice when auth is unavailable.

## 4. Stage plan

| # | Stage | Success criteria | Auth needed? |
|---|---|---|---|
| 1 | Binary discovery | Resolve `grok` (PATH / `TRACER_GROK_BIN` / `--grok`); capture version | No |
| 2 | Process startup | `RuntimeAdapter::start(grok_stdio_spawn_config(...))`; process alive | No |
| 3 | Protocol initialization | `initialize()` → protocol ready; process ready ≠ session ready | No |
| 4 | Authentication requirement | Inspect auth methods / state from initialize | No |
| 5 | Authenticated session creation | `create_session` → session ready **or** map `AuthenticationRequired` | Yes for Pass |
| 6 | Prompt submission | `submit_prompt` with public-safe text | Yes |
| 7 | Streaming | ≥1 normalized Tracer event type from stream path | Yes |
| 8 | Approval if requested | Observe `approval.requested`; **never auto-approve** | Yes if tools |
| 9 | Cancellation | `cancel_prompt` concurrent with prompt; no deadlock | Yes |
| 10 | Shutdown | `shutdown` / force; process not alive (no orphan) | No (if started) |

## 5. Scenarios (LVS)

| ID | Maps to stages | Notes |
|---|---|---|
| LVS-01 | 2 | runtime process starts |
| LVS-02 | 3 | protocol initialize succeeds |
| LVS-03 | 4 | authentication state identified correctly |
| LVS-04 | 5 | session creation succeeds |
| LVS-05 | 6–7 | prompt streams ≥1 normalized event |
| LVS-06 | 6–7 | completion or controlled terminal result |
| LVS-07 | 9 | cancellation does not deadlock |
| LVS-08 | 10 | shutdown leaves no orphan process |

## 6. Assumption checks (W0-B / W1-D)

| ID | Assumption | Expected observation |
|---|---|---|
| A-W0B-01 | Start command is `grok agent --no-leader stdio` | Spawn plan args match |
| A-W1D-01 | `grok_stdio_spawn_config` emits that argv | Product helper used (no reimplementation) |
| A-W0B-02 | `initialize` may succeed without credentials | Live initialize Ok when binary present |
| A-W0B-03 | `session/new` without auth → Authentication required | Error class / blocked stage when unauthenticated |
| A-W1D-02 | process alive ≠ protocol ready ≠ session ready | After initialize, session_ready false until session/new |

## 7. Result classification

| Classification | When |
|---|---|
| `NOT_RUN` | Dry-run only; or live not invoked |
| `BLOCKED_BY_AUTH` | Process/init ok; session/prompt blocked on auth |
| `PARTIAL` | Some live stages/scenarios passed; not full LVS set |
| `PASS` | LVS-01…LVS-08 all Pass under authenticated run |
| `FAIL` | Unexpected non-auth failure (spawn crash, init fail, orphan, etc.) |

## 8. Operator runbook

```powershell
# From tracer worktree root
cargo test -p live-grok-smoke
cargo run -p live-grok-smoke -- dry-run --out target/live-grok-smoke/dry-run.json

# Unauthenticated live probe (expected BLOCKED_BY_AUTH at session if not logged in)
$env:TRACER_LIVE_GROK = "1"
# Optional hermetic home:
# $env:GROK_HOME = "$PWD/target/live-grok-smoke/grok-home"
cargo run -p live-grok-smoke -- run --allow-unauth --out target/live-grok-smoke/live.json

# Authenticated full smoke (operator must already be logged in / stock auth available)
cargo run -p live-grok-smoke -- run --out target/live-grok-smoke/live-auth.json
```

## 9. Out of scope

- Control-plane / ACP adapter / process-manager redesign  
- Desktop product work  
- Wave 2 features  
- Automatic live execution in standard CI  
- Claiming fake-runtime parity failures when only live auth is missing  
