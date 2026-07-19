#!/usr/bin/env node
/**
 * LGJ unit checks — CI-safe (no GUI, no Grok spawn, no network).
 *
 * Usage:
 *   node tools/tauri-e2e/live/unit.mjs
 *
 * Exit 0 on all assertions pass; 1 on failure.
 * Never requires TRACER_LIVE_* env.
 */

import {
  LgjClass,
  suiteResultFromLgj,
  looksLikeAuthBlock,
  pass,
  fail,
  notRun,
  notObserved,
  unsupported,
  blockedAuth,
  blockedTooling,
  partial,
} from "./lib/classify.mjs";
import {
  checkLiveOptIn,
  isSecretLookingPrompt,
  parseArgs,
  DEFAULT_STREAM_PROMPT,
  DEFAULT_APPROVAL_PROMPT,
  DEFAULT_CANCEL_PROMPT,
} from "./lib/opt-in.mjs";
import { filterJourneys, JOURNEY_RUNNERS } from "./lib/journeys.mjs";
import { stockGrokSpawnPlan } from "./launch-live-grok.mjs";
import {
  sanitizeArtifactText,
  sanitizeJsonValue,
} from "./lib/sanitize.mjs";
import {
  checkPromptBound,
  MAX_PROMPT_CHARS,
  MAX_JOURNEYS_PER_RUN,
  CANCEL_DEADLOCK_MS,
  SESSION_READY_TIMEOUT_MS,
  STREAM_EVENT_TIMEOUT_MS,
  APPROVAL_OBSERVE_TIMEOUT_MS,
  ORPHAN_CHECK_NAMES,
} from "./lib/policy.mjs";
import { findOrphans } from "../lib/process.mjs";

let failed = 0;

function assert(cond, msg) {
  if (!cond) {
    console.error(`FAIL: ${msg}`);
    failed += 1;
  } else {
    console.log(`ok: ${msg}`);
  }
}

function eq(a, b, msg) {
  assert(a === b, `${msg} (got ${JSON.stringify(a)}, want ${JSON.stringify(b)})`);
}

// --- suite aggregation ---
eq(suiteResultFromLgj([]), LgjClass.NOT_RUN, "empty → NOT_RUN");
eq(
  suiteResultFromLgj([
    pass("LGJ-01", "x"),
    pass("LGJ-02", "x"),
  ]),
  LgjClass.PASS,
  "all PASS → PASS",
);
eq(
  suiteResultFromLgj([notRun("LGJ-01", "d"), notRun("LGJ-02", "d")]),
  LgjClass.NOT_RUN,
  "all NOT_RUN → NOT_RUN",
);
eq(
  suiteResultFromLgj([pass("LGJ-01", "x"), fail("LGJ-02", "x")]),
  LgjClass.FAIL,
  "any FAIL → FAIL",
);
eq(
  suiteResultFromLgj([
    blockedAuth("LGJ-01", "a"),
    blockedAuth("LGJ-02", "a"),
  ]),
  LgjClass.BLOCKED_BY_AUTH,
  "all auth → BLOCKED_BY_AUTH",
);
eq(
  suiteResultFromLgj([
    pass("LGJ-06", "x"),
    blockedAuth("LGJ-01", "a"),
  ]),
  LgjClass.PARTIAL,
  "PASS + auth → PARTIAL",
);
eq(
  suiteResultFromLgj([
    blockedTooling("LGJ-01", "t"),
    blockedTooling("LGJ-02", "t"),
  ]),
  LgjClass.BLOCKED_BY_TOOLING,
  "all tooling → BLOCKED_BY_TOOLING",
);
eq(
  suiteResultFromLgj([
    pass("LGJ-01", "x"),
    notObserved("LGJ-05", "rr"),
  ]),
  LgjClass.PARTIAL,
  "PASS + NOT_OBSERVED → PARTIAL",
);
eq(
  suiteResultFromLgj([
    pass("LGJ-01", "x"),
    unsupported("LGJ-05", "no rr"),
  ]),
  LgjClass.PARTIAL,
  "PASS + UNSUPPORTED → PARTIAL",
);
eq(
  suiteResultFromLgj([
    pass("LGJ-01", "x"),
    partial("LGJ-03", "fast"),
  ]),
  LgjClass.PARTIAL,
  "PASS + PARTIAL → PARTIAL",
);

