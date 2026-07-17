# ADR-002: Normalize Runtime Events Before UI and Storage

**Status:** Accepted (Wave 0)  
**Date:** 2026-07-17  
**Deciders:** W0-A Architecture and Contract Lead (task `tracer-w0-architecture-contracts`)  
**Related:** `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`, `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`

## Context

ACP-compatible runtimes (and vendor extensions) emit heterogeneous notifications: message chunks, tool calls, plans, permission requests, logs, and proprietary fields. Tracer needs:

- a stable UI programming model
- durable session history
- the ability to change or replace runtimes without rewriting React features or migrations
- debuggability when vendor fields matter

If the UI or SQLite schema binds directly to raw ACP or Grok-specific shapes, every upstream change becomes a product-wide break.

## Decision

**All runtime traffic that affects product behavior is normalized into Tracer Event Protocol v1 envelopes** inside the control plane (adapter/normalizer) **before**:

1. streaming to the desktop UI, and
2. persisting to the primary database.

Rules:

1. React components **must not** depend on Grok-specific or raw ACP method names for behavior.
2. Storage tables **must not** require Grok-specific identifiers as primary keys; runtime ids are optional metadata.
3. Raw or vendor payloads **may** be preserved under adapter metadata when safe and size-bounded for debugging.
4. Unknown but well-formed runtime notifications become `adapter.protocol.unknown` (or equivalent) rather than being dropped silently.
5. Malformed traffic becomes `adapter.protocol.error` without crashing the UI.
6. Control plane assigns Tracer `eventId`, monotonic per-session `sequence`, and observation `timestamp`.

## Consequences

### Positive

- UI, tests, and storage share one envelope contract.
- Runtime replacement cost is concentrated in the adapter.
- Contract tests can use fixtures independent of live vendors.
- Forward compatibility via ignore-unknown-fields and unknown types.

### Negative / costs

- Mapping layer must be maintained when ACP or vendors evolve.
- Some fidelity may be lost if metadata is truncated (mitigate with caps + debug flags).
- Dual representation (raw optional + normalized required) increases storage size.

### Neutral

- Exact ACP field mapping for stock Grok Build is produced by W0-B research; normalization **semantics** remain owned by this ADR and the event protocol.

## Alternatives considered

### A. Pass raw ACP JSON to the UI

- **Rejected:** leaks protocol churn into every feature; prevents multi-runtime; weak typing; security footguns.

### B. Store only raw events; normalize at read time

- **Rejected as primary model:** read-time normalization complicates every consumer and risks inconsistent historical interpretation when mappers change. Optional re-normalization tools may exist later for repair.

### C. Per-runtime UI codepaths

- **Rejected:** combinatorial explosion; contradicts runtime independence design rule.

## Normalization responsibilities

| Stage | Responsibility |
|---|---|
| Process manager | Bytes on pipes; exit; no product event meaning |
| ACP client | Framing; JSON-RPC correlation |
| Normalizer | Map to `type` + `payload` + adapter metadata |
| Control plane | Envelope identity, sequence, persist, broadcast |
| UI | Render known types; generic fallback for unknown |

## Unknown, cancel, and exit (summary)

- **Unknown events:** keep session alive; show generic timeline entry; preserve metadata when safe.
- **Cancel:** emit status/cancel events; cooperative cancel if capable; else process stop; never silent success.
- **Process exit:** always explicit lifecycle events; map to failed/disconnected/stopped; never leave UI believing the agent is running.

Details: `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md` and `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`.

## Validation

- Contract tests assert envelope required fields and unknown-type tolerance.
- Integration tests prove UI mock consumers work with fixtures only (no ACP parser in UI).
- Storage round-trip preserves `sequence` order and unknown payloads.

## Revision history

| Version | Note |
|---|---|
| 1.0.0 | Initial acceptance in Wave 0 |
