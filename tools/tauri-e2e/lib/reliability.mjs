/**
 * W2.3-C reliability helpers:
 * - state-based waits with timeouts (no fixed sleep as sole control)
 * - fresh-env bookkeeping
 * - failure-injection (harness-only; never masks product flakes)
 * - temp cleanup verification
 * - Edge-update resilience advisory
 */

import {
  existsSync,
  mkdirSync,
  rmSync,
  writeFileSync,
  readdirSync,
  statSync,
} from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import {
  detectEdgeVersionWindows,
  evaluateEdgeDriverCompatibility,
  readMsEdgeDriverVersion,
  resolveLocalMsEdgeDriver,
} from "../../tauri-driver/lib/edge.mjs";
import { findOrphans, reapOrphans } from "./process.mjs";

/**
 * Poll async predicate until true or timeout.
 * Prefer this over bare `await delay(N)` for readiness.
 *
 * @param {() => boolean | Promise<boolean>} pred
 * @param {{ timeoutMs?: number, intervalMs?: number, label?: string }} [opts]
 */
export async function waitUntil(pred, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 30_000;
  const intervalMs = opts.intervalMs ?? 250;
  const label = opts.label || "condition";
  const deadline = Date.now() + timeoutMs;
  let lastErr = null;
  while (Date.now() < deadline) {
    try {
      if (await pred()) return true;
    } catch (e) {
      lastErr = e;
    }
    await delay(intervalMs);
  }
  const detail =
    lastErr instanceof Error
      ? lastErr.message
      : lastErr
        ? String(lastErr)
        : "predicate false";
  throw new Error(`waitUntil(${label}) timeout after ${timeoutMs}ms: ${detail}`);
}

/**
 * Bounded delay used only as a backoff slice between state polls
 * (never as the sole readiness signal).
 * @param {number} ms
 */
export function backoff(ms) {
  return delay(Math.max(0, Math.min(ms, 5_000)));
}

/** Orphan image names checked after GUI teardown. */
export const ORPHAN_CHECK_NAMES = Object.freeze([
  "tracer-desktop",
  "tracer_desktop",
  "tauri-driver",
  "msedgedriver",
  "WebKitWebDriver",
]);

/**
 * Failure-injection modes (harness reliability only).
 * Never used to re-run product asserts or mask flakes.
 *
 * TRACER_E2E_INJECT modes (harness only; never masks product flakes):
 *   none | orphan_leak | port_hold | artifact_secret | mid_journey_kill
 *   app_launch_failure | tauri_driver_startup_failure | msedgedriver_startup_failure
 *   root_marker_missing | fake_runtime_crash | sqlite_unavailable
 *   forced_gui_assertion_failure | shutdown_timeout | stale_edge_driver
 */
export const INJECT_MODES = Object.freeze([
  "none",
  "orphan_leak",
  "port_hold",
  "artifact_secret",
  "mid_journey_kill",
  "app_launch_failure",
  "tauri_driver_startup_failure",
  "msedgedriver_startup_failure",
  "root_marker_missing",
  "fake_runtime_crash",
  "sqlite_unavailable",
  "forced_gui_assertion_failure",
  "shutdown_timeout",
  "stale_edge_driver",
]);

export function parseInjectMode(raw = process.env.TRACER_E2E_INJECT) {
  const v = String(raw || "none").trim().toLowerCase();
  const allowed = new Set(INJECT_MODES);
  if (!allowed.has(v)) {
    return { mode: "none", raw: v, invalid: true };
  }
  return { mode: v, raw: v, invalid: false };
}

/**
 * Expected classification for each deterministic inject mode (C6 contract).
 * Product asserts are first-attempt FAIL; tooling mismatches are BLOCKED_BY_TOOLING.
 */
