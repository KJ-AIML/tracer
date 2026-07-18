#!/usr/bin/env node
/**
 * W2.3-C consecutive fresh-env full GUI journey runner.
 *
 * Runs N independent L3-J suites (default 5). Each run:
 *  - fresh temp SQLite / workDir / port
 *  - first attempt only (no product retry loops)
 *  - records per-journey first-attempt results
 *  - enforces orphans=0, port collisions handled, temp cleanup
 *
 * Usage:
 *   node tools/tauri-e2e/repeat-gui.mjs
 *   node tools/tauri-e2e/repeat-gui.mjs --runs 5 --skip-build
 *   node tools/tauri-e2e/repeat-gui.mjs --json
 *
 * Env:
 *   TRACER_E2E_REPEAT_RUNS   default 5
 *   TRACER_E2E_KEEP_TEMP     keep workdirs
 */

import { spawn } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { REPO_ROOT } from "./lib/discover.mjs";
import {
  edgeUpdateResilienceProbe,
  countProductAssertionFailures,
  writeReliabilityReport,
  verifyNoOrphans,
  ORPHAN_CHECK_NAMES,
} from "./lib/reliability.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const jsonOut = args.includes("--json");
const skipBuild = args.includes("--skip-build");
const runs = (() => {
  const i = args.indexOf("--runs");
  if (i >= 0 && args[i + 1]) return Math.max(1, Number(args[i + 1]));
  const eq = args.find((a) => a.startsWith("--runs="));
  if (eq) return Math.max(1, Number(eq.split("=")[1]));
  return Math.max(1, Number(process.env.TRACER_E2E_REPEAT_RUNS || 5));
})();

