#!/usr/bin/env node
/**
 * LGJ — Live Grok GUI product journeys (W2.3-B).
 *
 * Lifecycle (live only):
 *   opt-in gate → doctor/tooling → build → msedgedriver → tauri-driver
 *   → launch isolated env with live Grok bridge (not fake ACP)
 *   → WebDriver session → LGJ-01…07 → sanitized artifacts
 *   → cleanup → orphan check
 *
 * Opt-in:
 *   TRACER_LIVE_GROK=1  AND  TRACER_LIVE_GUI=1  AND  TRACER_LIVE_GUI_AUTHORIZED=1
 *   AND  `run`/`--live`
 *
 * Dry-run:
 *   node tools/tauri-e2e/live/dry-run.mjs
 *   (or) node tools/tauri-e2e/live/lgj.mjs dry-run
 *
 * Never part of pnpm -r test / standard CI.
 */

import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";
import {
  ResultClass,
  FailureCode,
  StageId,
} from "../lib/classify.mjs";
import {
  REPO_ROOT,
  FRONTEND_DIST,
  discoverEnvironment,
  resolvePreferredBinary,
  resolveDriverSpawnPaths,
} from "../lib/discover.mjs";
import {
  spawnOwned,
  findOrphans,
  reapOrphans,
  uniqueTempDir,
  installExitHooks,
  stopAllOwned,
  processAlive,
} from "../lib/process.mjs";
import { createStageReport } from "../lib/stages.mjs";
import { WebDriverClient, waitDriverReady } from "../lib/webdriver.mjs";
import { attrTestId, waitAppReady } from "../lib/gui.mjs";
import {
  LgjClass,
  suiteResultFromLgj,
} from "./lib/classify.mjs";
import {
  parseArgs,
  checkLiveOptIn,
  printOperationClass,
  printExecutionPlan,
  isSecretLookingPrompt,
  DEFAULT_STREAM_PROMPT,
  DEFAULT_APPROVAL_PROMPT,
  DEFAULT_CANCEL_PROMPT,
  OPERATION_CLASS,
} from "./lib/opt-in.mjs";
import { filterJourneys } from "./lib/journeys.mjs";
import { sanitizeArtifactText } from "./lib/sanitize.mjs";
import {
  checkPromptBound,
  ORPHAN_CHECK_NAMES,
  WD_SESSION_TIMEOUT_MS,
  APP_READY_TIMEOUT_MS,
  MAX_JOURNEYS_PER_RUN,
} from "./lib/policy.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const LIVE_BRIDGE = path.join(__dirname, "launch-live-grok.mjs");

const args = process.argv.slice(2);
const cli = parseArgs(args);

if (cli.help || args.length === 0) {
  console.log(`Live Grok GUI (LGJ) — W2.3-B

USAGE:
  node tools/tauri-e2e/live/lgj.mjs dry-run [--out path] [--json]
  node tools/tauri-e2e/live/lgj.mjs run --live   # requires env gates
  node tools/tauri-e2e/live/dry-run.mjs          # alias for dry-run

ENV (live):
  TRACER_LIVE_GROK=1   (or TRACER_LIVE_SMOKE=1)
  TRACER_LIVE_GUI=1
  TRACER_LIVE_GUI_AUTHORIZED=1   (operator authorization — W2.4.3-A)
  TRACER_GROK_BIN      optional grok path
  GROK_HOME            optional hermetic/operator home

OPTIONS:
  --journey LGJ-01,LGJ-02
  --skip-build
  --out <path>         write sanitized JSON report
  --prompt <text>      public-safe stream prompt
  --json
  --allow-unauth       treat auth blocks as non-FAIL overall when all blocked
`);
  process.exit(args.length === 0 ? 0 : 0);
}

