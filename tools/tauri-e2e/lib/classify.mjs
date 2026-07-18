/**
 * W2.2-A classification vocabularies.
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
  UNSUPPORTED_PLATFORM: "UNSUPPORTED_PLATFORM",
  FAIL: "FAIL",
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
  if (statuses.some((s) => s === "fail")) return ResultClass.FAIL;
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

export function isSuccessClass(c) {
  return c === ResultClass.PASS || c === ResultClass.PARTIAL;
}
