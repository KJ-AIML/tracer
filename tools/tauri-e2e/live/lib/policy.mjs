/**
 * Timeout / prompt / run limits for Live Grok GUI (W2.3-B).
 * Applied before any provider-capable path.
 */

/** Max characters for operator --prompt overrides (public-safe bound). */
export const MAX_PROMPT_CHARS = 500;

/** Max live journeys in a single suite invocation. */
export const MAX_JOURNEYS_PER_RUN = 7;

/** Deadlock budget for cancel path (LGJ-03). */
export const CANCEL_DEADLOCK_MS = 45_000;

/** Default wait for session readiness via live bridge. */
export const SESSION_READY_TIMEOUT_MS = 90_000;

/** Default wait for stream / timeline events. */
export const STREAM_EVENT_TIMEOUT_MS = 120_000;

/** Approval RR observation budget (LGJ-05). */
export const APPROVAL_OBSERVE_TIMEOUT_MS = 60_000;

/** WebDriver new-session timeout. */
export const WD_SESSION_TIMEOUT_MS = 120_000;

/** App ready marker timeout. */
export const APP_READY_TIMEOUT_MS = 60_000;

/** Overall soft wall for a full live suite (informational / operator guidance). */
export const SUITE_SOFT_WALL_MS = 30 * 60_000;

/**
 * @param {string|null|undefined} prompt
 * @returns {{ ok: boolean, reason?: string, length: number }}
 */
export function checkPromptBound(prompt) {
  if (prompt == null || prompt === "") {
    return { ok: true, length: 0 };
  }
  const length = String(prompt).length;
  if (length > MAX_PROMPT_CHARS) {
    return {
      ok: false,
      reason: "prompt exceeds MAX_PROMPT_CHARS=" + MAX_PROMPT_CHARS + " (got " + length + ")",
      length,
    };
  }
  return { ok: true, length };
}

/**
 * Orphan process names checked after live teardown (includes stock grok).
 */
export const ORPHAN_CHECK_NAMES = Object.freeze([
  "tracer-desktop",
  "tracer_desktop",
  "tauri-driver",
  "msedgedriver",
  "WebKitWebDriver",
  "grok",
]);