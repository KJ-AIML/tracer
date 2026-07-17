#!/usr/bin/env node
/**
 * CLI entry: deterministic fake ACP agent over stdio (NDJSON JSON-RPC 2.0).
 *
 * Scenario selection (first match wins):
 *   1. --scenario <id>
 *   2. env TRACER_FAKE_ACP_SCENARIO=<id>
 *
 * Never performs network I/O or reads provider credentials.
 */

import { runFakeRuntime, listScenarioIds, IMPLEMENTED_SCENARIOS, LIVE_ONLY_SCENARIOS } from "../src/index.js";

function printHelp() {
  const text = `
fake-acp-runtime — Tracer deterministic fake ACP process (W1-G)

Usage:
  fake-acp-runtime --scenario <id>
  TRACER_FAKE_ACP_SCENARIO=<id> fake-acp-runtime

Options:
  --scenario <id>         Catalog scenario id (standardCi only)
  --list-scenarios        Print implemented scenario ids and exit
  --chunk-delay-ms <n>    Delay between stream chunks (default 0)
  --cancel-delay-ms <n>   Delay before honoring cancel (slow_cancel_ack default 60000)
  --help                  Show help

Env:
  TRACER_FAKE_ACP_SCENARIO
  TRACER_FAKE_ACP_CHUNK_DELAY_MS
  TRACER_FAKE_ACP_CANCEL_DELAY_MS

Transport:
  stdin  — NDJSON JSON-RPC requests/notifications from Tracer client
  stdout — NDJSON JSON-RPC responses/notifications (agent)
  stderr — logs only

Evidence:
  Fake runtime output is evidence label "fake-runtime" (or synthetic for vendor-unknown).
  It MUST NOT be claimed as live stock Grok multi-turn parity.
  Live-only catalog ids are rejected: ${LIVE_ONLY_SCENARIOS.join(", ")}
`.trim();
  process.stdout.write(`${text}\n`);
}

function parseArgs(argv) {
  const out = {
    scenario: process.env.TRACER_FAKE_ACP_SCENARIO || null,
    list: false,
    help: false,
    chunkDelayMs: process.env.TRACER_FAKE_ACP_CHUNK_DELAY_MS
      ? Number(process.env.TRACER_FAKE_ACP_CHUNK_DELAY_MS)
      : 0,
    cancelDelayMs: process.env.TRACER_FAKE_ACP_CANCEL_DELAY_MS
      ? Number(process.env.TRACER_FAKE_ACP_CANCEL_DELAY_MS)
      : undefined,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--scenario" || a === "-s") {
      out.scenario = argv[++i];
    } else if (a.startsWith("--scenario=")) {
      out.scenario = a.slice("--scenario=".length);
    } else if (a === "--list-scenarios") {
      out.list = true;
    } else if (a === "--help" || a === "-h") {
      out.help = true;
    } else if (a === "--chunk-delay-ms") {
      out.chunkDelayMs = Number(argv[++i]);
    } else if (a.startsWith("--chunk-delay-ms=")) {
      out.chunkDelayMs = Number(a.slice("--chunk-delay-ms=".length));
    } else if (a === "--cancel-delay-ms") {
      out.cancelDelayMs = Number(argv[++i]);
    } else if (a.startsWith("--cancel-delay-ms=")) {
      out.cancelDelayMs = Number(a.slice("--cancel-delay-ms=".length));
    } else if (a.startsWith("-")) {
      throw new Error(`Unknown flag: ${a}`);
    }
  }
  return out;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printHelp();
    process.exit(0);
  }
  if (args.list) {
    for (const id of listScenarioIds()) {
      process.stdout.write(`${id}\n`);
    }
    process.exit(0);
  }
  if (!args.scenario) {
    process.stderr.write(
      "error: --scenario <id> or TRACER_FAKE_ACP_SCENARIO is required\n" +
        `implemented: ${IMPLEMENTED_SCENARIOS.join(", ")}\n`,
    );
    process.exit(2);
  }

  // Refuse live-only ids explicitly
  if (LIVE_ONLY_SCENARIOS.includes(args.scenario)) {
    process.stderr.write(
      `error: scenario "${args.scenario}" is live-only and is not implemented by the fake runtime\n`,
    );
    process.exit(2);
  }

  await runFakeRuntime({
    scenarioId: args.scenario,
    chunkDelayMs: args.chunkDelayMs,
    cancelDelayMs: args.cancelDelayMs,
    stdin: process.stdin,
    stdout: process.stdout,
    stderr: process.stderr,
    onExit(code) {
      process.exit(code);
    },
  });
}

main().catch((e) => {
  process.stderr.write(`fatal: ${e?.stack || e}\n`);
  process.exit(1);
});
