# Authenticode Signing Results (W2.4.2-A)

**Date:** 2026-07-19
**Host OS:** Windows 10.0.26200 (win32/x64)
**Heli session:** heli-ses-47e0f854-d596-44df-a3c2-5a6c3f0c956f
**Branch:** agent/tracer-w2-signing-readiness
**Base:** d83a873f0cbad9478ee311315e53f6ca22035970

## Signing doctor

| Field | Value |
|---|---|
| classification | READY_SELF_SIGNED_TEST_ONLY |
| trustedReadiness | BLOCKED_NO_CERTIFICATE |
| preferred tool | signtool |
| tool path | `C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe` |
| PowerShell Authenticode | available (5.1.26100.8875) |
| certificate category | NONE |
| publisher | UNPROVEN |
| timestamp | UNPROVEN |
| secrets logged | false |

## Self-signed mechanics

| Field | Value |
|---|---|
| pipelineMechanics | PASS |
| selfSignedTestValidation | PASS |
| tamperDetection | PASS |
| secretCleanup | PASS |
| trustedDistributionSigning | BLOCKED_NO_CERTIFICATE |
| originalsUnchanged | true |
| signing tool | powershell Set-AuthenticodeSignature |

## Unsigned RC artifacts (post release:windows)

| Artifact | sha256 | signaturePresent | signingMode |
|---|---|---|---|
| tracer-desktop.exe | `a24f953f023a9045f8101d1054f1d3c955ba7bff635acc8405d1c6740d657465` | false | UNSIGNED |
| Tracer_0.1.1_x64-setup.exe | `0176a92a323b025e7f34edc258f3fbfde7ebe2cfd658fb131e7c8c18756d8941` | false | UNSIGNED |

## Trusted Authenticode

Not executed. Blocker: **BLOCKED_NO_CERTIFICATE**.

## Network

Doctor/self-signed test did not contact timestamp authorities for trusted signing. NSIS tooling may download bundler helpers during `pnpm release:windows` (unrelated to certificate procurement).

## Standard CI isolation

Unit tests under `tests/release/windows/signing` do not invoke trusted signing. Generic CI without `TRACER_RELEASE_SIGNING_WORKFLOW=1` is refused.
