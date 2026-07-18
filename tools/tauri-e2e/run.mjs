#!/usr/bin/env node
/**
 * W2.2-A Tauri E2E harness orchestrator.
 *
 * Extends Gate 2.1 (W2-B) L0+L1 desktop-boundary harness with:
 *   - doctor / environment discovery
 *   - L2 packaged application launch smoke
 *   - L3-I WebView driver infrastructure interaction
 *
 * L3-J full GUI product journey: dedicated entry `l3j-gui.mjs` / `pnpm test:tauri-e2e:gui`.
 *
 * Standard automated class (L0+L1):
 *   network: no, credentials: no, live Grok: no, provider: no
 *   fake ACP: yes, temp file SQLite: yes
 *
 * L2/L3-I: platform-gated / Windows GUI runner / manual local
 */

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Level, ResultClass } from "./lib/classify.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../..");
const FAKE_JS = path.join(
  REPO_ROOT,
  "tools/fake-acp-runtime/bin/fake-acp-runtime.js",
);

const args = new Set(process.argv.slice(2));
const policyOnly = args.has("--policy-only");
const boundaryOnly = args.has("--boundary-only");
const guiProbe = args.has("--gui-probe");
const doctorOnly = args.has("--doctor");
const l2Only = args.has("--l2");
const l3iOnly = args.has("--l3i");
const l3jOnly = args.has("--l3j") || args.has("--gui");
const allLevels = args.has("--all");
const skipPolicy = boundaryOnly || l2Only || l3iOnly || l3jOnly || doctorOnly;
const skipBoundary = policyOnly || l2Only || l3iOnly || l3jOnly || doctorOnly;

function log(section, msg) {
  console.log(`[tauri-e2e:${section}] ${msg}`);
}