// Delegate dry-run
if (cli.dryRun || args[0] === "dry-run") {
  const dry = path.join(__dirname, "dry-run.mjs");
  const r = spawnSync(process.execPath, [dry, ...args.filter((a) => a !== "dry-run" && a !== "--dry-run")], {
    stdio: "inherit",
    cwd: REPO_ROOT,
    env: process.env,
  });
  process.exit(r.status ?? 1);
}

const liveGate = checkLiveOptIn(cli);
if (!liveGate.ok) {
  printOperationClass({ live: false });
  console.error(`error: ${liveGate.reason}`);
  console.error("hint: use dry-run, or set TRACER_LIVE_GROK=1 + TRACER_LIVE_GUI=1 + TRACER_LIVE_GUI_AUTHORIZED=1 and pass run/--live");
  process.exit(2);
}

if (cli.prompt && isSecretLookingPrompt(cli.prompt)) {
  console.error("error: --prompt looks secret-bearing; refuse to run (public-safe only)");
  process.exit(2);
}

const promptBound = checkPromptBound(cli.prompt);
if (!promptBound.ok) {
  console.error(`error: ${promptBound.reason}`);
  process.exit(2);
}

printOperationClass({ live: true });
const journeyFilter = cli.journey;
const journeysPlanned = filterJourneys(journeyFilter);
printExecutionPlan({
  journeyIds: journeysPlanned.map((j) => j.id),
  promptOverride: cli.prompt || null,
});
console.log("Intent confirmed via triple opt-in (env + authorization + run). Provider usage possible.");
console.log("Credentials/tokens will not be printed. Prompts are public-safe/bounded.");
console.log(
  `Limits: MAX_JOURNEYS_PER_RUN=${MAX_JOURNEYS_PER_RUN}, WD_SESSION=${WD_SESSION_TIMEOUT_MS}ms, APP_READY=${APP_READY_TIMEOUT_MS}ms`,
);

const jsonOut = cli.json;
const skipBuild = cli.skipBuild;
const port = Number(process.env.TRACER_TAURI_DRIVER_PORT || 4444);
const host = process.env.TRACER_TAURI_DRIVER_HOST || "127.0.0.1";
const baseUrl = `http://${host}:${port}`;
const profile = process.env.TRACER_E2E_PROFILE || "debug";

if (cli.grok) {
  process.env.TRACER_GROK_BIN = cli.grok;
}

installExitHooks();

function runCapture(cmd, cmdArgs, opts = {}) {
  return spawnSync(cmd, cmdArgs, {
    cwd: opts.cwd ?? REPO_ROOT,
    env: { ...process.env, ...(opts.env || {}) },
    encoding: "utf8",
    shell: process.platform === "win32",
    windowsHide: true,
    timeout: opts.timeoutMs ?? 600_000,
  });
}

function ensureFrontend({ build }) {
  const indexHtml = path.join(FRONTEND_DIST, "index.html");
  if (existsSync(indexHtml) && !build) return { path: FRONTEND_DIST, built: false };
  if (build) {
    const r = runCapture("pnpm", ["--filter", "@tracer/desktop", "build"], {
      timeoutMs: 600_000,
    });
    if (r.status !== 0) {
      throw Object.assign(
        new Error(`frontend build failed: ${(r.stderr || r.stdout || "").slice(0, 800)}`),
        { code: FailureCode.FRONTEND_DIST_NOT_FOUND },
      );
    }
  }
  if (!existsSync(indexHtml)) {
    throw Object.assign(new Error("frontend dist missing"), {
      code: FailureCode.FRONTEND_DIST_NOT_FOUND,
    });
  }
  return { path: FRONTEND_DIST, built: Boolean(build) };
}

