/**
 * W2.2-T classification vocabularies.
 * Honest results only — never emit PASS when tooling blocks.
 */

/** Doctor readiness / preflight classifications. */
export const DoctorClass = Object.freeze({
  READY: "READY",
  MISSING_TOOL: "MISSING_TOOL",
  INCOMPATIBLE_VERSION: "INCOMPATIBLE_VERSION",
  WEBVIEW_UNAVAILABLE: "WEBVIEW_UNAVAILABLE",
  DRIVER_UNAVAILABLE: "DRIVER_UNAVAILABLE",
  BUILD_REQUIRED: "BUILD_REQUIRED",
  UNSUPPORTED_PLATFORM: "UNSUPPORTED_PLATFORM",
});

/** Suite / stage result classifications. */
export const ResultClass = Object.freeze({
  PASS: "PASS",
  PARTIAL: "PARTIAL",
  BLOCKED_BY_TOOLING: "BLOCKED_BY_TOOLING",
  BLOCKED_BY_WEBVIEW: "BLOCKED_BY_WEBVIEW",
  BLOCKED_BY_PRODUCT_GAP: "BLOCKED_BY_PRODUCT_GAP",
  BLOCKED_BY_FIXTURE: "BLOCKED_BY_FIXTURE",
  UNSUPPORTED_PLATFORM: "UNSUPPORTED_PLATFORM",
  FAIL: "FAIL",
  NOT_STARTED: "NOT_STARTED",
});

/** L3-J product journey ids. */
export const JourneyId = Object.freeze({
  GJ_01: "GJ-01",
  GJ_02: "GJ-02",
  GJ_03: "GJ-03",
  GJ_04: "GJ-04",
  GJ_05: "GJ-05",
  GJ_06: "GJ-06",
  GJ_07: "GJ-07",
  GJ_08: "GJ-08",
  GJ_09: "GJ-09",
  GJ_10: "GJ-10",
  GJ_11: "GJ-11",
  GJ_12: "GJ-12",
});

/** Precise failure / issue codes (doctor + runners). */
export const FailureCode = Object.freeze({
  TAURI_DRIVER_NOT_FOUND: "TAURI_DRIVER_NOT_FOUND",
  TAURI_DRIVER_INSTALL_FAILED: "TAURI_DRIVER_INSTALL_FAILED",
  EDGE_BROWSER_NOT_FOUND: "EDGE_BROWSER_NOT_FOUND",
  EDGE_BROWSER_VERSION_UNKNOWN: "EDGE_BROWSER_VERSION_UNKNOWN",
  EDGE_DRIVER_NOT_FOUND: "EDGE_DRIVER_NOT_FOUND",
  EDGE_DRIVER_VERSION_MISMATCH: "EDGE_DRIVER_VERSION_MISMATCH",
  EDGE_DRIVER_VERSION_UNVERIFIED: "EDGE_DRIVER_VERSION_UNVERIFIED",
  EDGE_DRIVER_DOWNLOAD_FAILED: "EDGE_DRIVER_DOWNLOAD_FAILED",
  WEBVIEW2_NOT_FOUND: "WEBVIEW2_NOT_FOUND",
  APP_BINARY_NOT_FOUND: "APP_BINARY_NOT_FOUND",
  FRONTEND_DIST_NOT_FOUND: "FRONTEND_DIST_NOT_FOUND",
  PORT_IN_USE: "PORT_IN_USE",
  PORT_CHECK_FAILED: "PORT_CHECK_FAILED",
  PROCESS_CLEANUP_UNAVAILABLE: "PROCESS_CLEANUP_UNAVAILABLE",
  ORPHAN_PROCESS: "ORPHAN_PROCESS",
  DRIVER_STARTUP_FAILED: "DRIVER_STARTUP_FAILED",
  MSEDGEDRIVER_STARTUP_FAILED: "MSEDGEDRIVER_STARTUP_FAILED",
  APP_LAUNCH_FAILED: "APP_LAUNCH_FAILED",
  SESSION_CREATE_FAILED: "SESSION_CREATE_FAILED",
  ROOT_MARKER_MISSING: "ROOT_MARKER_MISSING",
  FAKE_RUNTIME_CRASH: "FAKE_RUNTIME_CRASH",
  SQLITE_UNAVAILABLE: "SQLITE_UNAVAILABLE",
  GUI_ASSERTION_FAILED: "GUI_ASSERTION_FAILED",
  SHUTDOWN_TIMEOUT: "SHUTDOWN_TIMEOUT",
  UNSUPPORTED_PLATFORM: "UNSUPPORTED_PLATFORM",
  RUST_NOT_FOUND: "RUST_NOT_FOUND",
  NODE_NOT_FOUND: "NODE_NOT_FOUND",
  NODE_VERSION_INCOMPATIBLE: "NODE_VERSION_INCOMPATIBLE",
  PNPM_NOT_FOUND: "PNPM_NOT_FOUND",
  FAKE_ACP_NOT_FOUND: "FAKE_ACP_NOT_FOUND",
});

