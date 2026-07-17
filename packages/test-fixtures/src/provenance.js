/** Evidence provenance labels (TEST_STRATEGY §3.2). */

export const EVIDENCE_LABELS = Object.freeze([
  "synthetic",
  "live-scrubbed",
  "live-authenticated",
  "fake-runtime",
  "unit-generated",
]);

export function isValidEvidence(label) {
  return EVIDENCE_LABELS.includes(label);
}

/**
 * Synthetic / fake evidence must never be claimed as live multi-turn parity.
 */
export function assertNotLiveParityClaim(evidence, claimLiveParity) {
  if (!claimLiveParity) return;
  if (evidence === "synthetic" || evidence === "fake-runtime" || evidence === "unit-generated") {
    throw new Error(
      `Evidence "${evidence}" must not be claimed as live stock Grok multi-turn parity`,
    );
  }
}

/** Gate 0 ACP wire fixture provenance (authoritative until replaced). */
export const ACP_FIXTURE_PROVENANCE = Object.freeze({
  "initialize-request.json": "synthetic",
  "initialize-response.json": "live-scrubbed",
  "session-new-auth-required.json": "live-scrubbed",
  "session-prompt-stream.jsonl": "synthetic",
  "permission-request.json": "synthetic",
  "cancel-notification.json": "synthetic",
});