function ensureBinary({ build }) {
  if (process.env.TRACER_E2E_APP_BINARY && existsSync(process.env.TRACER_E2E_APP_BINARY)) {
    return process.env.TRACER_E2E_APP_BINARY;
  }
  let bin = resolvePreferredBinary({ preferRelease: profile === "release" });
  if (bin?.path && !build) return bin.path;
  if (build || !bin?.path) {
    const r = runCapture(
      "cargo",
      ["build", "-p", "tracer-desktop", ...(profile === "release" ? ["--release"] : [])],
      { timeoutMs: 900_000 },
    );
    if (r.status !== 0) {
      throw Object.assign(
        new Error(`cargo build failed: ${(r.stderr || r.stdout || "").slice(0, 800)}`),
        { code: FailureCode.APP_BINARY_NOT_FOUND },
      );
    }
    bin = resolvePreferredBinary({ preferRelease: profile === "release" });
  }
  if (!bin?.path || !existsSync(bin.path)) {
    throw Object.assign(new Error("app binary missing after build"), {
      code: FailureCode.APP_BINARY_NOT_FOUND,
    });
  }
  return bin.path;
}

function runId() {
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  return `lgj-${stamp}-${process.pid}`;
}

async function main() {
  const env = discoverEnvironment();
  const report = createStageReport();
  const rid = runId();
  const workDir = uniqueTempDir("tracer-lgj");
  const artifactsDir = path.join(REPO_ROOT, "artifacts", "tauri-e2e-live", rid);
  mkdirSync(artifactsDir, { recursive: true });
  const logDir = path.join(workDir, "logs");
  mkdirSync(logDir, { recursive: true });
  const dbPath = path.join(workDir, "tracer-lgj.sqlite");
  const projectRoot = path.join(workDir, "sample-project");
  mkdirSync(projectRoot, { recursive: true });
  writeFileSync(path.join(projectRoot, "README.md"), "# LGJ live sample project\n", "utf8");
  const heliEmpty = path.join(workDir, "heli-empty");
  mkdirSync(heliEmpty, { recursive: true });
  const readyMarker = path.join(workDir, "app-ready.marker");

  /** @type {import('../lib/process.mjs').OwnedProcess | null} */
  let driver = null;
  const client = new WebDriverClient(baseUrl);
  const spawnPaths = resolveDriverSpawnPaths();
  let binary = null;
  /** @type {Record<string,string>} */
  let appEnv = {};

  console.log("=== LGJ Live Grok GUI Journeys (W2.3-B) ===");
  console.log(`runId: ${rid}`);
  console.log(`workDir: ${workDir}`);
  console.log(`artifacts: ${artifactsDir}`);
  console.log(`bridge: ${LIVE_BRIDGE}`);
  console.log(`journeys: ${journeyFilter || "ALL LGJ-01..LGJ-07"}`);

  if (!existsSync(LIVE_BRIDGE)) {
    console.error("error: live bridge missing:", LIVE_BRIDGE);
    process.exit(1);
  }

  const journeysToRun = filterJourneys(journeyFilter);
  const journeyResults = [];

  async function captureArtifact(label) {
    const dir = path.join(artifactsDir, label);
    mkdirSync(dir, { recursive: true });
    try {
      const src = await client.getPageSource();
      const body = sanitizeArtifactText(src.raw || JSON.stringify(src.body));
      writeFileSync(path.join(dir, "page.html"), body, "utf8");
    } catch (e) {
      writeFileSync(
        path.join(dir, "page-error.txt"),
        sanitizeArtifactText(e instanceof Error ? e.message : String(e)),
        "utf8",
      );
    }
    try {
      const probe = await client.execute(`
        return {
          ready: !!document.querySelector('[data-testid="tracer-app-ready"]'),
          backend: document.querySelector('[data-testid="tracer-app-root"]')?.getAttribute('data-tracer-backend'),
          route: document.querySelector('[data-testid="tracer-app-root"]')?.getAttribute('data-tracer-route'),
          status: document.querySelector('[data-testid="tracer-session-workspace"]')?.getAttribute('data-session-status'),
          title: document.title
        };
      `);
      writeFileSync(
        path.join(dir, "probe.json"),
        sanitizeArtifactText(JSON.stringify(probe.body, null, 2)),
        "utf8",
      );
    } catch {
      /* ignore */
    }
    return { dir };
  }

  function writeE2eEnvFile(envMap) {
    const envFile = path.join(workDir, "tracer-e2e.env");
    const body = Object.entries(envMap)
      .map(([k, v]) => `${k}=${v}`)
      .join("\n");
    writeFileSync(envFile, body + "\n", "utf8");
    return envFile;
  }

  async function startDriverAndApp() {
    if (driver) {
      try {
        await client.deleteSession().catch(() => {});
      } catch {
        /* ignore */
      }
      await driver.stop().catch(() => {});
      driver = null;
    }
    const driverArgs = ["--port", String(port)];
    const native = process.env.TRACER_NATIVE_DRIVER || spawnPaths.nativePath || null;
    if (native) driverArgs.push("--native-driver", native);
    driver = spawnOwned(spawnPaths.tauriPath, driverArgs, {
      label: "tauri-driver",
      logDir,
      windowsHide: true,
    });
    await waitDriverReady(baseUrl, { timeoutMs: 30_000 });

    // LIVE: point TRACER_FAKE_ACP_JS at the live Grok bridge (minimal test-only launch config)
    appEnv = {
      TRACER_DATABASE_PATH: dbPath,
      TRACER_FAKE_ACP_JS: LIVE_BRIDGE,
      TRACER_HELI_PROBE_PATH: heliEmpty,
      TRACER_NODE_BIN: process.env.TRACER_NODE_BIN || "node",
      TRACER_E2E_READY_MARKER: readyMarker,
      TRACER_E2E_PROFILE: profile,
    };
    if (process.env.TRACER_GROK_BIN) {
      // Not in allowlist for e2e env file — set on parent process; bridge inherits when node spawns grok
      // Desktop child won't get TRACER_GROK_BIN via allowlist; bridge resolves PATH/TRACER_GROK_BIN from
      // its own process env which is the node process spawned by the control plane (inherits desktop env).
      // Desktop process env may not include TRACER_GROK_BIN unless we inject via process-level only.
      // Workaround: write a small wrapper note — bridge uses PATH by default.
    }
    const envFile = writeE2eEnvFile(appEnv);
    console.log(`[lgj] e2e env file: ${envFile}`);
    console.log(`[lgj] live bridge as TRACER_FAKE_ACP_JS: ${LIVE_BRIDGE}`);
    const res = await client.newSession(
      {
        application: binary,
        args: [`--tracer-e2e-env=${envFile}`],
        env: {
          ...appEnv,
          ...(process.env.TRACER_GROK_BIN
            ? { TRACER_GROK_BIN: process.env.TRACER_GROK_BIN }
            : {}),
          ...(process.env.GROK_HOME ? { GROK_HOME: process.env.GROK_HOME } : {}),
        },
      },
      { timeoutMs: WD_SESSION_TIMEOUT_MS },
    );
    if (!client.sessionId) {
      throw Object.assign(
        new Error(
          `WebDriver new session failed: HTTP ${res.statusCode} ${JSON.stringify(res.body).slice(0, 500)}`,
        ),
        { code: FailureCode.SESSION_CREATE_FAILED },
      );
    }
    await waitAppReady(client, { timeoutMs: APP_READY_TIMEOUT_MS });
    await client
      .execute(`globalThis.__TRACER_E2E__ = true; return true;`)
      .catch(() => {});
  }

  async function relaunchApp() {
    try {
      await client.deleteSession({ timeoutMs: 30_000 });
    } catch {
      /* ignore */
    }
    await delay(1500);
    for (let i = 0; i < 20; i++) {
      const orphans = findOrphans(["tracer-desktop", "tracer_desktop", "grok"]);
      if (!orphans.length) break;
      reapOrphans(["tracer-desktop", "tracer_desktop", "grok"]);
      await delay(500);
    }
    await delay(500);
    const envFile = writeE2eEnvFile(appEnv);
    const res = await client.newSession(
      {
        application: binary,
        args: [`--tracer-e2e-env=${envFile}`],
        env: {
          ...appEnv,
          ...(process.env.TRACER_GROK_BIN
            ? { TRACER_GROK_BIN: process.env.TRACER_GROK_BIN }
            : {}),
          ...(process.env.GROK_HOME ? { GROK_HOME: process.env.GROK_HOME } : {}),
        },
      },
      { timeoutMs: WD_SESSION_TIMEOUT_MS },
    );
    if (!client.sessionId) {
      throw new Error(
        `relaunch session failed: HTTP ${res.statusCode} ${JSON.stringify(res.body).slice(0, 400)}`,
      );
    }
    await waitAppReady(client, { timeoutMs: APP_READY_TIMEOUT_MS });
    await client
      .execute(`globalThis.__TRACER_E2E__ = true; return true;`)
      .catch(() => {});
  }

  try {
    if (!env.os.supportedForL3I_externalDriver) {
      for (const j of journeysToRun) {
        journeyResults.push({
          id: j.id,
          name: j.name,
          result: LgjClass.BLOCKED_BY_TOOLING,
          message: `unsupported on ${env.os.platform}`,
        });
      }
      return finish(report.summary(), {
        workDir,
        artifactsDir,
        rid,
        journeyResults,
        resultOverride: LgjClass.BLOCKED_BY_TOOLING,
      });
    }

    await report.run(StageId.FRONTEND_BUILD, async () => {
      const r = ensureFrontend({ build: !skipBuild });
      return { status: "pass", message: `frontend dist ${r.path}`, detail: r };
    });

    await report.run(StageId.BACKEND_BUILD, async () => {
      binary = ensureBinary({ build: !skipBuild });
      return { status: "pass", message: `binary ${binary}`, detail: { binary } };
    });

    await report.run(StageId.PACKAGING, async () => {
      if (!binary || !existsSync(binary)) {
        return {
          status: "blocked_tooling",
          classification: ResultClass.BLOCKED_BY_TOOLING,
          message: "no test binary",
        };
      }
      return { status: "pass", message: "test binary ready" };
    });

    const hasDriver = Boolean(spawnPaths.tauriPath);
    const nativePath = spawnPaths.nativePath || process.env.TRACER_NATIVE_DRIVER || null;
    const hasNative =
      env.os.platform === "win32"
        ? Boolean(
            nativePath &&
              env.drivers.nativeDriver.msedgedriver.compatibility?.compatible,
          )
        : Boolean(env.drivers.nativeDriver.webkitWebDriver.available || nativePath);

    if (!hasDriver || !hasNative) {
      const msg = !hasDriver
        ? "tauri-driver not found"
        : "native WebDriver missing or incompatible";
      for (const j of journeysToRun) {
        journeyResults.push({
          id: j.id,
          name: j.name,
          result: LgjClass.BLOCKED_BY_TOOLING,
          message: msg,
        });
      }
      return finish(report.summary(), {
        workDir,
        artifactsDir,
        rid,
        journeyResults,
        resultOverride: LgjClass.BLOCKED_BY_TOOLING,
        failureCode: FailureCode.EDGE_DRIVER_NOT_FOUND,
      });
    }

    await report.run(StageId.DRIVER_STARTUP, async () => ({
      status: "pass",
      message: "driver start deferred to app launch bundle",
    }));

    await report.run(StageId.APP_LAUNCH, async () => {
      await startDriverAndApp();
      return {
        status: "pass",
        message: `session ${client.sessionId}`,
        detail: { sessionId: client.sessionId, binary, dbPath, liveBridge: LIVE_BRIDGE },
      };
    });

    await report.run(StageId.READINESS, async () => {
      const backend = await attrTestId(client, "tracer-app-root", "data-tracer-backend");
      return {
        status: "pass",
        message: `app ready backend=${backend}`,
        detail: { backend },
      };
    });

    await report.run(StageId.SMOKE, async () => {
      const ctx = {
        client,
        workDir,
        projectRoot,
        dbPath,
        artifactsDir,
        captureArtifact,
        relaunchApp,
        prompt: cli.prompt || DEFAULT_STREAM_PROMPT,
        approvalPrompt: DEFAULT_APPROVAL_PROMPT,
        cancelPrompt: DEFAULT_CANCEL_PROMPT,
      };
      for (const j of journeysToRun) {
        console.log(`\n--- ${j.id} ${j.name} ---`);
        const started = Date.now();
        let result;
        try {
          result = await j.run(ctx);
        } catch (e) {
          await captureArtifact(`${j.id}-throw`).catch(() => {});
          result = {
            id: j.id,
            name: j.name,
            result: LgjClass.FAIL,
            message: e instanceof Error ? e.message : String(e),
          };
        }
        result.durationMs = Date.now() - started;
        result.name = j.name;
        journeyResults.push(result);
        console.log(`[${result.result}] ${j.id}: ${result.message || ""}`);
        if (result.result === LgjClass.FAIL) {
          await captureArtifact(`${j.id}-fail`).catch(() => {});
        }
      }
      const overall = suiteResultFromLgj(journeyResults);
      return {
        status:
          overall === LgjClass.PASS
            ? "pass"
            : overall === LgjClass.PARTIAL ||
                overall === LgjClass.NOT_OBSERVED ||
                overall === LgjClass.UNSUPPORTED ||
                overall === LgjClass.BLOCKED_BY_AUTH
              ? "partial"
              : overall === LgjClass.BLOCKED_BY_TOOLING
                ? "blocked_tooling"
                : "fail",
        classification: overall,
        message: `journeys ${journeyResults.filter((r) => r.result === LgjClass.PASS).length}/${journeyResults.length} PASS`,
        detail: { journeys: journeyResults },
      };
    });

    await report.run(StageId.APP_SHUTDOWN, async () => {
      const res = await client.deleteSession({ timeoutMs: 30_000 });
      await delay(800);
      return { status: "pass", message: `session deleted HTTP ${res.statusCode}` };
    });

    await report.run(StageId.DRIVER_SHUTDOWN, async () => {
      if (driver) {
        const pid = driver.pid;
        await driver.stop();
        await delay(400);
        if (processAlive(pid)) {
          throw new Error(`tauri-driver pid ${pid} still alive`);
        }
        return { status: "pass", message: `driver stopped pid=${pid}` };
      }
      return { status: "pass", message: "no driver handle" };
    });

    await report.run(StageId.ORPHAN_VERIFY, async () => {
      await delay(600);
      let orphans = findOrphans(ORPHAN_CHECK_NAMES);
      if (orphans.length) {
        const reaped = reapOrphans(ORPHAN_CHECK_NAMES);
        orphans = findOrphans(ORPHAN_CHECK_NAMES);
        if (orphans.length) {
          const gj = journeyResults.find((j) => j.id === "LGJ-07");
          if (gj) {
            gj.result = LgjClass.FAIL;
            gj.message = `orphans remain: ${JSON.stringify(orphans)}`;
          }
          throw Object.assign(
            new Error(`orphans remain: ${JSON.stringify(orphans)}`),
            { code: FailureCode.ORPHAN_PROCESS },
          );
        }
        return {
          status: "partial",
          classification: ResultClass.PARTIAL,
          message: "orphans reaped",
          detail: { reaped },
        };
      }
      const gj07 = journeyResults.find((j) => j.id === "LGJ-07");
      if (gj07 && gj07.result === LgjClass.PASS) {
        gj07.message = "clean shutdown; no orphans after teardown";
        gj07.detail = { ...(gj07.detail || {}), orphans: [] };
      }
      return { status: "pass", message: "no orphans" };
    });

    try {
      if (process.env.TRACER_E2E_KEEP_TEMP !== "1") {
        const overall = suiteResultFromLgj(journeyResults);
        if (overall === LgjClass.PASS) {
          rmSync(workDir, { recursive: true, force: true });
        }
      }
    } catch {
      /* ignore */
    }

    return finish(report.summary(), {
      workDir,
      artifactsDir,
      rid,
      binary,
      journeyResults,
    });
  } catch (e) {
    console.error("[lgj] FAILED:", e instanceof Error ? e.message : e);
    try {
      await captureArtifact("harness-fail");
    } catch {
      /* ignore */
    }
    try {
      await client.deleteSession().catch(() => {});
      await stopAllOwned();
      reapOrphans(ORPHAN_CHECK_NAMES);
    } catch {
      /* ignore */
    }
    const summary = report.summary();
    return finish(
      summary,
      {
        workDir,
        artifactsDir,
        rid,
        journeyResults,
        error: e instanceof Error ? e.message : String(e),
        failureCode: e?.code || FailureCode.DRIVER_STARTUP_FAILED,
        resultOverride: journeyResults.length
          ? suiteResultFromLgj(journeyResults)
          : LgjClass.FAIL,
      },
      1,
    );
  }
}

