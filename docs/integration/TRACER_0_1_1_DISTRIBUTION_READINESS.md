# Tracer 0.1.1 Distribution Readiness (Gate 2.4.2)

**Product:** Tracer desktop `0.1.1` / `dev.tracer.desktop`  
**Date:** 2026-07-19  
**Prior gates:** 2.3 Windows RC PASS; 2.4.1 upgrade+provenance PASS (`tracer-wave2.4.1-upgrade-verified` @ `d83a873`)

## Channel matrix

| Channel | Status | Notes |
|---|---|---|
| Internal unsigned RC (testers) | **READY_WITH_WARNINGS** | NSIS + portable; `UNSIGNED_DEVELOPMENT_RC` |
| Self-signed test artifacts | **MECHANICS_ONLY** | Not for distribution |
| Public download (signed) | **BLOCKED** | No org Authenticode certificate |
| SmartScreen-clean public | **UNPROVEN / BLOCKED** | Requires trusted signing + reputation |
| Auto-update signed channel | **BLOCKED** | Outside Gate 2.4.2; no trusted updater signing claim |

## Signing posture for 0.1.1

| Field | Value |
|---|---|
| Default mode | `UNSIGNED` |
| Trusted readiness | `BLOCKED_NO_CERTIFICATE` |
| Publisher | `UNPROVEN` |
| Timestamp | `UNPROVEN` |
| Production signing | `BLOCKED` |

## Integrity / provenance

Release provenance (`pnpm release:provenance`) emits explicit unsigned signing fields per artifact. Rebuilds change SHA-256 (timestamp/clock sensitive); always record hashes per build.

## Honest distribution statement

Tracer **0.1.1** may be shared as an **unsigned internal development RC** with explicit warnings. It is **not** ready for public production distribution that depends on Authenticode trust, publisher identity, timestamp longevity, or SmartScreen reputation.