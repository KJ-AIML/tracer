# VS1-H3 Soak Runner

Time-bounded concurrency and persistence soak tools for Vertical Slice 1 hardening.

## Environment

| Knob | Value |
|------|-------|
| Fake ACP runtime | yes (stock + soak burst) |
| File-backed SQLite | yes |
| Network | no |
| Credentials | no |
| Provider / live Grok | no |

## How to run

From the tracer worktree root:

```powershell
# Full soak + stress (recommended)
pwsh -File tools/soak-runner/run-soak.ps1

# Soak only
cargo test -p tracer-vs1-soak -- --nocapture --test-threads=1

# Stress only (time-capped)
cargo test -p tracer-vs1-stress -- --nocapture --test-threads=1
```

### Optional env

| Env | Purpose |
|-----|---------|
| `TRACER_SOAK_BURST_COUNT` | Chunk count for `burst-fake-acp.js` (default 600; must exceed bridge 256) |
| `TRACER_SOAK_BURST_DELAY_MS` | Inter-chunk delay in burst fake |
| `TRACER_SOAK_SCENARIO` | `happy_burst` (default) or `permission_hold` |
| `TRACER_SOAK_PERSIST_DELAY_MS` | Artificial per-event persist delay in control plane (soak02) |

## Artifacts

| Path | Role |
|------|------|
| `burst-fake-acp.js` | Soak-only ACP that floods `agent_message_chunk` beyond bridge capacity |
| `run-soak.ps1` | Windows runner |
| `tests/soak` | Scenario suite (SOAK-01…06) |
| `tests/stress` | Bounded repeated-session stress |
| `docs/validation/soak/` | Plan + results |

## Thresholds (hard invariants)

Defined in `tests/soak/src/lib.rs` (`thresholds` module) before run:

- max event loss: 0
- max duplicated persisted events: 0
- terminal events lost: 0
- orphan processes: 0
- stale actionable approvals: 0
- unjoined owned tasks after shutdown: 0

No production throughput SLAs are invented; observed metrics are recorded in results docs.
