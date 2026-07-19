# Wave 2.4.2 Integration Report — Authenticode Signing Readiness

**Gate:** 2.4.2  
**Task:** `tracer-w2-signing-readiness-integration`  
**Work item:** W2.4.2-I  
**Integrator host:** `grok-build`  
**Heli session:** `heli-ses-26b01af7-555d-440d-a6e0-da64824c2c21`  
**Lease:** `heli-lease-3c716867-ddea-4088-9127-0039e03708ab`  
**Write target:** `tracer` (`repos/tracer` main worktree)  
**Integration branch:** `integration/tracer-w2-4-2-signing`  
**Date:** 2026-07-19  
**Platform:** Microsoft Windows 10.0.26200 | rustc/cargo 1.96.0 | Node v24.16.0 | pnpm 9.15.0

> Enforcement note: Heli-Harness SessionStart plugin context was **not** injected this session — guardrails treated as advisory; discipline applied (no push; no secrets; target `repos/tracer` only).

## 1. Gate 2.4.2 decision

| Field | Value |
|---|---|
| **Gate 2.4.2** | **PASS** |
| Signing pipeline mechanics | **PASS** |
| Self-signed test validation | **PASS** |
| Trusted Authenticode readiness | **BLOCKED_NO_CERTIFICATE** |
| Publisher identity | **UNPROVEN** |
| Timestamp readiness | **UNPROVEN** |
| SmartScreen posture | **UNPROVEN** |
| Production distribution signing | **BLOCKED** |
| Internal unsigned RC | **READY_WITH_WARNINGS** |
| Public signed distribution | **BLOCKED** |
| Standard CI isolation | **PASS** (trusted workflow **absent / not active**) |
| Upgrade + provenance regression | **PASS** |

## 2. Binding / lease

| Check | Result |
|---|---|
| Task create | `tracer-w2-signing-readiness-integration` / `W2.4.2-I` / repo `tracer` / worktree `repos/tracer` |
| Claim | write / host `grok-build` |
| Session | `heli-ses-26b01af7-555d-440d-a6e0-da64824c2c21` |
| Target | `tracer` → writes under `repos/tracer` |
| Source branch | `agent/tracer-w2-signing-readiness` @ `6ed4f7acbc2308a07f5f48bba2f82abe564c20a7` |
| Baseline main | `d83a873f0cbad9478ee311315e53f6ca22035970` (Gate 2.4.1 PASS) |
| Push | **Never** |

## 3. Gate 2.4.1 tag verification (Part 3)

```
git ls-remote --tags origin "tracer-wave2.4.1-upgrade-verified*"
```

| Item | Value |
|---|---|
| Annotated tag object | `d1e020d9fa2bf425afc7b33155c255e3caf01a41` |
| Remote peeled `^{commit}` | `d83a873f0cbad9478ee311315e53f6ca22035970` |
| Local peeled `^{commit}` | `d83a873f0cbad9478ee311315e53f6ca22035970` |
| **Classification** | **ALIGNED** (not pushed / not modified during Gate 2.4.2) |

## 4. Gate 2.4.1 Heli provenance (Part 4 — historical)

| Role | Session |
|---|---|
| Initial claim | `heli-ses-ee781bf9` |
| Takeover / completion | `heli-ses-26b01af7` (Gate 2.4.1 integration; lease released 2026-07-19) |

Not a typo. Gate 2.4.1 historical reports were **not** rewritten. This Gate 2.4.2 session reuses a new full id under the same host pool prefix: `heli-ses-26b01af7-555d-440d-a6e0-da64824c2c21`.

## 5. Merge + reconciliation

| Role | SHA | Message |
|---|---|---|
| Merge (`--no-ff`) | `224cdab6a4eb44e615689f9a7be9f080b6ecb7da` | `merge(w2.4.2-i): integrate W2.4.2-A Authenticode signing readiness` |
| Provenance schema reconcile | `7419bb991edbf8dec36ee85f0deda09aa81f6623` (reports); tip `fe89d1f3845d00b61779c70a7fc8dc4fa8d2efdd` | artifact signing fields made explicit in fixture schema |
| Gate reports | `7419bb991edbf8dec36ee85f0deda09aa81f6623` (reports); tip `fe89d1f3845d00b61779c70a7fc8dc4fa8d2efdd` | Gate 2.4.2 integration artifacts |

No squash. Scope audit: OWN paths only (`tools/release/signing/`, `tools/release/`, `tests/release/windows/signing/`, `docs/modules/w2-4-2/`, `docs/validation/release/`, minimal root scripts / `.gitignore` / `digestAlgorithm: sha256`). No unrelated domain/process/storage/runtime/control-plane/desktop `src` changes.

## 6. Signing doctor / tools

| Field | Value |
|---|---|
| Doctor classification | `READY_SELF_SIGNED_TEST_ONLY` |
| Trusted readiness | `BLOCKED_NO_CERTIFICATE` |
| Preferred tool | signtool |
| signtool path | `C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe` |
| signtool version | not machine-readable via `/?` (path authoritative; Kits `10.0.26100.0`) |
| PowerShell Authenticode | 5.1.26100.8875 |

