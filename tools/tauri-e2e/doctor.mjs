#!/usr/bin/env node
/**
 * Tauri E2E doctor — environment discovery + readiness classification
 * (W2.2-T base + W2.3-C Edge-update resilience).
 *
 * Modes:
 *   plan (default)  — inventory only, no install
 *   apply           — opt-in provisioning via tools/tauri-driver/setup.mjs
 *
 * Apply authorization:
 *   --apply  OR  TRACER_TAURI_E2E_SETUP=1
 *
 * Exit codes:
 *   0  READY (or advisory BUILD_REQUIRED / DRIVER_UNAVAILABLE when L0/L1 ok)
 *   2  blocked hard (MISSING critical / WEBVIEW / UNSUPPORTED / INCOMPATIBLE)
 *   1  unexpected error
 *
 * Usage:
 *   node tools/tauri-e2e/doctor.mjs
 *   node tools/tauri-e2e/doctor.mjs --json
 *   node tools/tauri-e2e/doctor.mjs --apply
 *   pnpm test:tauri-e2e:doctor
 */

import { writeFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  DoctorClass,
  CiClass,
  Level,
  worstDoctorClass,
  ComponentStatus,
} from "./lib/classify.mjs";
import { runDiscovery, REPO_ROOT } from "./lib/discover.mjs";
import { edgeUpdateResilienceProbe } from "./lib/reliability.mjs";

const args = new Set(process.argv.slice(2));
const asJson = args.has("--json");
const writeReport = args.has("--write-report");
const wantApply =
  args.has("--apply") ||
  process.env.TRACER_TAURI_E2E_SETUP === "1" ||
  process.env.TRACER_TAURI_E2E_SETUP === "true";
const planOnly = args.has("--plan") || !wantApply;

function printHuman(report) {
  const { env, issues, capabilities, classification, ci, components, mode } =
    report;
  console.log("=== Tauri E2E Doctor (W2.3-C reliability) ===");
  console.log(`mode: ${mode}`);
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
    `edge: ${
      env.drivers.edgeBrowser?.available
        ? env.drivers.edgeBrowser.version || "present"
        : env.os.platform === "win32"
          ? "MISSING"
          : "N/A"
    }`,
  );
  console.log(
    `tauri-driver: ${
      env.drivers.tauriDriver.available
        ? `${env.drivers.tauriDriver.version || "present"} @ ${env.drivers.tauriDriver.pathRedacted}`
        : "MISSING"
    }`,
  );
  if (env.os.platform === "win32") {
    const md = env.drivers.nativeDriver.msedgedriver;
    console.log(
      `msedgedriver: ${
        md.available
          ? `${md.version || "UNVERIFIED"} @ ${md.pathRedacted}`
          : "MISSING"
      }`,
    );
    if (md.compatibility) {
      console.log(
        `  compatibility: ${md.compatibility.code} — ${md.compatibility.message}`,
      );
      console.log(`  rule: ${md.compatibility.rule || "major(msedgedriver)==major(Edge)"}`);
    }
  }
  console.log(
    `frontendDist: ${env.build.frontendDistPresent ? env.paths.frontendDist : "MISSING"}`,
  );
  console.log(
    `appBinary: ${env.paths.appBinaryRedacted || "MISSING (build required)"}`,
  );
  console.log(
    `fakeAcp: ${env.paths.fakeAcpPresent ? env.paths.fakeAcpJs : "MISSING"}`,
  );
  console.log(
    `ports: viteDev=${env.ports.viteDev} tauriDriver=${env.ports.tauriDriverDefault} available=${env.ports.tauriDriver.available}`,
  );
  console.log(
    `processCleanup: ${env.processCleanup.available ? env.processCleanup.method : "UNAVAILABLE"}`,
  );
  if (report.edgeResilience) {
    const er = report.edgeResilience;
    console.log(
      `edgeResilience: applicable=${er.applicable} compatible=${er.compatible} code=${er.code}`,
    );
    if (er.remediation) {
      console.log(`  remediation: ${er.remediation.command}`);
    }
  }
  console.log("");
  console.log("Components:");
  for (const c of components || []) {
    const ver = c.version ? ` version=${c.version}` : "";
    const p = c.path ? ` path=${c.path}` : "";
    const code = c.code ? ` code=${c.code}` : "";
    console.log(`  ${c.id}: ${c.status}${ver}${p}${code}`);
  }
  console.log("");
  console.log("Capabilities:");
  for (const [level, info] of Object.entries(capabilities)) {
    if (
      level === "blockers" ||
      !info ||
      typeof info !== "object" ||
      !("attemptable" in info)
    ) {
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
      if (i.rule) console.log(`    rule: ${i.rule}`);
    }
  } else {
    console.log("Issues: none");
  }
  if (planOnly && classification !== DoctorClass.READY) {
    console.log("");
    console.log(
      "Plan mode: no installs performed. Authorize apply with --apply or TRACER_TAURI_E2E_SETUP=1",
    );
  }
  console.log("");
  const l3j = capabilities["L3-J"];
  console.log(
    l3j?.attemptable
      ? "L3-J full GUI product journey: attemptable via pnpm test:tauri-e2e:gui (W2.2-B)"
      : "L3-J full GUI product journey: blocked until drivers+build ready (then pnpm test:tauri-e2e:gui)",
  );
}

