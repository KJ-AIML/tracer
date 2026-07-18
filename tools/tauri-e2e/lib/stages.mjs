/**
 * Ordered stage runner with distinct failure classification per stage.
 */

import { ResultClass, StageId, suiteResultFromStages } from "./classify.mjs";

/**
 * @typedef {object} StageResult
 * @property {string} id
 * @property {string} status  pass|fail|skip|partial|blocked_tooling|blocked_webview|unsupported
 * @property {string} [classification]
 * @property {string} [message]
 * @property {number} [durationMs]
 * @property {object} [detail]
 */

export function createStageReport() {
  /** @type {StageResult[]} */
  const stages = [];
  return {
    stages,
    async run(id, fn, opts = {}) {
      const start = Date.now();
      try {
        const detail = await fn();
        const status = detail?.status || "pass";
        const entry = {
          id,
          status,
          classification: detail?.classification || mapStatus(status),
          message: detail?.message,
          durationMs: Date.now() - start,
          detail: detail?.detail,
        };
        stages.push(entry);
        return entry;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        const status = opts.onErrorStatus || "fail";
        const entry = {
          id,
          status,
          classification: opts.onErrorClass || mapStatus(status),
          message: msg,
          durationMs: Date.now() - start,
          detail: opts.onErrorDetail,
        };
        stages.push(entry);
        if (opts.rethrow !== false && status === "fail") throw e;
        return entry;
      }
    },
    skip(id, message, classification = ResultClass.BLOCKED_BY_TOOLING) {
      const entry = {
        id,
        status: "skip",
        classification,
        message,
        durationMs: 0,
      };
      stages.push(entry);
      return entry;
    },
    summary() {
      return {
        stages,
        result: suiteResultFromStages(stages),
      };
    },
  };
}

function mapStatus(status) {
  switch (status) {
    case "pass":
      return ResultClass.PASS;
    case "partial":
      return ResultClass.PARTIAL;
    case "blocked_tooling":
      return ResultClass.BLOCKED_BY_TOOLING;
    case "blocked_webview":
      return ResultClass.BLOCKED_BY_WEBVIEW;
    case "unsupported":
      return ResultClass.UNSUPPORTED_PLATFORM;
    case "skip":
      return ResultClass.BLOCKED_BY_TOOLING;
    default:
      return ResultClass.FAIL;
  }
}

export { StageId };