## 7. Self-signed integration proof (Part 12)

Evidence file: `target/release-rc/windows/signing-self-signed-test.json` (local, not committed).

| Artifact | Pre-sign SHA-256 (canonical unsigned) | Post-sign copy SHA-256 |
|---|---|---|
| portable `tracer-desktop.exe` | `507aeadd2a7585b923f37f51e141a77447a3b57fa011c64f202027f0e176ccbb` | `8aed61396ef103ec8f2276a3a160b48998fa5aba69ecfc314ca9cf9f76797b1d` |
| NSIS `Tracer_0.1.1_x64-setup.exe` | `c897e6a832d284b2c46df6444e4893e7b188018ddf9aad9a5ff0b46f925d1f6c` | `623f58a29e11bb8ca2e6db6b2d373ec37c236c3422fa1a75800639e2cc049352` |

| Check | Result |
|---|---|
| Originals unchanged byte-for-byte | **true** |
| Signature classification | `PRESENT_SELF_SIGNED_UNTRUSTED_ROOT` |
| Tamper detection | **PASS** → `TAMPERED_OR_HASH_MISMATCH` |
| Secret / temp cleanup | **PASS** |
| Cert store after cleanup | **0** `Tracer Self-Signed` certs in `Cert:\CurrentUser\My` |
| Signing tool used for test | PowerShell `Set-AuthenticodeSignature` 5.1.26100.8875 |

Note: a later `pnpm test:release:upgrade` rebuild changed on-disk unsigned hashes (rebuild-clock sensitive). Signing proof hashes above are the authoritative Gate 2.4.2 mechanics evidence.

## 8. Authorization boundary (Part 10)

Audited against `tools/release/signing/secrets.mjs` 10 rules + `sign.mjs` fail-closed path. Live check without material:

- `TRACER_SIGNING_MODE=TRUSTED_AUTHENTICODE` without `TRACER_SIGNING_AUTHORIZED=1` → `BLOCKED_NO_AUTHORIZATION` (exit 2).
- Authoritative auth env: **`TRACER_SIGNING_AUTHORIZED=1`** (not `TRACER_SIGN_RELEASE`).

Trusted signing was **not** executed with real authorized material.

## 9. Provenance / hygiene / CI

| Check | Result |
|---|---|
| Unsigned provenance signing fields | Explicit (`signaturePresent=false`, `signingMode=UNSIGNED`, null cert/timestamp) |
| Hygiene (stage JSON) | No PEM / password secrets |
| `.github` trusted signing workflow | **Absent / not active** |
| `pnpm -r test` trusted-sign | **Never** |

## 10. Aggregate validation summary

| Suite | Result |
|---|---|
| cargo fmt / check / test / clippy | PASS |
| pnpm install / `-r test` / `-r build` | PASS |
| release:windows | PASS |
| release:sign:doctor / sign:test / verify-signature | PASS |
| test:release:signing | PASS 13/13 |
| release:provenance | PASS |
| test:release:upgrade | PASS |

Material flags: trusted cert=**no**; real private key=**no**; trusted timestamp=**no**; provider=**no**; live Grok=**no**.

## 11. Finalize plan

| Step | Result |
|---|---|
| FF main ← `integration/tracer-w2-4-2-signing` | after report commit |
| Tag | `tracer-wave2.4.2-signing-pipeline-verified` (annotated, **local**) |
| Keep | `tracer-wave2.4.1-upgrade-verified` untouched |
| Lease release | `tracer-w2-signing-readiness-integration` |
| Push | **Never** |

## 12. Commit pin table

| Role | SHA |
|---|---|
| Baseline main (Gate 2.4.1) | `d83a873f0cbad9478ee311315e53f6ca22035970` |
| W2.4.2-A tip | `6ed4f7acbc2308a07f5f48bba2f82abe564c20a7` |
| Merge | `224cdab6a4eb44e615689f9a7be9f080b6ecb7da` |
| Gate 2.4.2 reports / schema | `7419bb991edbf8dec36ee85f0deda09aa81f6623` (reports); tip `fe89d1f3845d00b61779c70a7fc8dc4fa8d2efdd` |

## 13. Residual risks

1. No organization code-signing certificate — production Authenticode remains blocked.
2. Timestamp authority not configured or live-probed for trusted signing.
3. SmartScreen reputation unproven even after a future trusted signature.
4. Artifact SHA-256 values are rebuild-sensitive; record per build.
5. Clippy warnings remain in unrelated crates (soft clippy PASS only).
6. Coordinator brief name `TRACER_SIGN_RELEASE` differs from implemented `TRACER_SIGNING_AUTHORIZED` — operators must use the implemented name.

**Final tip before FF:** `fe89d1f3845d00b61779c70a7fc8dc4fa8d2efdd`.