export function injectClassification(mode) {
  const m = parseInjectMode(mode).mode;
  /** @type {Record<string, { failureCode: string, result: string, retries: number, cleanupRequired: boolean }>} */
  const map = {
    none: {
      failureCode: null,
      result: "PASS",
      retries: 0,
      cleanupRequired: false,
    },
    orphan_leak: {
      failureCode: "ORPHAN_PROCESS",
      result: "PARTIAL",
      retries: 0,
      cleanupRequired: true,
    },
    port_hold: {
      failureCode: "PORT_IN_USE",
      result: "PASS",
      retries: 0,
      cleanupRequired: true,
    },
    artifact_secret: {
      failureCode: null,
      result: "PASS",
      retries: 0,
      cleanupRequired: false,
    },
    mid_journey_kill: {
      failureCode: "DRIVER_STARTUP_FAILED",
      result: "FAIL",
      retries: 0,
      cleanupRequired: true,
    },
    app_launch_failure: {
      failureCode: "APP_LAUNCH_FAILED",
      result: "FAIL",
      retries: 0,
      cleanupRequired: true,
    },
    tauri_driver_startup_failure: {
      failureCode: "DRIVER_STARTUP_FAILED",
      result: "BLOCKED_BY_TOOLING",
      retries: 0,
      cleanupRequired: true,
    },
    msedgedriver_startup_failure: {
      failureCode: "MSEDGEDRIVER_STARTUP_FAILED",
      result: "BLOCKED_BY_TOOLING",
      retries: 0,
      cleanupRequired: true,
    },
    root_marker_missing: {
      failureCode: "ROOT_MARKER_MISSING",
      result: "FAIL",
      retries: 0,
      cleanupRequired: true,
    },
    fake_runtime_crash: {
      failureCode: "FAKE_RUNTIME_CRASH",
      result: "FAIL",
      retries: 0,
      cleanupRequired: true,
    },
    sqlite_unavailable: {
      failureCode: "SQLITE_UNAVAILABLE",
      result: "FAIL",
      retries: 0,
      cleanupRequired: true,
    },
    forced_gui_assertion_failure: {
      failureCode: "GUI_ASSERTION_FAILED",
      result: "FAIL",
      retries: 0,
      cleanupRequired: true,
    },
    shutdown_timeout: {
      failureCode: "SHUTDOWN_TIMEOUT",
      result: "FAIL",
      retries: 0,
      cleanupRequired: true,
    },
    stale_edge_driver: {
      failureCode: "EDGE_DRIVER_VERSION_MISMATCH",
      result: "BLOCKED_BY_TOOLING",
      retries: 0,
      cleanupRequired: false,
    },
  };
  return map[m] || map.none;
}

/**
 * Detect Edge vs msedgedriver drift after Windows Edge auto-update.
 * Returns actionable remediation without installing (plan-only).
 */
export function edgeUpdateResilienceProbe() {
  if (process.platform !== "win32") {
    return {
      platform: process.platform,
      applicable: false,
      compatible: true,
      code: "N/A",
      message: "Edge/msedgedriver resilience is Windows-only",
      remediation: null,
    };
  }
  const edge = detectEdgeVersionWindows();
  const driverPath =
    process.env.TRACER_NATIVE_DRIVER || resolveLocalMsEdgeDriver();
  const driver = readMsEdgeDriverVersion(driverPath);
  const compat = evaluateEdgeDriverCompatibility(edge, {
    ...driver,
    path: driverPath,
  });
  const remediation = compat.compatible
    ? null
    : {
        command: "node tools/tauri-e2e/doctor.mjs --apply",
        alt: "node tools/tauri-driver/setup.mjs --apply",
        env: "TRACER_TAURI_E2E_SETUP=1",
        note:
          "Re-download project-local msedgedriver matching current Edge major after Edge auto-update",
      };
  return {
    platform: "win32",
    applicable: true,
    edge: {
      available: edge.available,
      version: edge.version,
      major: edge.major,
      method: edge.method,
    },
    driver: {
      available: driver.available,
      version: driver.version,
      major: driver.major,
      pathPresent: Boolean(driverPath),
    },
    compatible: Boolean(compat.compatible),
    code: compat.code,
    message: compat.message,
    rule: compat.rule || "major(msedgedriver) == major(Edge)",
    remediation,
  };
}

/**
 * Remove temp workDir and report outcome.
 * @param {string} workDir
 * @param {{ keep?: boolean }} [opts]
 */
export function cleanupTempDir(workDir, opts = {}) {
  const keep =
    opts.keep === true ||
    process.env.TRACER_E2E_KEEP_TEMP === "1" ||
    process.env.TRACER_E2E_KEEP_TEMP === "true";
  if (!workDir) {
    return { attempted: false, cleaned: false, reason: "no_workDir" };
  }
  if (keep) {
    return { attempted: false, cleaned: false, reason: "keep_temp", workDir };
  }
  if (!existsSync(workDir)) {
    return { attempted: true, cleaned: true, reason: "already_gone", workDir };
  }
  try {
    rmSync(workDir, { recursive: true, force: true });
    const gone = !existsSync(workDir);
    return {
      attempted: true,
      cleaned: gone,
      reason: gone ? "removed" : "still_present",
      workDir,
    };
  } catch (e) {
    return {
      attempted: true,
      cleaned: false,
      reason: "error",
      error: e instanceof Error ? e.message : String(e),
      workDir,
    };
  }
}

