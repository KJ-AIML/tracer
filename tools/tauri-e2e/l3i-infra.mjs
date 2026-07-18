#!/usr/bin/env node
/**
 * L3-I — WebView driver infrastructure interaction (NOT product journey L3-J).
 *
 * Proves: build artifacts → msedgedriver path → tauri-driver → app via WebDriver
 *         session → root marker → minimal non-product property → clean shutdown
 *         → orphan verify.
 *
 * Does NOT: session create/prompt/approval product flow through the GUI.
 *
 * If driver stack missing → BLOCKED_BY_TOOLING (not false PASS, not FAIL).
 */

import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { ResultClass, StageId, Level, FailureCode } from "./lib/classify.mjs";
import {
  REPO_ROOT,
  FAKE_ACP_JS,
  discoverEnvironment,
  resolvePreferredBinary,
  resolveDriverSpawnPaths,
} from "./lib/discover.mjs";
import {
  spawnOwned,
  processAlive,
  findOrphans,
  reapOrphans,
  uniqueTempDir,
  installExitHooks,
  stopAllOwned,
} from "./lib/process.mjs";
import { createStageReport } from "./lib/stages.mjs";
import { WebDriverClient, waitDriverReady } from "./lib/webdriver.mjs";

const args = new Set(process.argv.slice(2));
const jsonOut = args.has("--json");
const port = Number(process.env.TRACER_TAURI_DRIVER_PORT || 4444);
const host = process.env.TRACER_TAURI_DRIVER_HOST || "127.0.0.1";
const baseUrl = `http://${host}:${port}`;

installExitHooks();

const ORPHAN_NAMES = [
  "tracer-desktop",
  "tracer_desktop",
  "tauri-driver",
  "msedgedriver",
  "WebKitWebDriver",
];

