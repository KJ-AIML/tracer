# W2.4.2 Test Matrix - Authenticode Signing

| ID | Case | Expected | Suite |
|---|---|---|---|
| SG-01 | Unsigned artifact classification | UNSIGNED / explicit provenance fields | unit + verify-signature |
| SG-02 | Self-signed test verification | PASS mechanics; SELF_SIGNED_TEST | release:sign:test |
| SG-03 | Tampered artifact rejection | HashMismatch / rejected | release:sign:test |
| SG-04 | Wrong certificate subject | WRONG_CERTIFICATE_SUBJECT | unit |
| SG-05 | Expired certificate | CERTIFICATE_EXPIRED | unit |
| SG-06 | Not-yet-valid certificate | CERTIFICATE_NOT_YET_VALID | unit |
| SG-07 | Missing timestamp (when required) | MISSING_TIMESTAMP | unit |
| SG-08 | Signing tool unavailable | BLOCKED_NO_SIGNING_TOOL | doctor + unit |
| SG-09 | Certificate unavailable | BLOCKED_NO_CERTIFICATE | doctor |
| SG-10 | Publisher identity unavailable | UNPROVEN / BLOCKED_NO_PUBLISHER_IDENTITY | doctor |
| SG-11 | Secret redaction | no password/PEM in dumps | unit |
| SG-12 | Temporary secret cleanup | temp + store cert removed | release:sign:test |
| SG-13 | Generic CI isolation | trusted sign forbidden | unit |
| SG-14 | Manifest signature fields | unsigned explicit false/UNSIGNED | provenance |
| SG-15 | pnpm -r test never trusted-signs | no auth path | CI isolation |

## Commands

```text
pnpm test:release:signing
pnpm release:sign:doctor
pnpm release:sign:test
```

Trusted Authenticode is out of standard CI.