/**
 * Verify orphans are gone; optionally reap once (reported, not silent PASS).
 * @param {string[]} [names]
 * @param {{ reap?: boolean }} [opts]
 */
export function verifyNoOrphans(names = ORPHAN_CHECK_NAMES, opts = {}) {
  let found = findOrphans(names);
  /** @type {any[]} */
  let reaped = [];
  if (found.length && opts.reap !== false) {
    reaped = reapOrphans(names);
    found = findOrphans(names);
  }
  return {
    ok: found.length === 0,
    remaining: found,
    reaped,
    names: [...names],
  };
}

/**
 * Build a fresh-env run record skeleton for first-attempt reporting.
 * @param {{ runIndex: number, runId: string, port: number, workDir: string, artifactsDir: string }} meta
 */
export function freshEnvRunRecord(meta) {
  return {
    schemaVersion: 1,
    module: "W2.3-C",
    attempt: "first",
    runIndex: meta.runIndex,
    runId: meta.runId,
    startedAt: new Date().toISOString(),
    finishedAt: null,
    port: meta.port,
    workDir: meta.workDir,
    artifactsDir: meta.artifactsDir,
    result: null,
    journeys: [],
    productAssertionFailures: 0,
    orphans: null,
    portCollisions: 0,
    tempCleanup: null,
    edge: null,
    inject: parseInjectMode(),
    retries: 0,
    timing: {
      driverStartupMs: null,
      appReadinessMs: null,
      suiteMs: null,
      shutdownMs: null,
    },
    notes: [],
  };
}

/**
 * Timing / wait policy documentation helper (C5).
 * Fixed delays are backoff slices only; readiness uses state polls.
 */
export const WAIT_POLICY = Object.freeze({
  driverReady: {
    expectedState: "tauri-driver HTTP /status responds",
    mechanism: "waitDriverReady poll",
    timeoutMs: 30_000,
    failureCode: "DRIVER_STARTUP_FAILED",
  },
  appReady: {
    expectedState: "DOM [data-testid=tracer-app-ready] present",
    mechanism: "waitForTestId / waitAppReady",
    timeoutMs: 60_000,
    failureCode: "ROOT_MARKER_MISSING",
  },
  desktopExitBeforeRelaunch: {
    expectedState: "no tracer-desktop orphan PIDs",
    mechanism: "waitUntil(findOrphans empty) + reap",
    timeoutMs: 15_000,
    failureCode: "ORPHAN_PROCESS",
  },
  sessionStatus: {
    expectedState: "data-session-status matches predicate",
    mechanism: "waitSessionStatus poll + refresh",
    timeoutMs: 60_000,
    failureCode: "GUI_ASSERTION_FAILED",
  },
  shutdown: {
    expectedState: "WebDriver session deleted; driver PID dead",
    mechanism: "deleteSession + waitUntil(!processAlive)",
    timeoutMs: 30_000,
    failureCode: "SHUTDOWN_TIMEOUT",
  },
});

/**
 * Count product assertion failures from journey results.
 * Tooling blocks / unsupported are not product asserts.
 * @param {Array<{ result?: string }>} journeys
 */
export function countProductAssertionFailures(journeys) {
  let n = 0;
  for (const j of journeys || []) {
    if (j.result === "FAIL" || j.result === "BLOCKED_BY_PRODUCT_GAP") n += 1;
  }
  return n;
}

/**
 * Write JSON report for a reliability batch.
 * @param {string} outPath
 * @param {object} report
 */
export function writeReliabilityReport(outPath, report) {
  mkdirSync(path.dirname(outPath), { recursive: true });
  writeFileSync(outPath, JSON.stringify(report, null, 2), "utf8");
  return outPath;
}

/**
 * List leftover temp dirs matching tracer-l3j prefix under TEMP (diagnostic).
 * @param {string} [prefix]
 */
export function listStaleTempDirs(prefix = "tracer-l3j") {
  const base = process.env.TEMP || process.env.TMPDIR || process.env.TMP;
  if (!base || !existsSync(base)) return [];
  const out = [];
  try {
    for (const name of readdirSync(base)) {
      if (!name.startsWith(prefix)) continue;
      const full = path.join(base, name);
      try {
        if (statSync(full).isDirectory()) out.push(full);
      } catch {
        /* ignore */
      }
    }
  } catch {
    /* ignore */
  }
  return out;
}