// Honesty: never auto-promote RR absence to PASS
const rrAbsent = notObserved("LGJ-05", "no card");
assert(rrAbsent.result !== LgjClass.PASS, "NOT_OBSERVED is not PASS");
const rrUnsupported = unsupported("LGJ-05", "provider skipped tools");
assert(rrUnsupported.result !== LgjClass.PASS, "UNSUPPORTED is not PASS");

// Auth heuristics
assert(looksLikeAuthBlock("failed", "Authentication required"), "auth phrase detected");
assert(looksLikeAuthBlock(null, "login required"), "login required detected");
assert(looksLikeAuthBlock(null, "unauthorized"), "unauthorized detected");
assert(!looksLikeAuthBlock("ready", "session ready"), "ready is not auth");

// Opt-in triple gate (clear live env for unit isolation)
const prevGrok = process.env.TRACER_LIVE_GROK;
const prevSmoke = process.env.TRACER_LIVE_SMOKE;
const prevGui = process.env.TRACER_LIVE_GUI;
const prevAuth = process.env.TRACER_LIVE_GUI_AUTHORIZED;
delete process.env.TRACER_LIVE_GROK;
delete process.env.TRACER_LIVE_SMOKE;
delete process.env.TRACER_LIVE_GUI;
delete process.env.TRACER_LIVE_GUI_AUTHORIZED;

assert(!checkLiveOptIn(parseArgs([])).ok, "no env/cli → gate closed");
assert(!checkLiveOptIn(parseArgs(["run"])).ok, "run without env → closed");
process.env.TRACER_LIVE_GROK = "1";
assert(!checkLiveOptIn(parseArgs(["run"])).ok, "grok only + run → closed");
process.env.TRACER_LIVE_GUI = "1";
assert(!checkLiveOptIn(parseArgs([])).ok, "env without run → closed");
assert(!checkLiveOptIn(parseArgs(["run"])).ok, "dual env + run without authorization → closed");
process.env.TRACER_LIVE_GUI_AUTHORIZED = "1";
assert(checkLiveOptIn(parseArgs(["run"])).ok, "triple env + run → open");
assert(checkLiveOptIn(parseArgs(["--live"])).ok, "triple env + --live → open");
delete process.env.TRACER_LIVE_GUI_AUTHORIZED;
assert(!checkLiveOptIn(parseArgs(["run"])).ok, "authorization removed → closed again");

// restore
if (prevGrok === undefined) delete process.env.TRACER_LIVE_GROK;
else process.env.TRACER_LIVE_GROK = prevGrok;
if (prevSmoke === undefined) delete process.env.TRACER_LIVE_SMOKE;
else process.env.TRACER_LIVE_SMOKE = prevSmoke;
if (prevGui === undefined) delete process.env.TRACER_LIVE_GUI;
else process.env.TRACER_LIVE_GUI = prevGui;
if (prevAuth === undefined) delete process.env.TRACER_LIVE_GUI_AUTHORIZED;
else process.env.TRACER_LIVE_GUI_AUTHORIZED = prevAuth;

// Secret-looking prompt rejection
assert(isSecretLookingPrompt("api_key=sk-abc"), "secret api_key rejected");
assert(isSecretLookingPrompt("Bearer abcdefghijklmnopqrstuvwxyz012345"), "bearer rejected");
assert(!isSecretLookingPrompt(DEFAULT_STREAM_PROMPT), "default stream prompt public-safe");
assert(!isSecretLookingPrompt(DEFAULT_APPROVAL_PROMPT), "default approval prompt public-safe");
assert(!isSecretLookingPrompt(DEFAULT_CANCEL_PROMPT), "default cancel prompt public-safe");