/** Doctor component ids (Gate 2.2.2 inventory). */
export const ComponentId = Object.freeze({
  TAURI_DRIVER: "TAURI_DRIVER",
  EDGE_BROWSER: "EDGE_BROWSER",
  WEBVIEW2_RUNTIME: "WEBVIEW2_RUNTIME",
  EDGE_DRIVER: "EDGE_DRIVER",
  APPLICATION_BINARY: "APPLICATION_BINARY",
  FRONTEND_DIST: "FRONTEND_DIST",
  PORT_AVAILABILITY: "PORT_AVAILABILITY",
  PROCESS_CLEANUP_CAPABILITY: "PROCESS_CLEANUP_CAPABILITY",
});

/** Component status values. */
export const ComponentStatus = Object.freeze({
  OK: "OK",
  MISSING: "MISSING",
  MISMATCH: "MISMATCH",
  UNVERIFIED: "UNVERIFIED",
  IN_USE: "IN_USE",
  UNKNOWN: "UNKNOWN",
  NA: "N/A",
});

/** Pipeline stage ids (ordered). */
export const StageId = Object.freeze({
  FRONTEND_BUILD: "frontend_build",
  BACKEND_BUILD: "backend_build",
  PACKAGING: "packaging_test_binary",
  DRIVER_STARTUP: "driver_startup",
  APP_LAUNCH: "app_launch",
  READINESS: "readiness",
  SMOKE: "smoke",
  APP_SHUTDOWN: "app_shutdown",
  DRIVER_SHUTDOWN: "driver_shutdown",
  ORPHAN_VERIFY: "orphan_verification",
});

/** Test levels — do not collapse. */
export const Level = Object.freeze({
  L0_INVOKE_POLICY: "L0",
  L1_BACKEND_BOUNDARY: "L1",
  L2_PACKAGED_SMOKE: "L2",
  L3I_WEBVIEW_INFRA: "L3-I",
  L3J_PRODUCT_JOURNEY: "L3-J",
});

/** CI environment classes. */
export const CiClass = Object.freeze({
  STANDARD_CI: "standard_ci",
  WINDOWS_GUI_RUNNER: "windows_gui_runner",
  PLATFORM_GATED_CI: "platform_gated_ci",
  MANUAL_LOCAL: "manual_local",
  FUTURE_CROSS_PLATFORM: "future_cross_platform",
});

/**
 * Map doctor issues → overall doctor classification (worst wins).
 * @param {{ class: string }[]} issues
 */
export function worstDoctorClass(issues) {
  const rank = {
    [DoctorClass.READY]: 0,
    [DoctorClass.BUILD_REQUIRED]: 1,
    [DoctorClass.DRIVER_UNAVAILABLE]: 2,
    [DoctorClass.MISSING_TOOL]: 3,
    [DoctorClass.INCOMPATIBLE_VERSION]: 4,
    [DoctorClass.WEBVIEW_UNAVAILABLE]: 5,
    [DoctorClass.UNSUPPORTED_PLATFORM]: 6,
  };
  let worst = DoctorClass.READY;
  for (const i of issues) {
    if ((rank[i.class] ?? 0) > (rank[worst] ?? 0)) worst = i.class;
  }
  return worst;
}