function runOnce(runIndex, { skipBuildOnce }) {
  return new Promise((resolve) => {
    const l3j = path.join(__dirname, "l3j-gui.mjs");
    const childArgs = [l3j, "--json"];
    if (skipBuildOnce) childArgs.push("--skip-build");
    // Force unique preferred port base per run to reduce collision surface
    const portBase = 4444 + runIndex * 10;
    const env = {
      ...process.env,
      TRACER_TAURI_DRIVER_PORT: String(portBase),
      // Explicit: no inject during reliability batch unless operator set it
      TRACER_E2E_INJECT: process.env.TRACER_E2E_INJECT || "none",
    };
    const startedAt = new Date().toISOString();
    const t0 = Date.now();
    const child = spawn(process.execPath, childArgs, {
      cwd: REPO_ROOT,
      env,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (b) => {
      stdout += b.toString("utf8");
      if (!jsonOut) process.stdout.write(b);
    });
    child.stderr.on("data", (b) => {
      stderr += b.toString("utf8");
      if (!jsonOut) process.stderr.write(b);
    });
    child.on("close", (code) => {
      let report = null;
      // Prefer last JSON object in stdout
      try {
        const start = stdout.lastIndexOf("{");
        if (start >= 0) {
          // find matching from last complete parse attempts
          const candidates = [];
          let depth = 0;
          let begin = -1;
          for (let i = 0; i < stdout.length; i++) {
            const ch = stdout[i];
            if (ch === "{") {
              if (depth === 0) begin = i;
              depth++;
            } else if (ch === "}") {
              depth--;
              if (depth === 0 && begin >= 0) {
                candidates.push(stdout.slice(begin, i + 1));
                begin = -1;
              }
            }
          }
          for (let i = candidates.length - 1; i >= 0; i--) {
            try {
              const obj = JSON.parse(candidates[i]);
              if (obj && obj.level === "L3-J" || obj?.module) {
                report = obj;
                break;
              }
            } catch {
              /* continue */
            }
          }
        }
      } catch {
        /* ignore */
      }

      // Also try report file under artifacts if present
      if (!report && report?.meta?.artifactsDir) {
        /* noop */
      }

      const finishedAt = new Date().toISOString();
      const journeys = report?.journeys || [];
      const productFails = countProductAssertionFailures(journeys);
      const orphanVerify = verifyNoOrphans(ORPHAN_CHECK_NAMES, { reap: true });

      resolve({
        runIndex,
        attempt: "first",
        retries: 0,
        startedAt,
        finishedAt,
        durationMs: Date.now() - t0,
        exitCode: code,
        result: report?.result || (code === 0 ? "UNKNOWN" : "FAIL"),
        journeys,
        productAssertionFailures: productFails,
        orphansRemaining: orphanVerify.remaining,
        orphansOk: orphanVerify.ok,
        port: portBase,
        portStrategy: report?.meta?.portStrategy || null,
        portCollisions: report?.meta?.portCollisions ?? 0,
        tempCleanup: report?.meta?.tempCleanup || null,
        runId: report?.meta?.runId || null,
        artifactsDir: report?.meta?.artifactsDir || null,
        workDir: report?.meta?.workDir || null,
        failureCode: report?.failureCode || null,
        timing: report?.meta?.timing || null,
        artifactAudit: report?.meta?.artifactAudit || null,
        retries: 0,
        stderrTail: stderr.slice(-1500),
      });
    });
  });
}

async function main() {
  console.log("=== W2.3-C repeat GUI (fresh-env consecutive) ===");
  console.log(`runs: ${runs}`);
  console.log(`skipBuild after first: ${skipBuild || "first builds, rest skip"}`);

  const edge = edgeUpdateResilienceProbe();
  if (!edge.compatible && edge.applicable) {
    console.warn(`[repeat-gui] Edge driver not compatible: ${edge.message}`);
    console.warn(`[repeat-gui] remediation: ${edge.remediation?.command}`);
  }

  const batchId = `repeat-${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
  const outDir = path.join(REPO_ROOT, "artifacts", "tauri-e2e", batchId);
  mkdirSync(outDir, { recursive: true });

  /** @type {any[]} */
  const runsOut = [];
  for (let i = 1; i <= runs; i++) {
    console.log(`\n========== RUN ${i}/${runs} (first attempt) ==========`);
    const skipBuildOnce = skipBuild || i > 1;
    const r = await runOnce(i, { skipBuildOnce });
    runsOut.push(r);
    console.log(
      `[repeat-gui] run ${i}: result=${r.result} productFails=${r.productAssertionFailures} orphansOk=${r.orphansOk} exit=${r.exitCode}`,
    );
    // Stop early on hard product fail? Continue for full matrix evidence.
  }

  const consecutivePass = (() => {
    let n = 0;
    for (const r of runsOut) {
      if (
        r.result === "PASS" &&
        r.productAssertionFailures === 0 &&
        r.orphansOk &&
        (r.portCollisions || 0) === 0 &&
        (r.tempCleanup == null || r.tempCleanup.cleaned !== false)
      ) {
        n += 1;
      } else break;
    }
    return n;
  })();

  const allPass = runsOut.every(
    (r) =>
      r.result === "PASS" &&
      r.productAssertionFailures === 0 &&
      r.orphansOk,
  );

  const summary = {
    schemaVersion: 1,
    module: "W2.3-C",
    suite: "repeat-gui",
    batchId,
    runsRequested: runs,
    runsCompleted: runsOut.length,
    consecutiveFirstAttemptPass: consecutivePass,
    allPass,
    objective: {
      minConsecutive: 5,
      met: consecutivePass >= 5 || (runs >= 5 && allPass),
      productAssertionFailures: runsOut.reduce(
        (a, r) => a + (r.productAssertionFailures || 0),
        0,
      ),
      orphans: runsOut.reduce(
        (a, r) => a + (r.orphansRemaining?.length || 0),
        0,
      ),
      portCollisions: runsOut.reduce((a, r) => a + (r.portCollisions || 0), 0),
      tempCleanupFailures: runsOut.filter(
        (r) => r.tempCleanup && r.tempCleanup.cleaned === false,
      ).length,
    },
    edge,
    runs: runsOut,
    policy: {
      firstAttemptOnly: true,
      unlimitedRetries: false,
      liveProvider: false,
      network: false,
      credentials: false,
    },
  };

  const reportPath = writeReliabilityReport(
    path.join(outDir, "repeat-gui-report.json"),
    summary,
  );

  if (jsonOut) console.log(JSON.stringify(summary, null, 2));
  else {
    console.log("");
    console.log(`repeat-gui consecutive first-attempt PASS: ${consecutivePass}/${runs}`);
    console.log(`objective met: ${summary.objective.met}`);
    console.log(`product assertion failures: ${summary.objective.productAssertionFailures}`);
    console.log(`orphans: ${summary.objective.orphans}`);
    console.log(`port collisions: ${summary.objective.portCollisions}`);
    console.log(`temp cleanup failures: ${summary.objective.tempCleanupFailures}`);
    console.log(`report: ${reportPath}`);
  }

  process.exitCode = summary.objective.met ? 0 : 1;
}

main().catch((e) => {
  console.error("[repeat-gui] FAILED:", e);
  process.exitCode = 1;
});