// Prompt bounding
assert(checkPromptBound("short").ok, "short prompt within bound");
assert(checkPromptBound("x".repeat(MAX_PROMPT_CHARS)).ok, "exact max prompt ok");
assert(!checkPromptBound("x".repeat(MAX_PROMPT_CHARS + 1)).ok, "oversize prompt rejected");
assert(DEFAULT_STREAM_PROMPT.length <= MAX_PROMPT_CHARS, "default stream within bound");
assert(DEFAULT_APPROVAL_PROMPT.length <= MAX_PROMPT_CHARS, "default approval within bound");

// Timeout / run policy
assert(CANCEL_DEADLOCK_MS === 45_000, "cancel deadlock budget");
assert(SESSION_READY_TIMEOUT_MS > 0, "session ready timeout set");
assert(STREAM_EVENT_TIMEOUT_MS > 0, "stream timeout set");
assert(APPROVAL_OBSERVE_TIMEOUT_MS > 0, "approval observe timeout set");
eq(MAX_JOURNEYS_PER_RUN, 7, "max journeys = 7");
assert(ORPHAN_CHECK_NAMES.includes("grok"), "orphan check includes grok");
assert(ORPHAN_CHECK_NAMES.includes("tracer-desktop"), "orphan check includes tracer-desktop");

// Artifact sanitization
assert(
  sanitizeArtifactText("Authorization: Bearer abcdefghijklmnop").includes("[REDACTED]"),
  "bearer sanitized",
);
assert(
  sanitizeArtifactText("api_key=sk-abcdefghijklmnop").includes("[REDACTED]"),
  "api_key sanitized",
);
assert(
  sanitizeArtifactText("C:\\Users\\Alice\\secret").includes("[USER]"),
  "windows user path sanitized",
);
assert(
  sanitizeArtifactText("/Users/alice/.grok/token").includes("[USER]"),
  "unix user path sanitized",
);
const sanitizedObj = /** @type {any} */ (
  sanitizeJsonValue({
    token: "super-secret",
    nested: { password: "x", ok: "safe" },
  })
);
eq(sanitizedObj.token, "[REDACTED]", "json token key redacted");
eq(sanitizedObj.nested.password, "[REDACTED]", "nested password redacted");
eq(sanitizedObj.nested.ok, "safe", "non-secret json preserved");

// Journey catalog
eq(JOURNEY_RUNNERS.length, 7, "7 journeys");
eq(filterJourneys(null).length, 7, "filter null = all");
eq(filterJourneys("LGJ-01,LGJ-05").length, 2, "filter subset");
eq(filterJourneys("01").map((j) => j.id)[0], "LGJ-01", "filter short id");
assert(JOURNEY_RUNNERS.length <= MAX_JOURNEYS_PER_RUN, "catalog within max journeys");

// Spawn plan / command construction (W0-B / W1-D)
const plan = stockGrokSpawnPlan("grok");
assert(plan.matchesW0bW1d === true, "spawn plan matches W0-B/W1-D");
eq(plan.args.join(" "), "agent --no-leader stdio", "stock grok argv");
eq(plan.argv.join(" "), "grok agent --no-leader stdio", "full argv");

// Process ownership API available (no live spawn)
assert(typeof findOrphans === "function", "findOrphans export present");
const orphans = findOrphans(["__tracer_lgj_unit_nonexistent__"]);
assert(Array.isArray(orphans), "findOrphans returns array");
assert(orphans.length === 0, "no orphans for nonexistent name");

console.log("");
if (failed) {
  console.error(`${failed} assertion(s) failed`);
  process.exit(1);
}
console.log("All LGJ unit checks passed");
process.exit(0);
