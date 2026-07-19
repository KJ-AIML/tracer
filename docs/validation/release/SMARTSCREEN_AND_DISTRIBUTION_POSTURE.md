# SmartScreen and Distribution Posture (W2.4.2-A)

## Separate layers

| Layer | What it proves | This wave |
|---|---|---|
| Authenticode cryptographic validity | Signature digests match file bytes | Proven via self-signed mechanics |
| Certificate trust | Chain to trusted CA / policy store | **UNPROVEN** (no org cert) |
| Publisher identity | Legal publisher DN matches org | **UNPROVEN** |
| Timestamp validity | Signature verifiable after cert expiry | **UNPROVEN** (TSA not configured/probed) |
| SmartScreen reputation | Microsoft reputation / telemetry | **UNPROVEN** |
| Download-channel reputation | CDN / site reputation | **UNPROVEN** |
| Malware scanning posture | AV / Defender clean verdicts | Outside this task |
| Production distribution readiness | All of the above for public installers | **BLOCKED** |

## Honest SmartScreen classification

**UNPROVEN**

A cryptographically valid Authenticode signature does **not** guarantee absence of SmartScreen warnings. Reputation often requires sustained clean distribution volume and/or EV certificates. Self-signed signatures do not improve SmartScreen posture.

## Classifications used

- `UNPROVEN` — no evidence collected
- `LIMITED` — partial evidence (not claimed here)
- `READY_WITH_EVIDENCE` — documented SmartScreen observations on real distribution (not claimed here)
