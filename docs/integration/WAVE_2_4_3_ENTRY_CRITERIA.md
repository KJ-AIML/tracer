# Wave 2.4.3 Entry Criteria (criteria only — no tasks)

**Predecessor:** Gate 2.4.2 Authenticode Signing Readiness (**PASS** expected after integration)  
**Date:** 2026-07-19  
**Scope of this document:** Entry criteria only. Do **not** create W2.4.3 tasks, branches, or leases from Gate 2.4.2.

## Required before starting Wave 2.4.3

1. Gate 2.4.2 annotated tag `tracer-wave2.4.2-signing-pipeline-verified` present locally and pointing at main tip that includes signing pipeline + integration reports.
2. Gate 2.4.1 tag `tracer-wave2.4.1-upgrade-verified` remains aligned to `d83a873` (must not be moved).
3. Signing modes contract stable: `UNSIGNED` / `SELF_SIGNED_TEST` / `TRUSTED_AUTHENTICODE`.
4. Trusted path remains fail-closed without `TRACER_SIGNING_AUTHORIZED=1` + certificate material.
5. Standard CI (`pnpm -r test`) still never trusted-signs.
6. No secrets (PFX/keys/passwords) in repository tree.

## Expected Wave 2.4.3 themes (non-binding preview)

- Organization certificate / managed signing provisioning (out of Gate 2.4.2)
- Authorized trusted Authenticode signing of release candidates
- Timestamp authority configuration and proof
- Publisher identity binding
- SmartScreen / public distribution evidence gathering
- Any remaining release-channel hardening beyond signing mechanics

## Explicit exclusions until authorized

- Purchasing or enrolling certificates from an integration worker alone
- Using real private keys in CI without release workflow isolation
- Claiming SmartScreen readiness without evidence
- Live Grok GUI, cross-platform packaging, IDE, ALMS, or plugin work unless separately authorized

## Exit criteria preview (for planning only)

Trusted Authenticode signing of 0.1.x (or successor) RC with recorded publisher + timestamp classifications that are no longer `UNPROVEN`/`BLOCKED` where evidence exists — details owned by the future W2.4.3 plan, not this gate.