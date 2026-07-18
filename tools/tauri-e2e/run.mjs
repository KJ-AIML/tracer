#!/usr/bin/env node
/**
 * W2-B Tauri E2E harness orchestrator.
 *
 * Standard automated class:
 *   network: no, credentials: no, live Grok: no, provider: no
 *   fake ACP: yes, temp file SQLite: yes
 *
 * Layers:
 *   1) Frontend invoke policy (vitest) — Tauri detection / no silent mock downgrade
 *   2) Desktop boundary journey (cargo test -p tracer-desktop) — real command glue + CP
 *   3) Optional GUI probe — documents blocker when WebView drive unavailable
 *
 * Classification: desktop-boundary E2E (not full WebView GUI E2E).
 */

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

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
const skipPolicy = boundaryOnly;
const skipBoundary = policyOnly;

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
 * (dist is gitignored; real product builds still run `pnpm build`.)
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
  log("policy", "frontend invoke policy (vitest)");
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
  log("boundary", "desktop boundary journey (cargo test -p tracer-desktop)");
  ensureFrontendDistStub();
  const env = {
    TRACER_FAKE_ACP_JS: FAKE_JS,
    // Force serial-ish behavior under Windows via test lock; keep node path.
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
 * GUI probe: strongest attempt without claiming full GUI E2E.
 * Does not launch interactive WebView drive (no tauri-driver / WebDriver).
 */
function runGuiProbe() {
  log("gui-probe", "checking Tauri tooling presence (no full WebView drive)");
  const blockers = [];

  const desktopPkg = path.join(REPO_ROOT, "apps/desktop/package.json");
  if (!existsSync(desktopPkg)) {
    blockers.push("apps/desktop/package.json missing");
  }

  // Do not run full `tauri build` in standard CI — long, platform SDK heavy.
  const report = {
    classification: "desktop-boundary-e2e",
    fullGuiE2e: false,
    reason:
      "Full browser-driving (Playwright/WebDriver via tauri-driver) is not wired in standard CI. " +
      "W2-B delivers executable desktop-boundary journey through the same plane_* handlers the Tauri app registers, " +
      "plus frontend invoke policy tests. Follow-up: install tauri-driver + WebView2 and drive preferred GUI path.",
    preferredPathDocumented: true,
    blockers,
    envHooks: [
      "TRACER_DATABASE_PATH",
      "TRACER_FAKE_ACP_JS",
      "TRACER_HELI_PROBE_PATH",
      "TRACER_NODE_BIN",
    ],
  };
  console.log(JSON.stringify(report, null, 2));
  if (blockers.length) {
    process.exitCode = 1;
  }
}

function main() {
  console.log("=== W2-B Tauri E2E harness ===");
  console.log(`repo: ${REPO_ROOT}`);
  assertFakeRuntime();

  try {
    if (!skipPolicy) {
      runPolicy();
      log("policy", "PASS");
    }
    if (!skipBoundary) {
      runBoundary();
      log("boundary", "PASS");
    }
    if (guiProbe || (!policyOnly && !boundaryOnly)) {
      // Always emit classification when running full suite or explicit probe.
      runGuiProbe();
      log("gui-probe", "documented (not false full-GUI claim)");
    }
  } catch (e) {
    console.error("[tauri-e2e] FAILED:", e instanceof Error ? e.message : e);
    process.exit(process.exitCode || 1);
  }

  console.log(
    "=== W2-B Tauri E2E harness: PASS (desktop-boundary classification) ===",
  );
}

main();
