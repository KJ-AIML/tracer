/**
 * W2.4.2-A Authenticode signing modes (authoritative).
 *
 * UNSIGNED            — default RC posture; no signature claimed
 * SELF_SIGNED_TEST    — pipeline mechanics only; never trusted distribution
 * TRUSTED_AUTHENTICODE — real publisher cert / managed signing; fails closed without material
 */

export const SIGNING_MODES = Object.freeze({
  UNSIGNED: "UNSIGNED",
  SELF_SIGNED_TEST: "SELF_SIGNED_TEST",
  TRUSTED_AUTHENTICODE: "TRUSTED_AUTHENTICODE",
});

/** Legacy W2.3-A / Gate 2.4.1 classes (still emitted for RC classify). */
export const LEGACY_SIGNING_CLASSES = Object.freeze({
  SIGNED: "SIGNED",
  UNSIGNED_DEVELOPMENT_RC: "UNSIGNED_DEVELOPMENT_RC",
  BLOCKED: "BLOCKED",
});

/** Doctor / readiness classifications (Part 8). */
export const DOCTOR_CLASSES = Object.freeze({
  READY_WITH_CERTIFICATE: "READY_WITH_CERTIFICATE",
  READY_SELF_SIGNED_TEST_ONLY: "READY_SELF_SIGNED_TEST_ONLY",
  BLOCKED_NO_CERTIFICATE: "BLOCKED_NO_CERTIFICATE",
  BLOCKED_NO_SIGNING_TOOL: "BLOCKED_NO_SIGNING_TOOL",
  BLOCKED_NO_PUBLISHER_IDENTITY: "BLOCKED_NO_PUBLISHER_IDENTITY",
  BLOCKED_TIMESTAMP_CONFIGURATION: "BLOCKED_TIMESTAMP_CONFIGURATION",
  BLOCKED_SIGNING_PROVIDER: "BLOCKED_SIGNING_PROVIDER",
  UNSUPPORTED_PLATFORM: "UNSUPPORTED_PLATFORM",
  FAIL: "FAIL",
});

/** Trusted Authenticode readiness (Part 17). */
export const TRUSTED_READINESS = Object.freeze({
  READY_WITH_CERTIFICATE: "READY_WITH_CERTIFICATE",
  BLOCKED_NO_CERTIFICATE: "BLOCKED_NO_CERTIFICATE",
  BLOCKED_NO_PUBLISHER_IDENTITY: "BLOCKED_NO_PUBLISHER_IDENTITY",
  BLOCKED_SIGNING_PROVIDER: "BLOCKED_SIGNING_PROVIDER",
  BLOCKED_TIMESTAMP_CONFIGURATION: "BLOCKED_TIMESTAMP_CONFIGURATION",
  FAIL: "FAIL",
});

export const TEST_ONLY_SUBJECT = "CN=Tracer Self-Signed Test Only, O=Tracer Test, OU=W2.4.2-A";

export function isSigningMode(value) {
  return Object.values(SIGNING_MODES).includes(value);
}

/**
 * Map mode → legacy RC class for Gate 2.4.1 compatibility.
 */
export function modeToLegacyClass(mode) {
  if (mode === SIGNING_MODES.TRUSTED_AUTHENTICODE) {
    return LEGACY_SIGNING_CLASSES.SIGNED;
  }
  if (mode === SIGNING_MODES.SELF_SIGNED_TEST) {
    // Self-signed is NOT production SIGNED.
    return LEGACY_SIGNING_CLASSES.UNSIGNED_DEVELOPMENT_RC;
  }
  return LEGACY_SIGNING_CLASSES.UNSIGNED_DEVELOPMENT_RC;
}