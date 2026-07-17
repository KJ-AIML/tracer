# WAVE 1.4 Hardening Report — Gate 1.4 (VS1 Hardening Integration)

**Gate:** 1.4  
**Branch:** `integration/tracer-vs1-hardening`  
**Platform:** Windows 10 / PowerShell / Rust workspace + pnpm  
**Date:** 2026-07-18  
**Integrator session:** `heli-ses-94103703-fc8c-441b-8a7a-a7b6ee532c6d`  
**Heli task / lease:** `tracer-vs1-hardening-integration` / `heli-lease-6e99802b-3847-4391-9684-e051876f259f`

---

## Decision

| Item | Result |
|---|---|
| **Gate 1.4** | **PASS** |
| **Product-local readiness** | **PASS** (fake ACP + file SQLite + desktop snapshot journey) |
| **Live-provider readiness** | **PROVEN_ON_AUTHORING_HOST** (H1 evidence reused; no new provider usage) |
| **Wave 2 product features** | **not started** |

---

## Source / merge provenance

| Item | Value |
|---|---|
| main tip (pre-integration / VS1 tag) | `15c9399c28f79bdf9c125c26f52d7bf956fb4722` |
| tag `tracer-vertical-slice-1` | points at pre-integration main tip |
| H3 soak tip / branch | `ae139db0004eb47dc79a133b0c3aa5bfb668e704` / `agent/tracer-vs1-soak` |
| H2 desktop tip / branch | `5f21a7ad0b4a2a68ed76d6913a0eccb0687cabef` / `agent/tracer-vs1-desktop-wiring` |
| H1 live tip / branch | `a8c0cd202f75af220ed575839a1124ef4505c353` / `agent/tracer-vs1-live-smoke` |

### Required merge order (non-FF)

| Order | Merge commit | Message |
|---|---|---|
| 1 H3 | `d0bbcbddb551d2d0e79f8dc1390e6fc19835129e` | merge(vs1-h3): soak concurrency and sequence-preservation fix |
| 2 H2 | `f39e6a3aa4153528e5443eae7388e97f9ba5f0b1` | merge(vs1-h2): desktop snapshot journey wiring |
| 3 H1 | `3152951f3885a31ad3ad8e407ef7eeec1a2e767f` | merge(vs1-h1): opt-in live Grok smoke harness |

### Reconciliation commits

| SHA | Purpose |
|---|---|
| `3152951` (H1 merge body) | Cargo.toml workspace member **union**: `tests/soak`, `tests/stress`, `tools/live-grok-smoke` |
| `a2fed0458f79f62cf78af89758820004bf76441f` | rustfmt after H3/H2/H1 integration |
| `40cf53dddcf495a23d06606fe0778cf894c8a8ad` | sticky persist_failed isolation test (SOAK-07) |
| `432eec519f792a8567f111a76455e0df9eacbec8` | Gate 1.4 reports (hardening, matrix, readiness, Wave 2 entry) |

No squash of source branches. Desktop TS aliases and live-smoke CI exclusion required no further mechanical conflict resolution beyond Cargo.toml union.

---

## Part 4 — Sequence correctness (mandatory)

### Defect (H3 finding)

`session_create` paths used full-row `update_session` from a stale `get_session` snapshot after the ingest pump had already advanced `next_sequence`, rewinding the counter → UNIQUE `(session_id, sequence)` storms → silent stream loss under burst.

### Fix present on integrated branch

`ControlPlane::update_session_preserving_sequence` in `crates/tracer-control-plane/src/plane.rs`:

- Computes `min_next = max(events.latest_sequence+1, current.next_sequence, rec.next_sequence, 1)`
- **Never decreases** `next_sequence`
- Retries up to 8 times if pump advances during write
- Used on session-ready and create-failure status persistence (ingest-active paths)

Pre-ingest spawn-failure path still uses plain `update_session` (no concurrent pump yet).

### Focused regression (SOAK-01 on integrated branch)

| Metric | Threshold | Observed |
|---|---|---|
| Burst size | > bridge 256 | 600 deltas |
| `bridge_accepted` | ≥ 600 | **607** |
| `events_persisted` | ≥ 600 / == accepted | **607** (matches accepted) |
| `persist_errors` | 0 | **0** |
| Storage sequences | monotonic, unique | **PASS** |
| Duplicate sequences | 0 | **0** |
| Event loss | 0 | **0** |
| Terminal-ish delivery | required | completed/cancelled/message.completed present |
| Shutdown bound | < 15s | ~59–108 ms class |

**Gate 1.4 sequence-rewind regression: PROVEN (PASS).**

---

## Parts 5–6 — Instrumentation + backpressure audit

| Requirement | Status | Evidence |
|---|---|---|
| Instrumentation test-focused | PASS | `IngestMetrics`, soak delay hook |
| Delay injection off by default | PASS | `TRACER_SOAK_PERSIST_DELAY_MS` only when set |
| No second unbounded persist queue | PASS | Bridge only: adapter unbounded → `mpsc(256)` → async pump |
| Bridge 256 + `blocking_send` | PASS | `BRIDGE_CAPACITY=256`; drain uses `blocking_send` |
| Cancel under saturation | PASS | SOAK-02 cancel under slow-DB; SOAK-04 races |
| Terminal not dropped on persist fail | PASS | `persist_failed` blocks false complete claim |
| Presentation fan-out | **Documented risk** | Optional `std::sync::mpsc` unbounded; post-persist; SOAK-03 proves slow consumer does not block persist. **No redesign** in Gate 1.4 (small/safe bound deferred). |

