# Vertical Slice 1 Acceptance

## Answer-first

**Yes — accepted on the fake-runtime path:** create/open project → runtime inspect → start fake runtime → session → prompt → stream → approval → cancel/complete → persist → restore → typed presentation works under Gate 1.3 evidence.

**Live Grok path: unproven** (optional; not executed; non-gating).

---

## Definition of accepted (this gate)

Vertical Slice 1 is **accepted** when the **fake ACP** stack proves command-boundary orchestration through persistence and presentation without credentials or network, and VS-01…VS-14 pass on the integrated tree.

## Supported

- Project register/list/get via control plane
- Session create with fake ACP (`tools/fake-acp-runtime`)
- Session readiness gates (process alive ≠ session ready ≠ auth)
- Prompt submit with continuous event drain
- Event persistence (memory + file SQLite) with monotonic storage sequences
- Presentation snapshot v1 (typed; no raw ACP)
- Approval list/resolve once
- Cancel concurrent with prompt (time-bounded, including permission-pending)
- Restart history reload (VS-12)
- Interrupted session reconcile (VS-13)
- Heli missing non-fatal (VS-14)
- Tauri command registration for `tracer_*` APIs
- Frontend invoke adapter: Tauri when available, mock fallback otherwise

## Unsupported / out of scope for VS1 acceptance

- Live authenticated Grok sessions
- Production provider credentials in CI
- Full React UI journey automation (beyond unit/mock store)
- Multi-user / multi-window concurrency product features
- Wave 2 product modules

## Evidence summary

| Class | Status |
|---|---|
| fake-runtime vertical slice | **accepted** |
| live Grok vertical slice | **unproven** |
| Platform | Windows |
| Gate 1.3 | **PASS** |
| Candidate SHA | `5b2232c84f35408449098ba82c32008d230a46e6` |

## Limitations

- VS scenario suite serializes fake-ACP process spawns on Windows for reliability.
- Live provider and auth UX remain unproven.
- Presentation fan-out channel is optional; shell restore-from-snapshot is the resilience path.