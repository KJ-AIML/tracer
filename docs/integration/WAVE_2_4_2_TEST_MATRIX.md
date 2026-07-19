# Wave 2.4.2 Test Matrix — Authenticode Signing Readiness

**Gate:** 2.4.2  
**Date:** 2026-07-19  
**Host:** grok-build

## A. Workspace / regression

| ID | Check | Result |
|---|---|---|
| A01 | `cargo fmt --all --check` | PASS |
| A02 | `cargo check --workspace` | PASS |
| A03 | `cargo test --workspace` | PASS |
| A04 | `cargo clippy --workspace --all-targets` | PASS (pre-existing warnings; no `-D`) |
| A05 | `pnpm install --frozen-lockfile` | PASS |
| A06 | `pnpm -r test` | PASS (includes signing unit tests; no trusted sign) |
| A07 | `pnpm -r build` | PASS |

## B. Release packaging / upgrade

| ID | Check | Result |
|---|---|---|
| B01 | `pnpm release:windows` | PASS / `UNSIGNED_DEVELOPMENT_RC` |
| B02 | `pnpm test:release:upgrade` | PASS (R01–R14 + UF-01..05) |
| B03 | `pnpm release:provenance` | PASS / signing fields explicit UNSIGNED |

## C. Signing readiness

| ID | Check | Result |
|---|---|---|
| C01 | `pnpm release:sign:doctor` | PASS → `READY_SELF_SIGNED_TEST_ONLY` / trusted `BLOCKED_NO_CERTIFICATE` |
| C02 | `pnpm release:sign:test` | PASS (mechanics, tamper, cleanup, originals unchanged) |
| C03 | `pnpm release:verify-signature` | PASS (canonical artifacts NotSigned / UNSIGNED) |
| C04 | `pnpm test:release:signing` | PASS 13/13 |
| C05 | Trusted without auth (`TRACER_SIGNING_MODE=TRUSTED_AUTHENTICODE`) | FAIL_CLOSED → `BLOCKED_NO_AUTHORIZATION` (exit 2) |
| C06 | Cert store cleanup after self-signed | PASS (0 Tracer Self-Signed certs remain) |
| C07 | Hygiene scan stage JSON | PASS (no PEM/password secrets) |

## D. Classifications (expected Gate 2.4.2)

| Concern | Classification |
|---|---|
| Pipeline mechanics | PASS |
| Self-signed validation | PASS |
| Trusted Authenticode | BLOCKED_NO_CERTIFICATE |
| Publisher | UNPROVEN |
| Timestamp | UNPROVEN |
| SmartScreen | UNPROVEN |
| Production signing | BLOCKED |
| Internal unsigned distribution | READY_WITH_WARNINGS |
| Public signed distribution | BLOCKED |
| Standard CI isolation | PASS (no trusted workflow active) |

## E. Explicit non-runs

| Item | Status |
|---|---|
| Trusted cert / private key | no |
| Trusted timestamp network | no |
| Managed signing provider | no |
| Live Grok GUI | no |
| Certificate purchase/enroll | no |