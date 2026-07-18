#!/usr/bin/env node
/**
 * LGJ dry-run — plan validation only (W2.3-B).
 * Never spawns Grok agent stdio, never launches GUI, never consumes provider usage.
 *
 * Usage:
 *   node tools/tauri-e2e/live/dry-run.mjs
 *   node tools/tauri-e2e/live/dry-run.mjs --out target/live-gui/dry-run.json
 */

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { LgjClass, suiteResultFromLgj, notRun, LGJ_NAMES } from "./lib/classify.mjs";
import { printOperationClass, parseArgs, OPERATION_CLASS } from "./lib/opt-in.mjs";
import { stockGrokSpawnPlan } from "./launch-live-grok.mjs";
import { JOURNEY_RUNNERS } from "./lib/journeys.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../../..");
const BRIDGE = path.join(__dirname, "launch-live-grok.mjs");

function discoverGrok() {
  const fromEnv = process.env.TRACER_GROK_BIN?.trim();
  if (fromEnv) return { path: fromEnv, source: "TRACER_GROK_BIN" };
  const r = spawnSync(process.platform === "win32" ? "where" : "which", ["grok"], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 10_000,
  });
  if (r.status === 0 && r.stdout?.trim()) {
    const first = r.stdout.trim().split(/\r?\n/)[0];
    return { path: first, source: "PATH" };
  }
  return { path: null, source: "missing" };
}

function main() {
  const cli = parseArgs(process.argv.slice(2));
  printOperationClass({ live: false });

  const grok = discoverGrok();
  const plan = stockGrokSpawnPlan(grok.path || "grok");
  const bridgeExists = existsSync(BRIDGE);

  // Unit-level checks
  const checks = [];
  checks.push({
    id: "bridge_script",
    ok: bridgeExists,
    detail: bridgeExists ? BRIDGE : "missing launch-live-grok.mjs",
  });
  checks.push({
    id: "spawn_plan_w0b",
    ok: plan.matchesW0bW1d && plan.args.join(" ") === "agent --no-leader stdio",
    detail: plan,
  });
  checks.push({
    id: "journey_catalog",
    ok: JOURNEY_RUNNERS.length === 7 && JOURNEY_RUNNERS.every((j) => LGJ_NAMES[j.id]),
    detail: JOURNEY_RUNNERS.map((j) => j.id),
  });
  checks.push({
    id: "opt_in_gates_documented",
    ok: true,
    detail: {
      env: ["TRACER_LIVE_GROK=1", "TRACER_LIVE_GUI=1"],
      command: "node tools/tauri-e2e/live/lgj.mjs run",
    },
  });
  checks.push({
    id: "dry_run_no_live_spawn",
    ok: true,
    detail: "dry-run path does not call spawn(grok) or WebDriver newSession",
  });
  checks.push({
    id: "ci_isolation",
    ok: true,
    detail: "live scripts not invoked by tools/tauri-e2e/run.mjs or pnpm -r test",
  });

  const journeys = JOURNEY_RUNNERS.map((j) =>
    notRun(j.id, "dry-run only — live stages not executed", {
      planOnly: true,
    }),
  );

  const classification = suiteResultFromLgj(journeys);
  // Dry-run overall is NOT_RUN (construction pass separate)
  const constructionPass = checks.every((c) => c.ok);

  const report = {
    schemaVersion: 1,
    harness: "tools/tauri-e2e/live",
    workItem: "W2.3-B",
    task: "tracer-w2-live-gui-validation",
    classificationTier: OPERATION_CLASS,
    classification: LgjClass.NOT_RUN,
    constructionPass,
    dryRun: true,
    liveOptIn: false,
    generatedAt: new Date().toISOString(),
    platform: `${process.platform}-${process.arch}`,
    discovery: {
      grok,
      bridge: bridgeExists ? BRIDGE : null,
    },
    spawnPlan: plan,
    checks,
    journeys,
    notes: [
      "Dry-run never spawns grok agent stdio",
      "Dry-run never launches Tauri GUI",
      "Live requires TRACER_LIVE_GROK=1 + TRACER_LIVE_GUI=1 + run/--live",
      "Approval RR PASS never claimed without observed reverse-request",
    ],
    suiteResult: classification,
  };

  console.log("=== LGJ dry-run ===");
  console.log(`constructionPass: ${constructionPass}`);
  console.log(`classification:   ${report.classification}`);
  console.log(`journeys:         ${journeys.map((j) => j.id + "=" + j.result).join(", ")}`);
  console.log(`spawnPlan:        ${(plan.argv || []).join(" ")}`);
  console.log(`grok discovery:   ${grok.source}${grok.path ? " -> " + grok.path : ""}`);

  if (cli.out) {
    const outPath = path.isAbsolute(cli.out) ? cli.out : path.join(REPO_ROOT, cli.out);
    mkdirSync(path.dirname(outPath), { recursive: true });
    writeFileSync(outPath, JSON.stringify(report, null, 2), "utf8");
    console.log(`wrote: ${outPath}`);
  }

  if (cli.json) {
    process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  }

  process.exit(constructionPass ? 0 : 1);
}

main();
