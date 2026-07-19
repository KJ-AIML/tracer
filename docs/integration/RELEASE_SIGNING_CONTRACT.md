# Release Signing Contract (Gate 2.4.2)

**Gate:** 2.4.2 — Authenticode Signing Readiness  
**Authority:** `tools/release/signing/*` on main after Gate 2.4.2  
**Date:** 2026-07-19

## Modes (mutually distinct)

| Mode | Behavior | Distribution claim |
|---|---|---|
| `UNSIGNED` | Default RC posture. No signature applied. Provenance records explicit unsigned fields. | Internal / development RC only (warnings OK) |
| `SELF_SIGNED_TEST` | Ephemeral test cert; signs **copies** only; proves pipeline mechanics | Never trusted / never public |
| `TRUSTED_AUTHENTICODE` | Real publisher cert or managed signing | Production only when authorized + material present |

Legacy RC class mapping: `SELF_SIGNED_TEST` and `UNSIGNED` → `UNSIGNED_DEVELOPMENT_RC`. Only successful trusted mode maps to `SIGNED`.

## Authorization boundary (trusted)

Trusted signing requires **all** of:

1. Mode `TRUSTED_AUTHENTICODE` (env `TRACER_SIGNING_MODE` or `pnpm release:sign -- --trusted`)
2. Explicit authorization: `TRACER_SIGNING_AUTHORIZED=1` (authoritative gate name; coordinator briefs may say `TRACER_SIGN_RELEASE` — that alias is **not** implemented; use `TRACER_SIGNING_AUTHORIZED`)
3. Certificate material: thumbprint (`TRACER_CODE_SIGN_THUMBPRINT` / `WINDOWS_CERTIFICATE_THUMBPRINT`) **or** PFX path outside repo (`TRACER_CODE_SIGN_CERTIFICATE_PATH`)
4. Not generic CI: refuse when `CI`/`GITHUB_ACTIONS`/`GITLAB_CI`/`TF_BUILD` unless `TRACER_RELEASE_SIGNING_WORKFLOW=1`
5. Signing tool available (`signtool` and/or PowerShell Authenticode)

**Fail closed:** missing auth → `BLOCKED_NO_AUTHORIZATION`; missing cert → `BLOCKED_NO_CERTIFICATE`; generic CI → `BLOCKED_CI_ISOLATION`.

## Entrypoints

| Command | Purpose |
|---|---|
| `pnpm release:sign:doctor` | Non-destructive readiness (never signs) |
| `pnpm release:sign:test` | Isolated self-signed mechanics + cleanup |
| `pnpm release:sign` | Default UNSIGNED no-op; `--trusted` for authorized path |
| `pnpm release:verify-signature` | Independent Authenticode inspection |
| `pnpm test:release:signing` | Deterministic unit/integration tests (no trusted material) |

## Secret-handling (10 rules)

See `docs/validation/release/SIGNING_SECRET_HANDLING.md`. Values of passwords/keys never logged; PFX never under repo; temp material cleaned and verified.

## Timestamp / SmartScreen / publisher

| Concern | Contract status without org material |
|---|---|
| Trusted timestamp | `UNPROVEN` — no network TSA without authorization |
| Publisher identity | `UNPROVEN` |
| SmartScreen | `UNPROVEN` |
| Production distribution signing | `BLOCKED` |

## CI isolation

`pnpm -r test` never trusted-signs. No `.github` trusted signing workflow is active in this repository (classified **not active / absent**).