# ADR-001: Runtime as a Managed Sidecar Process

**Status:** Accepted (Wave 0)  
**Date:** 2026-07-17  
**Deciders:** W0-A Architecture and Contract Lead (task `tracer-w0-architecture-contracts`)  
**Related:** `docs/architecture/TRACER_VERTICAL_SLICE.md`, `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`

## Context

Tracer must run an ACP-compatible coding agent to execute tools, read/write project files (under policy), and stream progress. Two broad options exist for hosting that agent:

1. **In-process** in the Tauri/Rust application or embedded in the UI process.
2. **Out-of-process sidecar** spawned and supervised by the Tracer control plane.

The first vertical slice may use a stock runtime (for example Grok Build) or a deterministic fake runtime. A future downstream runtime binary may appear only after a separate adoption gate. Tracer must not rewrite runtime internals to ship the slice.

## Decision

**Tracer runs the agent runtime as a managed sidecar process** owned by the Rust control plane (process manager + adapter), not inside the React UI and not in-process with the desktop webview.

Characteristics:

- Communication for the first milestone: **JSON-RPC over stdio**.
- Lifecycle (spawn, stdout/stderr capture, graceful stop, force kill, orphan prevention) is **control-plane owned**.
- Protocol semantics are isolated behind a **runtime adapter**.
- The sidecar **must not** write to the primary Tracer SQLite database.
- Multiple future runtime kinds remain possible; the first kind is `acp-stdio`.

## Consequences

### Positive

- Crash isolation: runtime panics do not take down the entire UI process.
- Clear security boundary: UI cannot spawn arbitrary shells directly.
- Swap-ability: fake runtime, stock runtime, and future downstream binaries share one adapter interface.
- Supervisability: exit codes, stderr, and timeouts are first-class events.
- Aligns with ACP's process-oriented client/server model.

### Negative / costs

- Process management complexity (especially Windows termination and path resolution).
- Need for readiness and health semantics beyond "thread started".
- Slightly higher latency than pure in-process calls (acceptable for agent workloads).
- Packaging must ship or locate a runtime executable.

### Neutral

- Stdio framing details are implementation concerns validated against W0-B evidence and the fake runtime.

## Alternatives considered

### A. In-process library link of the runtime

- **Rejected** for the slice: couples Tracer releases to runtime internals, weakens crash isolation, complicates licensing and upgrade, and fights ACP's process model.

### B. Long-lived user-managed external server (manual start)

- **Rejected** as the primary model: poor UX, weak orphan/lifecycle guarantees, harder Gate 1 determinism. Optional advanced attach mode may be revisited later without changing this ADR's default.

### C. Network daemon per machine

- **Deferred**: useful later for multi-client, not required for Gate 1; increases auth and port-management scope.

## Implementation notes (non-normative)

- Wave 1 process manager owns OS-level concerns.
- Wave 1 ACP adapter owns initialize, capabilities, prompt, cancel.
- Default CI uses fake sidecar; stock runtime is optional smoke.
- Do not create `repos/tracer-agent-runtime` until the runtime adoption gate answers fork necessity.

## Validation

Acceptance that this ADR holds:

- Runtime death emits explicit Tracer events and UI states.
- Stop/cancel does not leave orphan processes in integration tests.
- UI packages do not import runtime-specific protocol types for product behavior.

## Revision history

| Version | Note |
|---|---|
| 1.0.0 | Initial acceptance in Wave 0 |
