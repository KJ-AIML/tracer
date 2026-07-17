# Wave 2 Readiness (evaluation only — no Wave 2 tasks created)

## Stable after Gate 1.3

- Domain events / error classes / sequence model
- Process manager foundation
- Storage + migrations + sole-writer pattern via control plane
- ACP runtime adapter + fake ACP contract suite
- Control plane vertical-slice orchestration (VS-01…14)
- Desktop Tauri command surface + invoke wrapper
- Heli read-only probe (absence non-fatal)

## Fake-only

- End-to-end agent streaming, approvals, cancel, crash/EOF taxonomy proven **only** against `fake-acp-runtime`.
- CI remains credential-free / network-free.

## Needs live auth

- Real Grok / provider ACP binary spawn
- AuthenticationRequired / AuthenticationFailed against live runtime
- Long-running live cancel and permission UX
- Provider-specific capability matrix

## Technical debts to clear first

1. Pre-existing clippy style debt in domain/process/storage (non-blocking).
2. Desktop UI still heavily mock-store; wire live snapshots/events into shell screens.
3. Presentation event bus (`tracer://events`) optional wiring depth.
4. Encoding glitches in some W1-F docs/test comments (cosmetic).
5. Bounded bridge capacity tuning under slow-disk stress.

## Recommended next bounded wave (not created)

**Sequential first:**

1. Live-runtime optional smoke harness (manual/local class) behind explicit flags — still non-CI.
2. Shell presentation binding: replace mock session views with CP snapshots + event fan-out.
3. Hardening: soak tests for cancel/approval races; bridge metrics.

**Parallelizable later modules** (after live smoke strategy exists): multi-session UI, project library polish, settings — only once command contract remains stable.

## Deferred product scope

- Collaboration, multi-agent orchestration, plugin marketplace, cloud sync, non-ACP providers.

## Explicit non-actions

- **No Wave 2 Heli tasks created.**
- **No Wave 2 branches created.**
- Gate 1.3 does not authorize feature expansion beyond vertical-slice closure.