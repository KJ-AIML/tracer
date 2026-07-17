# Wave 2 Entry Criteria (post Gate 1.4)

**Status:** Evaluation only — **no Wave 2 product-feature tasks created by Gate 1.4.**

## Entry recommendation

**Gate 1.4 PASS** unlocks **planning / bounded Wave 2 entry** against the hardened VS1 baseline (`tracer-vs1-hardened` tag on main after finalize).

Wave 2 must **not** reopen VS1 sequence/backpressure correctness as greenfield; treat them as regression suite obligations.

## Prerequisites met

| Prerequisite | Status |
|---|---|
| VS1 fake path accepted (Gate 1.3) | yes |
| Sequence-preservation under concurrent ingest | proven (H3 + Gate 1.4 SOAK-01) |
| Soak suite SOAK-01…07 + stress | green on integrated branch |
| Desktop typed snapshot journey wired (H2) | integrated |
| Opt-in live harness present (H1) | integrated; CI-safe |
| Standard CI remains credential-free | yes |
| Integration branch clean + hardened tag | after finalize |

## Recommended Wave 2 sequencing (not created here)

1. **Shell presentation binding depth** — replace remaining mock-heavy views with live CP snapshots/events where Tauri is present; keep deterministic mock path for unit tests.
2. **Presentation fan-out capacity** — if product needs multi-subscriber UI, introduce a **bounded** presentation channel without redesigning orchestration.
3. **Live auth UX** — AuthenticationRequired / Failed paths against stock Grok (manual/live class only).
4. **Within-session persist_failed recovery UX** — clear-on-success or explicit operator recovery without poisoning other sessions (isolation already proven).
5. **Product polish** — multi-session UI, project library, settings — only after command contracts remain stable under soak regression.

## Hard constraints for any Wave 2 task

- Do not remove or weaken SOAK-01 sequence regression.
- Do not add a second unbounded persistence buffer.
- Do not put live Grok `run` into standard CI.
- Do not claim live parity without fresh sanitized evidence.
- Prefer fail-closed on persistence and terminal honesty.

## Explicit non-actions of Gate 1.4

- No Wave 2 Heli tasks created.
- No Wave 2 feature branches created.
- No push of tags or main.
- No automatic live provider usage.

## Deferred product scope (still out)

Collaboration, multi-agent orchestration, plugin marketplace, cloud sync, non-ACP providers.
