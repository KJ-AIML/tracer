# Authenticode Signing Readiness (Gate 2.4.2)

**Date:** 2026-07-19  
**Host:** grok-build / Windows 10.0.26200 win32/x64  
**Integration session:** `heli-ses-26b01af7-555d-440d-a6e0-da64824c2c21`

## Decision summary

| Dimension | Classification |
|---|---|
| Signing pipeline mechanics | **PASS** |
| Self-signed test validation | **PASS** |
| Trusted Authenticode readiness | **BLOCKED_NO_CERTIFICATE** |
| Publisher identity | **UNPROVEN** |
| Timestamp readiness | **UNPROVEN** |
| SmartScreen posture | **UNPROVEN** |
| Production distribution signing | **BLOCKED** |
| Internal unsigned RC distribution | **PASS** (with warnings) |
| Public signed distribution | **BLOCKED** |

## Tooling inventory

| Tool | Result |
|---|---|
| signtool | Detected — `C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe` (Windows Kits 10.0.26100.0); version string via `/?` not machine-readable |
| PowerShell Authenticode | Available — 5.1.26100.8875 |
| AzureSignTool | Not installed |
| Trusted OV/EV / cloud signing provider | **None** |
| Timestamp authority configured | **No** (`TRACER_TIMESTAMP_URL` unset) |

## Doctor

`pnpm release:sign:doctor` → `READY_SELF_SIGNED_TEST_ONLY` / trusted `BLOCKED_NO_CERTIFICATE`. Non-destructive (does not sign).

## Self-signed mechanics proof (integrated tree)

- Copies signed; canonical RC originals byte-unchanged
- Classification: `PRESENT_SELF_SIGNED_UNTRUSTED_ROOT`
- Tamper → `TAMPERED_OR_HASH_MISMATCH`
- Test cert removed from `Cert:\CurrentUser\My` (0 remaining)
- Temp dirs removed

## Explicit non-claims

- No purchase/enrollment of certificates
- No trusted private key used
- No trusted network timestamping
- No SmartScreen observation campaign
- No live Grok GUI / W2.4.3 work

## Related docs

- `docs/integration/RELEASE_SIGNING_CONTRACT.md`
- `docs/validation/release/AUTHENTICODE_SIGNING_RESULTS.md` (W2.4.2-A agent evidence)
- `docs/validation/release/SIGNING_SECRET_HANDLING.md`
- `docs/validation/release/SMARTSCREEN_AND_DISTRIBUTION_POSTURE.md`