function finish(summary, meta, exitHint) {
  const journeyOverall =
    meta.resultOverride ||
    (meta.journeyResults?.length
      ? suiteResultFromLgj(meta.journeyResults)
      : LgjClass.FAIL);

  const out = {
    schemaVersion: 1,
    module: "W2.3-B",
    level: "LGJ",
    harness: "tools/tauri-e2e/live",
    classificationTier: OPERATION_CLASS,
    result: journeyOverall,
    failureCode: meta.failureCode || null,
    stages: summary.stages,
    journeys: meta.journeyResults || [],
    meta: {
      runId: meta.rid,
      workDir: meta.workDir,
      artifactsDir: meta.artifactsDir,
      binary: meta.binary,
      error: meta.error ? sanitizeArtifactText(meta.error) : undefined,
      claimsLiveGui: true,
      ciClass: "manual_local",
      network: true,
      credentials: "operator_local_only_never_printed",
      liveGrok: true,
      liveGui: true,
      provider: "possible",
      fakeAcp: false,
      liveBridge: LIVE_BRIDGE,
      isolation:
        "NOT part of pnpm -r test or cargo test --workspace; explicit live opt-in only",
    },
  };

  const reportPath =
    cli.out ||
    path.join(REPO_ROOT, "artifacts", "tauri-e2e-live", meta.rid || "latest", "report.json");
  try {
    mkdirSync(path.dirname(reportPath), { recursive: true });
    writeFileSync(reportPath, sanitizeArtifactText(JSON.stringify(out, null, 2)), "utf8");
    console.log(`\nreport: ${reportPath}`);
  } catch (e) {
    console.warn("could not write report:", e instanceof Error ? e.message : e);
  }

  console.log("\n=== LGJ SUMMARY ===");
  console.log(`result: ${out.result}`);
  for (const j of out.journeys) {
    console.log(`  ${j.id} ${j.result}: ${j.message || ""}`);
  }

  if (jsonOut) {
    process.stdout.write(JSON.stringify(out, null, 2) + "\n");
  }

  const exitCode =
    exitHint ??
    (out.result === LgjClass.PASS
      ? 0
      : out.result === LgjClass.BLOCKED_BY_AUTH ||
          out.result === LgjClass.BLOCKED_BY_TOOLING ||
          out.result === LgjClass.PARTIAL ||
          out.result === LgjClass.NOT_OBSERVED ||
          out.result === LgjClass.UNSUPPORTED ||
          out.result === LgjClass.NOT_RUN
        ? 0 // honest non-pass classifications are not harness crashes
        : 1);
  process.exit(exitCode);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
