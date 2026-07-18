#!/usr/bin/env node
/**
 * Tauri E2E doctor — environment discovery + readiness classification.
 *
 * Exit codes:
 *   0  READY (or only BUILD_REQUIRED / DRIVER_UNAVAILABLE advisory for L3-I)
 *   2  blocked (MISSING_TOOL / WEBVIEW_UNAVAILABLE / UNSUPPORTED / INCOMPATIBLE)
 *   1  unexpected error
 *
 * Usage:
 *   node tools/tauri-e2e/doctor.mjs
 *   node tools/tauri-e2e/doctor.mjs --json
 *   pnpm --filter @tracer/tauri-e2e doctor
 */

import { writeFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import {
  DoctorClass,
  CiClass,
  Level,
  worstDoctorClass,
} from "./lib/classify.mjs";
import { runDiscovery, REPO_ROOT } from "./lib/discover.mjs";

const args = new Set(process.argv.slice(2));
const asJson = args.has("--json");
const writeReport = args.has("--write-report");

function printHuman(report) {
  const { env, issues, capabilities, classification, ci } = report;
  console.log("=== Tauri E2E Doctor (W2.2-A) ===");
  console.log(`repo: ${env.paths.repoRoot}`);
  console.log(`os: ${env.os.platform}/${env.os.arch} (${env.os.release})`);
  console.log(`rustc: ${env.rust.rustc || "MISSING"}`);
  console.log(`cargo: ${env.rust.cargo || "MISSING"}`);
  console.log(`node: ${env.node.version || "MISSING"}`);
  console.log(`pnpm: ${env.pnpm.version || "MISSING"}`);
  console.log(
    `tauri-cli: ${env.tauriCli.available ? env.tauriCli.version || "present" : "MISSING (optional for cargo-only)"}`,
  );
  console.log(
    `webview: ${env.webview.available ? env.webview.version || "present" : "UNAVAILABLE"}`,
  );
  console.log(
    `tauri-driver: ${env.drivers.tauriDriver.available ? env.drivers.tauriDriver.path : "MISSING"}`,
  );
  if (env.os.platform === "win32") {
    console.log(
      `msedgedriver: ${env.drivers.nativeDriver.msedgedriver.available ? env.drivers.nativeDriver.msedgedriver.path : "MISSING"}`,
    );
  }
  console.log(
    `frontendDist: ${env.build.frontendDistPresent ? env.paths.frontendDist : "MISSING"}`,
  );
  console.log(
    `appBinary: ${env.paths.appBinary || "MISSING (build required)"}`,
  );
  console.log(
    `fakeAcp: ${env.paths.fakeAcpPresent ? env.paths.fakeAcpJs : "MISSING"}`,
  );
  console.log(`ports: viteDev=${env.ports.viteDev} tauriDriver=${env.ports.tauriDriverDefault}`);
  console.log("");
  console.log("Capabilities:");
  for (const [level, info] of Object.entries(capabilities)) {
    if (level === "blockers" || !info || typeof info !== "object" || !("attemptable" in info)) {
      continue;
    }
    console.log(
      `  ${level}: attemptable=${info.attemptable} — ${info.claim}${info.needsBuild ? " [BUILD_REQUIRED]" : ""}`,
    );
  }
  if (Array.isArray(capabilities.blockers) && capabilities.blockers.length) {
    console.log(`  blockers: ${capabilities.blockers.join(", ")}`);
  }
  console.log("");
  console.log(`Doctor classification: ${classification}`);
  console.log(`CI class guidance: ${ci.join(", ")}`);
  if (issues.length) {
    console.log("");
    console.log("Issues:");
    for (const i of issues) {
      console.log(`  [${i.class}] ${i.code}: ${i.message}`);
      if (i.setup) console.log(`    setup: ${i.setup}`);
      if (i.fallback) console.log(`    fallback: ${i.fallback}`);
    }
  } else {
    console.log("Issues: none");
  }
  console.log("");
  console.log("L3-J full GUI product journey: DEFERRED (not claimed by W2.2-A)");
}

function ciGuidance(env, capabilities, classification) {
  const list = [CiClass.MANUAL_LOCAL];
  // L0+L1 always standard CI when tools present
  if (capabilities.L0.attemptable && capabilities.L1.attemptable) {
    list.unshift(CiClass.STANDARD_CI);
  }
  if (capabilities.L2.attemptable) {
    list.push(CiClass.WINDOWS_GUI_RUNNER);
    list.push(CiClass.PLATFORM_GATED_CI);
  }
  if (capabilities["L3-I"].attemptable) {
    list.push(CiClass.WINDOWS_GUI_RUNNER);
  }
  list.push(CiClass.FUTURE_CROSS_PLATFORM);
  if (classification === DoctorClass.UNSUPPORTED_PLATFORM) {
    return [CiClass.MANUAL_LOCAL, CiClass.FUTURE_CROSS_PLATFORM];
  }
  return [...new Set(list)];
}

/**
 * Exit policy:
 * - READY → 0
 * - BUILD_REQUIRED / DRIVER_UNAVAILABLE alone (still can run L0/L1) → 0 with advisory
 * - MISSING critical tools, WEBVIEW_UNAVAILABLE, INCOMPATIBLE, UNSUPPORTED → 2
 */
function exitCode(classification, capabilities) {
  if (classification === DoctorClass.READY) return 0;
  if (
    classification === DoctorClass.BUILD_REQUIRED ||
    classification === DoctorClass.DRIVER_UNAVAILABLE
  ) {
    // Advisory: L0/L1 may still run
    return capabilities.L0.attemptable || capabilities.L1.attemptable ? 0 : 2;
  }
  if (
    classification === DoctorClass.MISSING_TOOL ||
    classification === DoctorClass.INCOMPATIBLE_VERSION ||
    classification === DoctorClass.WEBVIEW_UNAVAILABLE ||
    classification === DoctorClass.UNSUPPORTED_PLATFORM
  ) {
    // If L0 still works, soft-fail with 0 when only optional tools missing?
    // Hard tools (node/rust) that break L0/L1 → 2
    if (!capabilities.L0.attemptable && !capabilities.L1.attemptable) return 2;
    // optional tool missing but L0/L1 ok → 0 advisory
    if (
      classification === DoctorClass.MISSING_TOOL &&
      (capabilities.L0.attemptable || capabilities.L1.attemptable)
    ) {
      return 0;
    }
    return 2;
  }
  return 0;
}

function main() {
  try {
    const { env, issues, capabilities } = runDiscovery();
    // Filter: tauri_cli missing is advisory only (not hard blocker for READY-ish)
    const hardIssues = issues.filter((i) => i.code !== "tauri_cli");
    const classification =
      hardIssues.length === 0
        ? DoctorClass.READY
        : worstDoctorClass(hardIssues);

    // READY means L2 process path can be attempted after optional build.
    // If only BUILD_REQUIRED + DRIVER_UNAVAILABLE, classification stays those.
    const ci = ciGuidance(env, capabilities, classification);

    const report = {
      schemaVersion: 1,
      module: "W2.2-A",
      task: "tracer-w2-tauri-e2e-infrastructure",
      classification,
      doctorClasses: DoctorClass,
      levels: Level,
      ci,
      capabilities,
      issues,
      env,
      notes: {
        l3j: "DEFERRED — full GUI product journey is future W2.2-B; not claimed",
        network: "no",
        credentials: "no",
        liveGrok: "no",
        fakeAcp: "optional for pure process smoke; yes for boundary L1",
        tempSqlite: "yes if needed",
      },
    };

    if (asJson) {
      console.log(JSON.stringify(report, null, 2));
    } else {
      printHuman(report);
    }

    if (writeReport) {
      const outDir = path.join(REPO_ROOT, "docs/validation/tauri");
      mkdirSync(outDir, { recursive: true });
      const out = path.join(outDir, "TAURI_E2E_DOCTOR_LAST.json");
      writeFileSync(out, JSON.stringify(report, null, 2), "utf8");
      if (!asJson) console.log(`wrote ${out}`);
    }

    process.exitCode = exitCode(classification, capabilities);
  } catch (e) {
    console.error("[doctor] FAILED:", e instanceof Error ? e.message : e);
    process.exitCode = 1;
  }
}

main();