async function main() {
  const env = discoverEnvironment();
  const report = createStageReport();
  const workDir = uniqueTempDir("tracer-l3i");
  const logDir = path.join(workDir, "logs");
  mkdirSync(logDir, { recursive: true });
  const dbPath = path.join(workDir, "tracer-e2e.sqlite");

  /** @type {import('./lib/process.mjs').OwnedProcess | null} */
  let driver = null;
  const client = new WebDriverClient(baseUrl);
  const spawnPaths = resolveDriverSpawnPaths();

  console.log("=== L3-I WebView driver infrastructure (W2.2-T) ===");
  console.log(`workDir: ${workDir}`);
  console.log(`driver: ${baseUrl}`);
  console.log(
    `tauri-driver: ${spawnPaths.tauriPath || "MISSING"}`,
  );
  console.log(
    `native-driver: ${spawnPaths.nativePath || process.env.TRACER_NATIVE_DRIVER || "default/PATH"}`,
  );
  console.log("NOTE: L3-J full product GUI journey is NOT_STARTED — not claimed.");

  try {
    if (!env.os.supportedForL3I_externalDriver) {
      for (const id of Object.values(StageId)) {
        report.skip(
          id,
          `external tauri-driver unsupported on ${env.os.platform}`,
          ResultClass.UNSUPPORTED_PLATFORM,
        );
      }
      return finish(report.summary(), {
        workDir,
        resultOverride: ResultClass.UNSUPPORTED_PLATFORM,
        failureCode: FailureCode.UNSUPPORTED_PLATFORM,
      });
    }

    let binary =
      process.env.TRACER_E2E_APP_BINARY ||
      resolvePreferredBinary()?.path ||
      null;

    await report.run(StageId.FRONTEND_BUILD, async () => {
      const distOk = existsSync(
        path.join(REPO_ROOT, "apps/desktop/dist/index.html"),
      );
      if (!distOk) {
        return {
          status: "blocked_tooling",
          classification: ResultClass.BLOCKED_BY_TOOLING,
          message: "frontend dist missing — run L2 build first",
          detail: { code: FailureCode.FRONTEND_DIST_NOT_FOUND },
        };
      }
      return { status: "pass", message: "frontend dist present" };
    });

    await report.run(StageId.BACKEND_BUILD, async () => {
      if (!binary || !existsSync(binary)) {
        return {
          status: "blocked_tooling",
          classification: ResultClass.BLOCKED_BY_TOOLING,
          message:
            "app binary missing — run L2 / cargo build -p tracer-desktop first",
          detail: { code: FailureCode.APP_BINARY_NOT_FOUND },
        };
      }
      return { status: "pass", message: `binary ${binary}`, detail: { binary } };
    });

    await report.run(StageId.PACKAGING, async () => {
      if (!binary || !existsSync(binary)) {
        return {
          status: "blocked_tooling",
          classification: ResultClass.BLOCKED_BY_TOOLING,
          message: "no test binary",
          detail: { code: FailureCode.APP_BINARY_NOT_FOUND },
        };
      }
      return { status: "pass", message: "test binary ready" };
    });

    const hasDriver = Boolean(spawnPaths.tauriPath);
    const nativePath =
      spawnPaths.nativePath || process.env.TRACER_NATIVE_DRIVER || null;
    const hasNative =
      env.os.platform === "win32"
        ? Boolean(
            nativePath &&
              env.drivers.nativeDriver.msedgedriver.compatibility?.compatible,
          )
        : Boolean(
            env.drivers.nativeDriver.webkitWebDriver.available || nativePath,
          );

    if (!hasDriver || !hasNative) {
      let code = FailureCode.TAURI_DRIVER_NOT_FOUND;
      let msg = "tauri-driver not found (PATH/cargo bin/project bin)";
      if (hasDriver && env.os.platform === "win32") {
        code =
          env.drivers.nativeDriver.msedgedriver.compatibility?.code ||
          FailureCode.EDGE_DRIVER_NOT_FOUND;
        msg =
          env.drivers.nativeDriver.msedgedriver.compatibility?.message ||
          "msedgedriver missing or incompatible";
      } else if (hasDriver) {
        code = FailureCode.EDGE_DRIVER_NOT_FOUND;
        msg = "WebKitWebDriver not on PATH";
      }
      report.skip(StageId.DRIVER_STARTUP, msg, ResultClass.BLOCKED_BY_TOOLING);
      report.skip(
        StageId.APP_LAUNCH,
        "blocked: no driver",
        ResultClass.BLOCKED_BY_TOOLING,
      );
      report.skip(
        StageId.READINESS,
        "blocked: no driver",
        ResultClass.BLOCKED_BY_TOOLING,
      );
      report.skip(
        StageId.SMOKE,
        "blocked: no driver",
        ResultClass.BLOCKED_BY_TOOLING,
      );
      report.skip(
        StageId.APP_SHUTDOWN,
        "blocked: no driver",
        ResultClass.BLOCKED_BY_TOOLING,
      );
      report.skip(
        StageId.DRIVER_SHUTDOWN,
        "blocked: no driver",
        ResultClass.BLOCKED_BY_TOOLING,
      );
      await report.run(StageId.ORPHAN_VERIFY, async () => ({
        status: "pass",
        message: "no driver processes started",
      }));
      return finish(report.summary(), {
        workDir,
        resultOverride: ResultClass.BLOCKED_BY_TOOLING,
        setup: msg,
        failureCode: code,
      });
    }

    if (!binary || !existsSync(binary)) {
      report.skip(
        StageId.DRIVER_STARTUP,
        "binary missing — not starting driver",
        ResultClass.BLOCKED_BY_TOOLING,
      );
      return finish(report.summary(), {
        workDir,
        resultOverride: ResultClass.BLOCKED_BY_TOOLING,
        failureCode: FailureCode.APP_BINARY_NOT_FOUND,
      });
    }

    // 4) driver startup
    await report.run(StageId.DRIVER_STARTUP, async () => {
      const driverArgs = ["--port", String(port)];
      const native =
        process.env.TRACER_NATIVE_DRIVER || spawnPaths.nativePath || null;
      if (native) {
        driverArgs.push("--native-driver", native);
      }
      driver = spawnOwned(spawnPaths.tauriPath, driverArgs, {
        label: "tauri-driver",
        logDir,
        windowsHide: true,
      });
      if (!driver.pid) {
        throw Object.assign(new Error("failed to spawn tauri-driver"), {
          code: FailureCode.DRIVER_STARTUP_FAILED,
        });
      }
      const status = await waitDriverReady(baseUrl, { timeoutMs: 25_000 });
      return {
        status: "pass",
        message: `tauri-driver ready pid=${driver.pid}`,
        detail: {
          pid: driver.pid,
          statusCode: status.statusCode,
          body: status.body,
          nativeDriver: native,
        },
      };
    });

    // 5) app launch via WebDriver session
    await report.run(StageId.APP_LAUNCH, async () => {
      const appEnv = {
        TRACER_DATABASE_PATH: dbPath,
        TRACER_FAKE_ACP_JS: FAKE_ACP_JS,
        TRACER_HELI_PROBE_PATH: path.join(workDir, "heli-empty"),
        TRACER_NODE_BIN: process.env.TRACER_NODE_BIN || "node",
      };
      mkdirSync(appEnv.TRACER_HELI_PROBE_PATH, { recursive: true });
      const res = await client.newSession(
        {
          application: binary,
          env: appEnv,
        },
        { timeoutMs: 90_000 },
      );
      if (!client.sessionId) {
        return {
          status: "fail",
          classification: ResultClass.FAIL,
          message: `WebDriver new session failed: HTTP ${res.statusCode} ${JSON.stringify(res.body).slice(0, 500)}`,
          detail: { ...res, code: FailureCode.SESSION_CREATE_FAILED },
        };
      }
      return {
        status: "pass",
        message: `session ${client.sessionId}`,
        detail: { sessionId: client.sessionId, statusCode: res.statusCode },
      };
    });

    // 6) readiness — poll until title / readyState settle (WebView may boot after session create)
    let title = null;
    let rootOk = null;
    let tauriDetect = null;
    await report.run(StageId.READINESS, async () => {
      const deadline = Date.now() + 20_000;
      let lastReady = null;
      while (Date.now() < deadline) {
        try {
          const t = await client.getTitle();
          title = t.body?.value ?? t.body;
        } catch (e) {
          title = { error: e instanceof Error ? e.message : String(e) };
        }
        try {
          const ready = await client.execute(`return document.readyState;`);
          lastReady = ready.body?.value;
        } catch {
          lastReady = null;
        }
        if (title === "Tracer" && lastReady === "complete") break;
        await delay(400);
      }
      return {
        status: "pass",
        message: `WebDriver session responsive; title=${JSON.stringify(title)}; readyState=${JSON.stringify(lastReady)}`,
        detail: { title, readyState: lastReady },
      };
    });

    // 7) infrastructure smoke (NOT product journey)
    // Tauri 2: withGlobalTauri is off by default → window.__TAURI__ may be absent.
    // IPC surface is still present as __TAURI_INTERNALS__ (and product may expose __TAURI__).
    // L3-I accepts either as proof of WebView↔Tauri bridge; does not claim L3-J.
    await report.run(StageId.SMOKE, async () => {
      const checks = {
        driverSession: Boolean(client.sessionId),
        title: title,
        frontendRoot: null,
        tauriApiDetect: null,
        tauriSurface: null,
        pageSourceSnippet: null,
        nonProductProperty: null,
      };

      const probeScript = `
        var root = !!(document.getElementById('root') || document.body);
        var ready = document.readyState;
        var g = globalThis;
        var hasPublic = !!(g.__TAURI__ && g.__TAURI__.core && typeof g.__TAURI__.core.invoke === 'function');
        var internals = g.__TAURI_INTERNALS__;
        var hasInternals = !!(internals && (
          typeof internals.invoke === 'function' ||
          typeof internals.transformCallback === 'function' ||
          typeof internals.metadata === 'object'
        ));
        var keys = Object.keys(g).filter(function (k) {
          return k.indexOf('TAURI') !== -1 || k.indexOf('__TAURI') !== -1;
        });
        return {
          root: root,
          readyState: ready,
          hasPublicTauri: hasPublic,
          hasInternals: hasInternals,
          tauriKeys: keys,
          title: document.title || null
        };
      `;

      // Retry probes — IPC globals can appear slightly after first paint
      let probe = null;
      const smokeDeadline = Date.now() + 15_000;
      while (Date.now() < smokeDeadline) {
        try {
          const res = await client.execute(probeScript);
          probe = res.body?.value ?? res.body;
          if (probe && probe.root && (probe.hasPublicTauri || probe.hasInternals)) {
            break;
          }
          if (probe && probe.root && probe.readyState === "complete") {
            // one more short wait for IPC inject
            await delay(500);
            const res2 = await client.execute(probeScript);
            probe = res2.body?.value ?? res2.body;
            break;
          }
        } catch (e) {
          probe = { error: e instanceof Error ? e.message : String(e) };
        }
        await delay(400);
      }

      rootOk = probe?.root === true;
      checks.frontendRoot = rootOk;
      checks.nonProductProperty = probe?.readyState ?? null;
      checks.tauriSurface = {
        hasPublicTauri: Boolean(probe?.hasPublicTauri),
        hasInternals: Boolean(probe?.hasInternals),
        tauriKeys: probe?.tauriKeys ?? null,
      };
      tauriDetect = Boolean(probe?.hasPublicTauri || probe?.hasInternals);
      checks.tauriApiDetect = tauriDetect;

      // Optional app info via public API only (sync-safe detect; async invoke optional)
      if (probe?.hasPublicTauri) {
        try {
          // Prefer execute_async if needed later; sync probe just re-checks shape
          const info = await client.execute(
            `var inv = globalThis.__TAURI__ && globalThis.__TAURI__.core && globalThis.__TAURI__.core.invoke;
             if (!inv) return 'no-tauri';
             return 'public-invoke-present';`,
          );
          checks.appInfo = info.body?.value;
        } catch (e) {
          checks.appInfo = {
            error: e instanceof Error ? e.message : String(e),
          };
        }
      } else if (probe?.hasInternals) {
        checks.appInfo = "internals-present-no-public-global";
      } else {
        checks.appInfo = "no-tauri-surface";
      }

      const hardFail = !checks.driverSession;
      if (hardFail) throw new Error("no driver session");

      if (checks.frontendRoot !== true) {
        return {
          status: "fail",
          classification: ResultClass.FAIL,
          message: "root marker (#root|body) not observed",
          detail: { checks, probe, code: FailureCode.ROOT_MARKER_MISSING },
        };
      }

      if (!tauriDetect) {
        // Infrastructure incomplete: driver+root OK but no Tauri IPC surface observed
        return {
          status: "partial",
          classification: ResultClass.PARTIAL,
          message:
            "driver session + root OK; Tauri IPC surface (__TAURI__ / __TAURI_INTERNALS__) not observed (PARTIAL — still not L3-J)",
          detail: { checks, probe, claimsL3J: false },
        };
      }

      return {
        status: "pass",
        message:
          "driver + WebView infrastructure probes OK (L3-I only; not L3-J product journey)",
        detail: { checks, probe, claimsL3J: false },
      };
    });

    // 8) app shutdown via session delete
    await report.run(StageId.APP_SHUTDOWN, async () => {
      const res = await client.deleteSession({ timeoutMs: 30_000 });
      await delay(800);
      return {
        status: "pass",
        message: `session deleted HTTP ${res.statusCode}`,
        detail: { statusCode: res.statusCode },
      };
    });

    // 9) driver shutdown
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

    // 10) orphans
    await report.run(StageId.ORPHAN_VERIFY, async () => {
      await delay(500);
      let orphans = findOrphans(ORPHAN_NAMES);
      if (orphans.length) {
        const reaped = reapOrphans(ORPHAN_NAMES);
        orphans = findOrphans(ORPHAN_NAMES);
        if (orphans.length) {
          throw Object.assign(
            new Error(`orphans remain: ${JSON.stringify(orphans)}`),
            { code: FailureCode.ORPHAN_PROCESS },
          );
        }
        return {
          status: "partial",
          classification: ResultClass.PARTIAL,
          message: "orphans reaped",
          detail: { reaped, code: FailureCode.ORPHAN_PROCESS },
        };
      }
      return { status: "pass", message: "no orphans" };
    });

    return finish(report.summary(), { workDir, binary });
  } catch (e) {
    console.error("[l3i-infra] FAILED:", e instanceof Error ? e.message : e);
    try {
      await client.deleteSession().catch(() => {});
      await stopAllOwned();
      reapOrphans(ORPHAN_NAMES);
    } catch {
      /* ignore */
    }
    const summary = report.summary();
    if (summary.result === ResultClass.PASS) summary.result = ResultClass.FAIL;
    return finish(
      summary,
      {
        workDir,
        error: e instanceof Error ? e.message : String(e),
        failureCode: e?.code || FailureCode.DRIVER_STARTUP_FAILED,
      },
      1,
    );
  }
}