function ciGuidance(env, capabilities, classification) {
  const list = [CiClass.MANUAL_LOCAL];
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
 * - BUILD_REQUIRED / DRIVER_UNAVAILABLE alone (still can run L0/L1) → 0 advisory
 * - MISSING critical tools, WEBVIEW_UNAVAILABLE, INCOMPATIBLE, UNSUPPORTED → 2 when L0/L1 blocked
 */
function exitCode(classification, capabilities) {
  if (classification === DoctorClass.READY) return 0;
  if (
    classification === DoctorClass.BUILD_REQUIRED ||
    classification === DoctorClass.DRIVER_UNAVAILABLE
  ) {
    return capabilities.L0.attemptable || capabilities.L1.attemptable ? 0 : 2;
  }
  if (
    classification === DoctorClass.MISSING_TOOL ||
    classification === DoctorClass.INCOMPATIBLE_VERSION ||
    classification === DoctorClass.WEBVIEW_UNAVAILABLE ||
    classification === DoctorClass.UNSUPPORTED_PLATFORM
  ) {
    if (!capabilities.L0.attemptable && !capabilities.L1.attemptable) return 2;
    if (
      classification === DoctorClass.MISSING_TOOL &&
      (capabilities.L0.attemptable || capabilities.L1.attemptable)
    ) {
      return 0;
    }
    // version mismatch of edge driver: advisory 0 if L0/L1 ok
    if (
      classification === DoctorClass.INCOMPATIBLE_VERSION &&
      (capabilities.L0.attemptable || capabilities.L1.attemptable)
    ) {
      return 0;
    }
    return 2;
  }
  return 0;
}

function runApply() {
  const setup = path.join(REPO_ROOT, "tools/tauri-driver/setup.mjs");
  console.log(`[doctor] apply → node ${setup} --apply`);
  const r = spawnSync(process.execPath, [setup, "--apply"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    windowsHide: true,
    timeout: 900_000,
    env: { ...process.env, TRACER_TAURI_E2E_SETUP: "1" },
  });
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  return {
    ok: r.status === 0,
    status: r.status,
    error: r.error ? r.error.message : null,
  };
}

function main() {
  try {
    let applyResult = null;
    if (wantApply) {
      applyResult = runApply();
    }

    const { env, issues, capabilities, components } = runDiscovery();
    const edgeResilience = edgeUpdateResilienceProbe();

    // Edge auto-update resilience: attach remediation when incompatible (W2.3-C).
    // Avoid duplicate issue codes if discovery already reported mismatch.
    if (edgeResilience.applicable && !edgeResilience.compatible) {
      const code = edgeResilience.code || "EDGE_DRIVER_VERSION_MISMATCH";
      const existing = issues.find((i) => i.code === code || String(i.code || "").includes("EDGE_DRIVER"));
      if (existing) {
        existing.setup = existing.setup || edgeResilience.remediation?.command;
        existing.fallback = existing.fallback || edgeResilience.remediation?.alt;
        existing.rule = existing.rule || edgeResilience.rule;
      } else {
        issues.push({
          class: DoctorClass.INCOMPATIBLE_VERSION,
          code,
          message: edgeResilience.message,
          setup: edgeResilience.remediation?.command,
          fallback: edgeResilience.remediation?.alt,
          rule: edgeResilience.rule,
        });
      }
    }

    const hardIssues = issues.filter((i) => i.code !== "tauri_cli");
    const classification =
      hardIssues.length === 0
        ? DoctorClass.READY
        : worstDoctorClass(hardIssues);

    const ci = ciGuidance(env, capabilities, classification);

    const report = {
      schemaVersion: 1,
      module: "W2.3-C",
      task: "tracer-w2-gui-reliability",
      mode: wantApply ? "apply" : "plan",
      classification,
      doctorClasses: DoctorClass,
      levels: Level,
      ci,
      components,
      capabilities,
      issues,
      env,
      apply: applyResult,
      edgeResilience,
      notes: {
        l3j: "L3-J via pnpm test:tauri-e2e:gui; reliability batch via pnpm test:tauri-e2e:repeat-gui",
        network: wantApply
          ? "one-time driver download may use network during apply"
          : "no",
        credentials: "no",
        liveGrok: "no",
        fakeAcp: "optional for pure process smoke; yes for boundary L1",
        tempSqlite: "yes if needed",
        applyAuth: "TRACER_TAURI_E2E_SETUP=1 or --apply",
        compatibilityRule: "major(msedgedriver) == major(Edge)",
        edgeUpdateResilience:
          "On Edge major auto-update, doctor reports INCOMPATIBLE_VERSION + remediation; --apply re-downloads matching msedgedriver (no silent PASS)",
        ciIsolation:
          "L3-I not in pnpm -r test / cargo workspace; use pnpm test:tauri-e2e:l3i",
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
      const out = path.join(outDir, "WEBVIEW_DRIVER_READINESS_LAST.json");
      writeFileSync(out, JSON.stringify(report, null, 2), "utf8");
      if (!asJson) console.log(`wrote ${out}`);
    }

    if (wantApply && applyResult && !applyResult.ok) {
      process.exitCode = 2;
    } else {
      process.exitCode = exitCode(classification, capabilities);
    }
  } catch (e) {
    console.error("[doctor] FAILED:", e instanceof Error ? e.message : e);
    process.exitCode = 1;
  }
}

main();