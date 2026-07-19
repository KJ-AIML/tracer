/**
 * Opt-in gates for Live Grok GUI validation (W2.3-B / W2.4.3-A).
 *
 * Live requires ALL of:
 *   TRACER_LIVE_GROK=1  (or TRACER_LIVE_SMOKE=1)
 *   TRACER_LIVE_GUI=1
 *   TRACER_LIVE_GUI_AUTHORIZED=1  (operator authorization — W2.4.3-A)
 *   explicit CLI `run` / `--live`
 *
 * Dry-run never requires env gates (including authorization) and never spawns live Grok.
 */

export const OPERATION_CLASS = "manual_local_live_authenticated_gui";

/**
 * @param {string[]} args process.argv.slice(2)
 */
export function parseArgs(args) {
  const set = new Set(args);
  const get = (name) => {
    const i = args.indexOf(name);
    if (i >= 0 && args[i + 1]) return args[i + 1];
    const eq = args.find((a) => a.startsWith(name + "="));
    return eq ? eq.split("=").slice(1).join("=") : null;
  };
  return {
    dryRun: set.has("dry-run") || set.has("--dry-run"),
    live: set.has("run") || set.has("--live") || set.has("live"),
    help: set.has("help") || set.has("--help") || set.has("-h"),
    json: set.has("--json"),
    skipBuild: set.has("--skip-build"),
    allowUnauth: set.has("--allow-unauth"),
    journey: get("--journey"),
    out: get("--out"),
    prompt: get("--prompt"),
    grok: get("--grok"),
  };
}

export function envLiveGrok() {
  return (
    process.env.TRACER_LIVE_GROK === "1" ||
    process.env.TRACER_LIVE_SMOKE === "1"
  );
}

export function envLiveGui() {
  return process.env.TRACER_LIVE_GUI === "1";
}

/** W2.4.3-A operator authorization gate — separate from dual opt-in. */
export function envLiveGuiAuthorized() {
  return process.env.TRACER_LIVE_GUI_AUTHORIZED === "1";
}

/**
 * Snapshot of live opt-in / authorization env (for reports; never prints secrets).
 */
export function liveEnvSnapshot() {
  return {
    TRACER_LIVE_GROK: envLiveGrok(),
    TRACER_LIVE_GUI: envLiveGui(),
    TRACER_LIVE_GUI_AUTHORIZED: envLiveGuiAuthorized(),
  };
}

/**
 * Live triple-gate: env pair + operator authorization + explicit run/--live.
 * Returns { ok, reason, operationClass, grok, gui, authorized }
 */
export function checkLiveOptIn(cli) {
  const grok = envLiveGrok();
  const gui = envLiveGui();
  const authorized = envLiveGuiAuthorized();
  const explicit = Boolean(cli.live);

  if (!explicit && !(grok && gui)) {
    return {
      ok: false,
      reason:
        "Live GUI requires TRACER_LIVE_GROK=1, TRACER_LIVE_GUI=1, TRACER_LIVE_GUI_AUTHORIZED=1, plus `run`/`--live`. Use `dry-run` for plan-only.",
      operationClass: OPERATION_CLASS,
      grok,
      gui,
      authorized,
    };
  }
  if (!grok) {
    return {
      ok: false,
      reason: "TRACER_LIVE_GROK=1 (or TRACER_LIVE_SMOKE=1) required for live GUI",
      operationClass: OPERATION_CLASS,
      grok,
      gui,
      authorized,
    };
  }
  if (!gui && !explicit) {
    return {
      ok: false,
      reason: "TRACER_LIVE_GUI=1 required for live GUI",
      operationClass: OPERATION_CLASS,
      grok,
      gui,
      authorized,
    };
  }
  // Require explicit run intent even when envs set (safety dual-gate)
  if (!explicit) {
    return {
      ok: false,
      reason:
        "Explicit `run` or `--live` subcommand required (env alone is insufficient)",
      operationClass: OPERATION_CLASS,
      grok,
      gui,
      authorized,
    };
  }
  // If --live without TRACER_LIVE_GUI, still require both envs for belt+suspenders
  if (!gui) {
    return {
      ok: false,
      reason: "TRACER_LIVE_GUI=1 required alongside TRACER_LIVE_GROK=1",
      operationClass: OPERATION_CLASS,
      grok,
      gui,
      authorized,
    };
  }
  if (!authorized) {
    return {
      ok: false,
      reason:
        "TRACER_LIVE_GUI_AUTHORIZED=1 required for live GUI (operator authorization gate — W2.4.3-A)",
      operationClass: OPERATION_CLASS,
      grok: true,
      gui: true,
      authorized: false,
    };
  }
  return {
    ok: true,
    reason: "opt-in satisfied",
    operationClass: OPERATION_CLASS,
    grok: true,
    gui: true,
    authorized: true,
  };
}