function finish(summary, meta, exitHint) {
  const out = {
    schemaVersion: 1,
    module: "W2.2-T",
    level: Level.L3I_WEBVIEW_INFRA,
    result: meta.resultOverride || summary.result,
    failureCode: meta.failureCode || null,
    stages: summary.stages,
    meta: {
      workDir: meta.workDir,
      binary: meta.binary,
      error: meta.error,
      setup: meta.setup,
      claimsL3J: false,
      ciClass: "windows_gui_runner | platform_gated_ci | manual_local",
      network: false,
      credentials: false,
      liveGrok: false,
      isolation:
        "NOT part of pnpm -r test or cargo test --workspace; explicit pnpm test:tauri-e2e:l3i only",
    },
  };
  if (jsonOut) {
    console.log(JSON.stringify(out, null, 2));
  } else {
    console.log("");
    console.log(`L3-I result: ${out.result}`);
    for (const s of out.stages) {
      console.log(
        `  [${s.status}] ${s.id}${s.message ? " — " + s.message : ""}`,
      );
    }
    if (meta.setup) console.log(`setup: ${meta.setup}`);
    if (meta.failureCode) console.log(`failureCode: ${meta.failureCode}`);
  }
  try {
    writeFileSync(
      path.join(meta.workDir, "l3i-report.json"),
      JSON.stringify(out, null, 2),
      "utf8",
    );
  } catch {
    /* ignore */
  }
  if (exitHint != null) process.exitCode = exitHint;
  else process.exitCode = out.result === ResultClass.FAIL ? 1 : 0;
  return out;
}

main();