/**
 * Opt-in gates for Live Grok GUI validation (W2.3-B).
 *
 * Live requires BOTH:
 *   TRACER_LIVE_GROK=1  (or TRACER_LIVE_SMOKE=1)
 *   TRACER_LIVE_GUI=1
 * OR explicit CLI --live / run subcommand.
 *
 * Dry-run never requires env gates and never spawns live Grok.
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

/**
 * Live dual-gate: env pair OR explicit --live with both envs.
 * Returns { ok, reason, operationClass }
 */
export function checkLiveOptIn(cli) {
  const grok = envLiveGrok();
  const gui = envLiveGui();
  const explicit = Boolean(cli.live);

  if (!explicit && !(grok && gui)) {
    return {
      ok: false,
      reason:
        "Live GUI requires TRACER_LIVE_GROK=1 and TRACER_LIVE_GUI=1 plus `run`/`--live`. Use `dry-run` for plan-only.",
      operationClass: OPERATION_CLASS,
      grok,
      gui,
    };
  }
  if (!grok) {
    return {
      ok: false,
      reason: "TRACER_LIVE_GROK=1 (or TRACER_LIVE_SMOKE=1) required for live GUI",
      operationClass: OPERATION_CLASS,
      grok,
      gui,
    };
  }
  if (!gui && !explicit) {
    return {
      ok: false,
      reason: "TRACER_LIVE_GUI=1 required for live GUI",
      operationClass: OPERATION_CLASS,
      grok,
      gui,
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
    };
  }
  return {
    ok: true,
    reason: "opt-in satisfied",
    operationClass: OPERATION_CLASS,
    grok: true,
    gui: true,
  };
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