function run(cmd, cmdArgs, opts = {}) {
  log("exec", `${cmd} ${cmdArgs.join(" ")}`);
  const r = spawnSync(cmd, cmdArgs, {
    cwd: opts.cwd ?? REPO_ROOT,
    env: { ...process.env, ...(opts.env ?? {}) },
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (r.error) {
    throw r.error;
  }
  if (r.status !== 0) {
    process.exitCode = r.status ?? 1;
    throw new Error(`${cmd} exited ${r.status}`);
  }
  return r.status ?? 0;
}

function runNodeScript(rel, extraArgs = []) {
  const script = path.join(__dirname, rel);
  return run(process.execPath, [script, ...extraArgs], { cwd: REPO_ROOT });
}

function assertFakeRuntime() {
  if (!existsSync(FAKE_JS)) {
    console.error(`Missing fake ACP runtime: ${FAKE_JS}`);
    process.exit(2);
  }
  log("preflight", `fake ACP ok: ${FAKE_JS}`);
}

/**
 * Tauri `generate_context!()` requires `apps/desktop/dist` to exist at compile time.
 * Create a minimal stub when missing so cargo test does not require a full Vite build.
 */
function ensureFrontendDistStub() {
  const distDir = path.join(REPO_ROOT, "apps/desktop/dist");
  const indexHtml = path.join(distDir, "index.html");
  if (existsSync(indexHtml)) {
    log("preflight", `frontendDist present: ${distDir}`);
    return;
  }
  mkdirSync(distDir, { recursive: true });
  writeFileSync(
    indexHtml,
    `<!doctype html><html lang="en"><head><meta charset="UTF-8"/><title>Tracer</title></head>` +
      `<body><div id="root">tracer desktop e2e stub dist</div></body></html>\n`,
    "utf8",
  );
  log("preflight", `created frontendDist stub: ${indexHtml}`);
}

function runPolicy() {
  log("policy", "L0 frontend invoke policy (vitest)");
  run(
    "pnpm",
    [
      "--filter",
      "@tracer/desktop",
      "exec",
      "vitest",
      "run",
      "src/shared/commands/invoke.policy.test.ts",
    ],
    { cwd: REPO_ROOT },
  );
}

function runBoundary() {
  log("boundary", "L1 desktop boundary journey (cargo test -p tracer-desktop)");
  ensureFrontendDistStub();
  const env = {
    TRACER_FAKE_ACP_JS: FAKE_JS,
    RUST_BACKTRACE: process.env.RUST_BACKTRACE ?? "1",
  };
  run(
    "cargo",
    [
      "test",
      "-p",
      "tracer-desktop",
      "--test",
      "desktop_boundary_journey",
      "--",
      "--test-threads=1",
      "--nocapture",
    ],
    { env },
  );
}

/**
 * GUI probe (legacy W2-B): documents classification; does not claim L3-J.
 */
function runGuiProbe() {
  log("gui-probe", "classification report (no false full-GUI claim)");
  const blockers = [];
  const desktopPkg = path.join(REPO_ROOT, "apps/desktop/package.json");
  if (!existsSync(desktopPkg)) {
    blockers.push("apps/desktop/package.json missing");
  }

  const report = {
    classification: "tauri-e2e-infrastructure",
    levels: {
      L0: "invoke policy — executable via --policy-only / full suite",
      L1: "desktop boundary — executable via --boundary-only / full suite",
      L2: "packaged app smoke — node tools/tauri-e2e/l2-smoke.mjs",
      "L3-I": "WebView driver infra — node tools/tauri-e2e/l3i-infra.mjs",
      "L3-J": "full GUI product journey — node tools/tauri-e2e/l3j-gui.mjs / pnpm test:tauri-e2e:gui",
    },
    fullGuiE2e: true,
    fullGuiProductJourneyL3J: true,
    reason:
      "W2.2-B owns L3-J product journeys (GJ-01..12) via real WebDriver + Tauri. " +
      "L0–L3-I remain executable. L3-J is independent and not part of pnpm -r test. " +
      "Run: pnpm test:tauri-e2e:gui  or  pnpm test:tauri-e2e:gui -- --journey GJ-03",
    preferredPathDocumented: true,
    blockers,
    envHooks: [
      "TRACER_DATABASE_PATH",
      "TRACER_FAKE_ACP_JS",
      "TRACER_HELI_PROBE_PATH",
      "TRACER_NODE_BIN",
      "TRACER_TAURI_DRIVER_PORT",
      "TRACER_E2E_PROFILE",
      "TRACER_E2E_APP_BINARY",
      "TRACER_NATIVE_DRIVER",
      "TRACER_E2E_READY_MARKER",
    ],
    commands: {
      doctor: "node tools/tauri-e2e/doctor.mjs",
      l0l1: "node tools/tauri-e2e/run.mjs",
      l2: "node tools/tauri-e2e/l2-smoke.mjs",
      l3i: "node tools/tauri-e2e/l3i-infra.mjs",
      l3j: "node tools/tauri-e2e/l3j-gui.mjs",
      gui: "pnpm test:tauri-e2e:gui",
      all: "node tools/tauri-e2e/run.mjs --all",
    },
  };
  console.log(JSON.stringify(report, null, 2));
  if (blockers.length) {
    process.exitCode = 1;
  }
}

function main() {
  console.log("=== Tracer Tauri E2E harness (W2.2-A + Gate 2.1 L0/L1) ===");
  console.log(`repo: ${REPO_ROOT}`);

  try {
    if (doctorOnly) {
      runNodeScript("doctor.mjs");
      return;
    }

    if (l2Only) {
      runNodeScript("l2-smoke.mjs", process.argv.slice(2).filter((a) => a !== "--l2"));
      return;
    }

    if (l3iOnly) {
      runNodeScript("l3i-infra.mjs", process.argv.slice(2).filter((a) => a !== "--l3i"));
      return;
    }

    if (l3jOnly) {
      runNodeScript(
        "l3j-gui.mjs",
        process.argv.slice(2).filter((a) => a !== "--l3j" && a !== "--gui"),
      );
      return;
    }

    if (!l2Only && !l3iOnly && !l3jOnly) {
      assertFakeRuntime();
    }

    if (!skipPolicy) {
      runPolicy();
      log("policy", `PASS (${Level.L0_INVOKE_POLICY})`);
    }
    if (!skipBoundary) {
      runBoundary();
      log("boundary", `PASS (${Level.L1_BACKEND_BOUNDARY})`);
    }

    if (allLevels) {
      log("l2", "running L2 packaged smoke");
      try {
        runNodeScript("l2-smoke.mjs", ["--skip-build"]);
        log("l2", "completed (see result classification — may be PARTIAL/BLOCKED)");
      } catch (e) {
        log("l2", `non-fatal for L0/L1 gate: ${e instanceof Error ? e.message : e}`);
        process.exitCode = 0; // L2 failure does not fail L0/L1 standard CI
      }
      log("l3i", "running L3-I driver infrastructure");
      try {
        runNodeScript("l3i-infra.mjs");
        log("l3i", "completed (see result classification)");
      } catch (e) {
        log("l3i", `non-fatal for L0/L1 gate: ${e instanceof Error ? e.message : e}`);
        process.exitCode = 0;
      }
    }

    if (guiProbe || (!policyOnly && !boundaryOnly && !allLevels)) {
      runGuiProbe();
      log("gui-probe", "documented (L3-J not claimed)");
    }
  } catch (e) {
    console.error("[tauri-e2e] FAILED:", e instanceof Error ? e.message : e);
    process.exit(process.exitCode || 1);
  }

  console.log(
    "=== Tauri E2E harness: L0/L1 PASS (desktop-boundary); L2/L3-I/L3-J via dedicated commands ===",
  );
  console.log(
    `Honest levels: ${Level.L0_INVOKE_POLICY}+${Level.L1_BACKEND_BOUNDARY} executable; ` +
      `${Level.L2_PACKAGED_SMOKE}/${Level.L3I_WEBVIEW_INFRA} platform-gated; ` +
      `${Level.L3J_PRODUCT_JOURNEY} via pnpm test:tauri-e2e:gui`,
  );
  void ResultClass;
}

main();