/**
 * Map stage failures → suite result.
 * Intentional N/A skips (classification PASS) do not downgrade the suite.
 * @param {{ status: string, classification?: string }[]} stages
 */
export function suiteResultFromStages(stages) {
  const material = stages.filter(
    (s) => !(s.status === "skip" && s.classification === ResultClass.PASS),
  );
  const statuses = material.map((s) => s.status);
  const classes = material.map((s) => s.classification).filter(Boolean);
  if (statuses.some((s) => s === "fail") || classes.includes(ResultClass.FAIL)) {
    return ResultClass.FAIL;
  }
  if (
    statuses.some((s) => s === "blocked_product_gap") ||
    classes.includes(ResultClass.BLOCKED_BY_PRODUCT_GAP)
  ) {
    return ResultClass.BLOCKED_BY_PRODUCT_GAP;
  }
  if (
    statuses.some((s) => s === "blocked_fixture") ||
    classes.includes(ResultClass.BLOCKED_BY_FIXTURE)
  ) {
    return ResultClass.BLOCKED_BY_FIXTURE;
  }
  if (statuses.some((s) => s === "blocked_webview")) {
    return ResultClass.BLOCKED_BY_WEBVIEW;
  }
  if (statuses.some((s) => s === "blocked_tooling")) {
    return ResultClass.BLOCKED_BY_TOOLING;
  }
  if (statuses.some((s) => s === "unsupported")) {
    return ResultClass.UNSUPPORTED_PLATFORM;
  }
  if (statuses.some((s) => s === "skip" || s === "partial")) {
    const anyPass = statuses.some((s) => s === "pass");
    return anyPass ? ResultClass.PARTIAL : ResultClass.BLOCKED_BY_TOOLING;
  }
  return ResultClass.PASS;
}

/**
 * Aggregate L3-J journey classifications into an overall L3-J decision.
 * @param {{ id: string, result: string }[]} journeys
 */
export function suiteResultFromJourneys(journeys) {
  if (!journeys.length) return ResultClass.NOT_STARTED;
  const results = journeys.map((j) => j.result);
  if (results.every((r) => r === ResultClass.PASS)) return ResultClass.PASS;
  if (results.some((r) => r === ResultClass.FAIL)) {
    // Fail only if any hard FAIL and no tooling/product-gap exclusive set
    const nonPass = results.filter((r) => r !== ResultClass.PASS);
    if (nonPass.every((r) => r === ResultClass.FAIL)) return ResultClass.FAIL;
  }
  if (results.some((r) => r === ResultClass.BLOCKED_BY_TOOLING)) {
    if (results.every((r) => r === ResultClass.BLOCKED_BY_TOOLING || r === ResultClass.PASS)) {
      return results.some((r) => r === ResultClass.PASS)
        ? ResultClass.PARTIAL
        : ResultClass.BLOCKED_BY_TOOLING;
    }
  }
  if (results.some((r) => r === ResultClass.BLOCKED_BY_PRODUCT_GAP)) {
    return ResultClass.BLOCKED_BY_PRODUCT_GAP;
  }
  if (results.some((r) => r === ResultClass.BLOCKED_BY_FIXTURE)) {
    return ResultClass.BLOCKED_BY_FIXTURE;
  }
  if (results.some((r) => r === ResultClass.BLOCKED_BY_WEBVIEW)) {
    return ResultClass.BLOCKED_BY_WEBVIEW;
  }
  if (results.some((r) => r === ResultClass.UNSUPPORTED_PLATFORM)) {
    return ResultClass.UNSUPPORTED_PLATFORM;
  }
  if (results.some((r) => r === ResultClass.FAIL)) return ResultClass.FAIL;
  if (results.some((r) => r === ResultClass.PARTIAL || r === ResultClass.PASS)) {
    return ResultClass.PARTIAL;
  }
  return ResultClass.FAIL;
}

export function isSuccessClass(c) {
  return c === ResultClass.PASS || c === ResultClass.PARTIAL;
}