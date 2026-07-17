/**
 * Fixed identities for deterministic fake ACP runs.
 * Align with tests/fixtures/acp/* and expected-events packs.
 */

export const PROTOCOL_VERSION = 1;

export const FIXED = Object.freeze({
  sessionId: "11111111-1111-4111-8111-111111111111",
  promptId: "22222222-2222-4222-8222-222222222222",
  agentId: "00000000-0000-4000-8000-000000000001",
  agentInstanceId: "00000000-0000-4000-8000-000000000002",
  toolCallList: "call_example_list",
  toolCallEdit: "call_example_edit",
  projectRootPlaceholder: "{{PROJECT_ROOT}}",
  hostnamePlaceholder: "{{HOSTNAME}}",
  agentVersion: "0.0.0-fake-acp",
});

/** Catalog scenario IDs implemented by this binary (standardCi only). */
export const IMPLEMENTED_SCENARIOS = Object.freeze([
  "happy_prompt_stream",
  "auth_required_session_new",
  "permission_allow",
  "permission_deny",
  "cancel_mid_stream",
  "cancel_while_permission_pending",
  "malformed_frame",
  "unknown_vendor_notification",
  "eof_mid_prompt",
  "crash_nonzero_exit",
  "cancel_unsupported",
  "slow_cancel_ack",
  "duplicate_response_id",
  "capability_minimal",
  "clean_shutdown_stdin_close",
]);

/** Live / optional scenarios — intentionally NOT implemented in the fake. */
export const LIVE_ONLY_SCENARIOS = Object.freeze([
  "live_stock_auth_prompt",
  "live_stock_auth_required_reprobe",
]);

export const EVIDENCE = Object.freeze({
  fakeRuntime: "fake-runtime",
  synthetic: "synthetic",
  liveScrubbed: "live-scrubbed",
  liveAuthenticated: "live-authenticated",
});
