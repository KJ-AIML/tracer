/**
 * LGJ classification vocabulary (W2.3-B Live Grok GUI).
 * Honest results only — never fabricate PASS for auth/RR gaps.
 */

/** Per-journey / suite classifications for live GUI. */
export const LgjClass = Object.freeze({
  PASS: "PASS",
  PARTIAL: "PARTIAL",
  NOT_RUN: "NOT_RUN",
  NOT_OBSERVED: "NOT_OBSERVED",
  UNSUPPORTED: "UNSUPPORTED",
  BLOCKED_BY_AUTH: "BLOCKED_BY_AUTH",
  BLOCKED_BY_TOOLING: "BLOCKED_BY_TOOLING",
  BLOCKED_BY_PRODUCT_GAP: "BLOCKED_BY_PRODUCT_GAP",
  FAIL: "FAIL",
});

/** Live GUI journey ids. */
export const LgjId = Object.freeze({
  LGJ_01: "LGJ-01",
  LGJ_02: "LGJ-02",
  LGJ_03: "LGJ-03",
  LGJ_04: "LGJ-04",
  LGJ_05: "LGJ-05",
  LGJ_06: "LGJ-06",
  LGJ_07: "LGJ-07",
});

export const LGJ_NAMES = Object.freeze({
  "LGJ-01": "Live runtime readiness",
  "LGJ-02": "Live prompt stream",
  "LGJ-03": "Cancel mid-stream",
  "LGJ-04": "Restart history (no auto re-prompt)",
  "LGJ-05": "Approval reverse-request (honesty)",
  "LGJ-06": "Fail-closed error",
  "LGJ-07": "Clean shutdown",
});

/**
 * Aggregate LGJ results into overall suite classification.
 * @param {{ id: string, result: string }[]} journeys
 */
export function suiteResultFromLgj(journeys) {
  if (!journeys.length) return LgjClass.NOT_RUN;
  const results = journeys.map((j) => j.result);

  if (results.every((r) => r === LgjClass.PASS)) return LgjClass.PASS;
  if (results.every((r) => r === LgjClass.NOT_RUN)) return LgjClass.NOT_RUN;

  if (results.some((r) => r === LgjClass.FAIL)) {
    return LgjClass.FAIL;
  }

  if (
    results.every(
      (r) =>
        r === LgjClass.BLOCKED_BY_AUTH ||
        r === LgjClass.PASS ||
        r === LgjClass.NOT_RUN,
    )
  ) {
    if (results.some((r) => r === LgjClass.BLOCKED_BY_AUTH)) {
      return results.some((r) => r === LgjClass.PASS)
        ? LgjClass.PARTIAL
        : LgjClass.BLOCKED_BY_AUTH;
    }
  }

  if (
    results.every(
      (r) =>
        r === LgjClass.BLOCKED_BY_TOOLING ||
        r === LgjClass.PASS ||
        r === LgjClass.NOT_RUN,
    )
  ) {
    if (results.some((r) => r === LgjClass.BLOCKED_BY_TOOLING)) {
      return results.some((r) => r === LgjClass.PASS)
        ? LgjClass.PARTIAL
        : LgjClass.BLOCKED_BY_TOOLING;
    }
  }

  if (
    results.some(
      (r) =>
        r === LgjClass.NOT_OBSERVED ||
        r === LgjClass.UNSUPPORTED ||
        r === LgjClass.PARTIAL ||
        r === LgjClass.BLOCKED_BY_AUTH ||
        r === LgjClass.BLOCKED_BY_TOOLING ||
        r === LgjClass.BLOCKED_BY_PRODUCT_GAP ||
        r === LgjClass.NOT_RUN,
    )
  ) {
    return LgjClass.PARTIAL;
  }

  return LgjClass.PARTIAL;
}

export function journeyResult(id, result, message, detail = {}) {
  return {
    id,
    name: LGJ_NAMES[id] || id,
    result,
    message,
    detail,
    claimsLiveGui: true,
  };
}

export function pass(id, message, detail) {
  return journeyResult(id, LgjClass.PASS, message, detail);
}
export function fail(id, message, detail) {
  return journeyResult(id, LgjClass.FAIL, message, detail);
}
export function partial(id, message, detail) {
  return journeyResult(id, LgjClass.PARTIAL, message, detail);
}
export function notRun(id, message, detail) {
  return journeyResult(id, LgjClass.NOT_RUN, message, detail);
}
export function notObserved(id, message, detail) {
  return journeyResult(id, LgjClass.NOT_OBSERVED, message, detail);
}
export function unsupported(id, message, detail) {
  return journeyResult(id, LgjClass.UNSUPPORTED, message, detail);
}
export function blockedAuth(id, message, detail) {
  return journeyResult(id, LgjClass.BLOCKED_BY_AUTH, message, detail);
}
export function blockedTooling(id, message, detail) {
  return journeyResult(id, LgjClass.BLOCKED_BY_TOOLING, message, detail);
}
export function blockedProductGap(id, message, detail) {
  return journeyResult(id, LgjClass.BLOCKED_BY_PRODUCT_GAP, message, detail);
}

/**
 * Detect auth-gated session failures from GUI status / error text.
 * @param {string|null|undefined} status
 * @param {string|null|undefined} errorText
 */
export function looksLikeAuthBlock(status, errorText) {
  const s = `${status || ""} ${errorText || ""}`.toLowerCase();
  return (
    /authentication\s*required/.test(s) ||
    /auth(entication)?(\s|_)*(required|error|fail|missing)/.test(s) ||
    /login\s*required/.test(s) ||
    /unauthori[sz]ed/.test(s) ||
    /not\s*logged\s*in/.test(s) ||
    /\bauth\b/.test(s)
  );
}
