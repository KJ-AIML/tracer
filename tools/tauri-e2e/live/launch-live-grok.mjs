#!/usr/bin/env node
/**
 * Minimal test-only ACP stdio bridge for Live Grok GUI validation (W2.3-B).
 *
 * Control plane session create always spawns:
 *   node <script> --scenario <id>
 * This bridge ignores the scenario id and execs the stock Grok ACP path:
 *   grok agent --no-leader stdio
 *
 * Not a product surface. Never print tokens. Preserve operator env (GROK_HOME).
 *
 * Env:
 *   TRACER_GROK_BIN   absolute/relative path to grok (else PATH `grok`)
 *   GROK_HOME         optional hermetic/operator home (passed through)
 */

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";

function resolveGrok() {
  const fromEnv = process.env.TRACER_GROK_BIN?.trim();
  if (fromEnv) {
    if (existsSync(fromEnv) || existsSync(fromEnv + ".exe")) {
      return fromEnv;
    }
    return fromEnv;
  }
  return "grok";
}

/** Documented spawn plan (for dry-run evidence). */
export function stockGrokSpawnPlan(executable = "grok") {
  return {
    executable,
    args: ["agent", "--no-leader", "stdio"],
    matchesW0bW1d: true,
    argv: [executable, "agent", "--no-leader", "stdio"],
    note: "bridge ignores --scenario; stock Grok ACP only",
  };
}

function main() {
  const argv = process.argv.slice(2);
  if (argv.includes("--help") || argv.includes("-h")) {
    process.stderr.write(
      "launch-live-grok.mjs — test-only ACP bridge\n" +
        "Usage: node launch-live-grok.mjs [--scenario <id>]\n" +
        "Spawns: grok agent --no-leader stdio (TRACER_GROK_BIN override)\n",
    );
    process.exit(0);
  }
  // --plan: print spawn plan JSON and exit (dry-run helper)
  if (argv.includes("--plan")) {
    process.stdout.write(JSON.stringify(stockGrokSpawnPlan(resolveGrok()), null, 2) + "\n");
    process.exit(0);
  }

  const grok = resolveGrok();
  const args = ["agent", "--no-leader", "stdio"];
  const base = path.basename(grok);
  process.stderr.write(
    `[live-grok-bridge] spawning ${base} ${args.join(" ")} (stdio bridge)\n`,
  );

  const child = spawn(grok, args, {
    stdio: ["pipe", "pipe", "pipe"],
    env: process.env,
    windowsHide: true,
  });

  if (!child.stdin || !child.stdout) {
    process.stderr.write("[live-grok-bridge] failed to allocate child stdio\n");
    process.exit(1);
  }

  process.stdin.pipe(child.stdin);
  child.stdout.pipe(process.stdout);
  child.stderr.pipe(process.stderr);

  child.on("error", (err) => {
    process.stderr.write(
      `[live-grok-bridge] spawn error: ${err instanceof Error ? err.message : String(err)}\n`,
    );
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) process.exit(1);
    process.exit(code ?? 0);
  });

  process.on("SIGINT", () => {
    try {
      child.kill("SIGTERM");
    } catch {
      /* ignore */
    }
  });
  process.on("SIGTERM", () => {
    try {
      child.kill("SIGTERM");
    } catch {
      /* ignore */
    }
  });
  process.stdin.on("end", () => {
    try {
      child.stdin.end();
    } catch {
      /* ignore */
    }
  });
}

// Only run as CLI when executed directly
const isMain =
  process.argv[1] &&
  (process.argv[1].endsWith("launch-live-grok.mjs") ||
    process.argv[1].includes("launch-live-grok"));
if (isMain) {
  main();
}