---

## Parts 7–8 — Desktop + Tauri / fake smoke

| Item | Status |
|---|---|
| H2 typed snapshot store + mock backend | Integrated |
| React: no raw ACP / no SQLite / no process lifecycle | **PASS** — UI uses typed commands + snapshot store; mock backend for deterministic shell tests |
| Desktop vitest | **18 passed** (`snapshotStore` + `mockStore`) |
| Desktop `pnpm build` | **PASS** (tsc + vite) |
| Strongest deterministic boundary | Control-plane VS scenarios + desktop command/snapshot tests |

**Not claimed:** full GUI E2E click-path against live Tauri + real process.

Classification of desktop smoke:

```text
fake ACP: yes (via CP VS / soak)
file SQLite: yes (file-backed VS + soak)
live Grok: no
network: no
credentials: no
```

---

## Parts 9–10 — Live harness + classification

| Item | Status |
|---|---|
| Opt-in harness integrated | `tools/live-grok-smoke` workspace member |
| Standard CI starts Grok / needs credentials | **No** — unit tests dry-run only; `run` requires `TRACER_LIVE_GROK=1` |
| Live evidence this gate | **Reused from H1 authoring-host validation** |
| New provider usage in Gate 1.4 | **None** |
| Version skew | W0-B documented **0.2.102**; H1 host observed **0.2.103** |
| Cross-platform / standard CI / all versions / production reliability | **Not claimed** |
| Approval reverse-request forced | **Not proven** on H1 default prompt (documented residual) |

Live readiness classification: **PROVEN_ON_AUTHORING_HOST** (consistent with sanitized H1 LVS-01…08 PASS evidence).

---

## Parts 11–12 — Soak + sticky persist_failed

| Scenario | Result |
|---|---|
| SOAK-01 event burst | PASS |
| SOAK-02 slow database | PASS |
| SOAK-03 slow presentation | PASS |
| SOAK-04 concurrent commands | PASS |
| SOAK-05 restart recovery | PASS |
| SOAK-06 repeated sessions | PASS |
| SOAK-07 sticky isolation (Gate 1.4) | PASS |
| Stress sequential sessions | PASS (20 sessions) |

Thresholds held: event loss 0, dup sequences 0, terminal lost 0, orphans 0, stale approvals 0 (suite asserts).

### Sticky `persist_failed` invariant

- Flag is **per-LiveSession** `SessionRuntimeState`, not global ControlPlane / DB health.
- Session stop removes live registry entry → flag discarded.
- Later sessions start with `persist_failed=false` (SOAK-07).
- Within a live session after true persist error: flag sticky; prompt path returns StorageError rather than claiming complete (fail-closed). Optional clear-on-success remains residual polish, not a cross-session poison vector.

---

## Part 13 — Full deterministic validation (executed)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | PASS (after reconciliation fmt) |
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS (incl. soak/stress/live-grok unit/VS) |
| `cargo clippy --workspace --all-targets` | PASS (pre-existing style warnings only; no deny failures) |
| `pnpm install --frozen-lockfile` | PASS |
| `pnpm -r test` | PASS (desktop 18, ui 3, event-types 11, fake-runtime 30) |
| `pnpm -r build` | PASS |
| `cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1` | **23 passed** |

No standard test required live Grok credentials or network.

---

## Residual risks

1. Presentation fan-out remains unbounded std mpsc (post-persist; SOAK-03 mitigates product risk for persistence).
2. Within-session sticky `persist_failed` after transient error does not auto-clear on later success.
3. Live Grok version skew 0.2.102 vs 0.2.103; authoring-host only.
4. Full GUI E2E not in Gate 1.4.
5. Bridge 256 may backpressure into W1-D unbounded adapter channel under extreme slow disk (by design).

---

## Wave 2 entry

See `docs/integration/WAVE_2_ENTRY_CRITERIA.md`. Gate 1.4 **allows** Wave 2 planning entry against hardened VS1; **does not** create Wave 2 product tasks in this gate.

---

## Finalize checklist

| Step | Status |
|---|---|
| Separate correctness / desktop / workspace / report commits | Done via merge + recon + reports |
| Clean integration branch | Yes |
| main FF to integration | **done** |
| Local annotated tag `tracer-vs1-hardened` | **done** |
| Push | **Never** |
| Heli lease release | **done after finalize** |

---

## Final SHAs (post-report amend)

| Item | SHA |
|---|---|
| Integration tip (pre-FF) | `84160ba922144c6e47455f7de02c51374d852175` |
| Sticky test commit | `40cf53dddcf495a23d06606fe0778cf894c8a8ad` |
| Reports commit | `432eec519f792a8567f111a76455e0df9eacbec8` |
