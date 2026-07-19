# Signing Secret Handling Contract (W2.4.2-A)

## Rules

1. Accept signing material only through explicit operator or secret-store configuration.
2. Never log private-key material.
3. Never log certificate passwords.
4. Redact sensitive environment variables in doctor/provenance dumps.
5. Avoid storing secrets in repository paths (refuse in-repo PFX).
6. Use protected temporary files only when unavoidable (OS temp, mode 0600).
7. Remove temporary signing material after success and failure.
8. Verify cleanup (existence checks after remove).
9. Prevent pull-request and generic CI builds from accessing trusted signing (`CI` without `TRACER_RELEASE_SIGNING_WORKFLOW=1`).
10. Require explicit release-signing authorization (`TRACER_SIGNING_AUTHORIZED=1`).

## Allowed environment variable names (values never documented)

| Name | Role |
|---|---|
| `TRACER_SIGNING_AUTHORIZED` | Explicit trusted-sign gate (`1`/`true`) |
| `TRACER_RELEASE_SIGNING_WORKFLOW` | Marks dedicated release workflow (not PR CI) |
| `TRACER_SIGNING_MODE` | `UNSIGNED` \| `SELF_SIGNED_TEST` \| `TRUSTED_AUTHENTICODE` |
| `TRACER_CODE_SIGN_CERTIFICATE_PATH` | PFX/P12 path **outside** repo |
| `TRACER_CODE_SIGN_CERTIFICATE_PASSWORD` | PFX password (never logged) |
| `TRACER_CODE_SIGN_THUMBPRINT` | Windows store thumbprint |
| `TRACER_CODE_SIGN_SUBJECT` | Expected publisher DN |
| `TRACER_TIMESTAMP_URL` | RFC3161 timestamp server |
| `WINDOWS_CERTIFICATE_THUMBPRINT` | Tauri-compatible alias |

## Never commit

PFX, P12, private keys, passwords, hardware-token PINs, signed production binaries, machine-specific secret paths.