/**
 * Sanitized execution plan printed before any provider-capable live path (W2.4.3-A).
 * @param {{ journeyIds: string[], promptOverride?: string|null }} opts
 */
export function printExecutionPlan({ journeyIds, promptOverride = null }) {
  const ids = journeyIds?.length ? journeyIds : [
    "LGJ-01", "LGJ-02", "LGJ-03", "LGJ-04", "LGJ-05", "LGJ-06", "LGJ-07",
  ];
  const budget = {
    "LGJ-01": 0,
    "LGJ-02": 1,
    "LGJ-03": 1,
    "LGJ-04": 0,
    "LGJ-05": 2,
    "LGJ-06": 0,
    "LGJ-07": 0,
  };
  const selectedBudget = ids.reduce((n, id) => n + (budget[id] ?? 0), 0);
  console.log("");
  console.log("=== LIVE EXECUTION PLAN (sanitized) ===");
  console.log(`scenarioIds:           ${ids.join(", ")}`);
  console.log(`providerPromptBudget:  hard max ~3 (selected plan: ${selectedBudget})`);
  console.log(`maxPromptLength:       ${500} chars (MAX_PROMPT_CHARS)`);
  console.log(`maxAttempts:           1 per journey; LGJ-05 max 1-2 approval observe cycles`);
  console.log(`maxRuntime:            soft wall 30m (SUITE_SOFT_WALL_MS)`);
  console.log(`cancellationScenarios: LGJ-03 (cancel mid-stream; deadlock budget 45s)`);
  console.log(`approvalPolicy:        LGJ-05 PASS only if RR observed; never fabricate`);
  console.log(`artifactRetention:     artifacts/tauri-e2e-live/<runId>/ (sanitized; gitignored)`);
  console.log(`providerUsageClass:    manual_local_live_authenticated_gui / BOUNDED`);
  if (promptOverride) {
    console.log(`promptOverride:        yes (${String(promptOverride).length} chars, public-safe only)`);
  } else {
    console.log("promptOverride:        no (stock public-safe defaults)");
  }
  console.log("=======================================");
  console.log("");
}

/** Print operation class banner before any provider-capable path. */
export function printOperationClass({ live }) {
  console.log("");
  console.log("=== OPERATION CLASS ===");
  console.log(`class:          ${OPERATION_CLASS}`);
  console.log(`mode:           ${live ? "LIVE (provider usage possible)" : "DRY-RUN (no live spawn)"}`);
  console.log("credentials:    existing local auth only — never printed");
  console.log("prompts:        public-safe / bounded only");
  console.log("artifacts:      sanitized (tokens/paths redacted)");
  console.log("standard CI:    FORBIDDEN — opt-in manual local Windows GUI only");
  console.log("=======================");
  console.log("");
}

export function isSecretLookingPrompt(text) {
  if (!text) return false;
  const s = String(text);
  return (
    /api[_-]?key/i.test(s) ||
    /bearer\s+[a-z0-9._-]{20,}/i.test(s) ||
    /sk-[a-zA-Z0-9]{10,}/.test(s) ||
    /password\s*[:=]/i.test(s) ||
    /-----BEGIN/.test(s)
  );
}

/** Public-safe default prompts (never private). */
export const DEFAULT_STREAM_PROMPT =
  "Reply with the single word pong and then stop. Do not use tools.";

export const DEFAULT_APPROVAL_PROMPT =
  "Create a new text file named tracer-lgj-probe.txt in the current working directory " +
  "containing the single word probe, then stop. Use a file tool if available.";

export const DEFAULT_CANCEL_PROMPT =
  "Count slowly from 1 to 200 in words, one number per line. Do not stop early.";
