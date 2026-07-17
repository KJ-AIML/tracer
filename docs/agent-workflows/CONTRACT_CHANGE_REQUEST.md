# Contract Change Request (CCR)

Gate 0 froze:

- `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`
- `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`
- `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md`
- Related ADRs and UX/test freezes referenced by readiness matrix

## When required

Any change that alters:

- Event type names or required envelope fields
- Tauri command names / error classes
- Adapter lifecycle guarantees
- Acceptance scenario IDs or expected-event forbidden aliases

## Template

```markdown
# CCR-<nnn>: <short title>

**Requester task:** `tracer-w1-...`
**Date:** YYYY-MM-DD
**Contracts touched:**
- docs/contracts/...

## Motivation

<why the freeze must move>

## Proposed change

<diff summary; keep minimal>

## Compatibility

- [ ] Backward compatible for in-flight Wave 1 modules
- [ ] Requires coordinated multi-module update (list modules)

## Impact

| Module | Action needed |
|---|---|
| W1-B | ... |
| W1-D | ... |
| W1-G | ... |

## Validation plan

- Fixture / expected-event updates
- Commands to re-run

## Approval

- [ ] Contract owner (W0-A lineage / maintainer)
- [ ] Affected module owners
- [ ] Integration owner (if cross-cutting)
```

## Process

1. File CCR under `docs/contracts/changes/` or task evidence (do not silently edit freeze docs).  
2. Get approvals listed above.  
3. Land coordinated PR/integration; update traceability matrix.  
