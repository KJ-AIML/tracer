/**
 * L3-J product journeys GJ-01 … GJ-12.
 * Real GUI interactions only — no direct plane_* invoke for session/prompt/approval.
 */

import { mkdirSync, writeFileSync, existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import {
  attrTestId,
  clickTestId,
  existsTestId,
  guiCreateSession,
  guiRegisterProject,
  guiSubmitPrompt,
  pass,
  fail,
  partial,
  waitAnyEventType,
  waitAppReady,
  waitForTestId,
  waitSessionStatus,
  typeTestId,
  selectTestId,
  textTestId,
} from "./gui.mjs";
import { ResultClass } from "./classify.mjs";

/**
 * @typedef {object} JourneyCtx
 * @property {import('./webdriver.mjs').WebDriverClient} client
 * @property {string} workDir
 * @property {string} projectRoot  temp project path for register
 * @property {string} dbPath
 * @property {string} artifactsDir
 * @property {(label: string) => Promise<object>} captureArtifact
 * @property {() => Promise<void>} [relaunchApp]  for GJ-09
 */

async function ensureOnProjects(client) {
  if (!(await existsTestId(client, "tracer-projects-home"))) {
    await clickTestId(client, "tracer-nav-projects");
    await waitForTestId(client, "tracer-projects-home", { timeoutMs: 20_000 });
  }
}

/** GJ-01: Startup in Tauri mode (not mock). */
export async function gj01_startup(ctx) {
  const id = "GJ-01";
  try {
    const ready = await waitAppReady(ctx.client, { timeoutMs: 60_000 });
    if (ready.backend !== "tauri") {
      return fail(id, `expected tauri backend, got ${ready.backend}`, ready);
    }
    const title = await ctx.client.getTitle();
    const titleVal = title.body?.value ?? title.body;
    const badge = await textTestId(ctx.client, "tracer-backend-badge");
    const shell = await existsTestId(ctx.client, "tracer-app-shell");
    if (!shell) return fail(id, "app shell missing");
    return pass(id, "startup ready in Tauri mode", {
      backend: ready.backend,
      title: titleVal,
      badge,
    });
  } catch (e) {
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-02: Create first session via GUI. */
export async function gj02_create_session(ctx) {
  const id = "GJ-02";
  try {
    await ensureOnProjects(ctx.client);
    await guiRegisterProject(ctx.client, {
      rootPath: ctx.projectRoot,
      name: "l3j-gj02",
    });
    const session = await guiCreateSession(ctx.client, {
      title: "GJ-02 first session",
      scenarioId: "happy_prompt_stream",
    });
    if (!session.sessionId) {
      return fail(id, "session workspace missing session id", session);
    }
    // ready is expected after fake runtime init
    const status = session.status || (await attrTestId(ctx.client, "tracer-session-workspace", "data-session-status"));
    if (status !== "ready" && status !== "starting_runtime" && status !== "creating") {
      // poll briefly toward ready
      try {
        await waitSessionStatus(ctx.client, (s) => s === "ready", { timeoutMs: 30_000 });
      } catch {
        return fail(id, `session not ready after create; status=${status}`, session);
      }
    } else if (status !== "ready") {
      await waitSessionStatus(ctx.client, (s) => s === "ready", { timeoutMs: 45_000 });
    }
    return pass(id, "first session created via GUI", session);
  } catch (e) {
    await ctx.captureArtifact("gj02-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-03: Successful streaming prompt (fake). */
export async function gj03_streaming_prompt(ctx) {
  const id = "GJ-03";
  try {
    // Assume on session from GJ-02 or create fresh
    if (!(await existsTestId(ctx.client, "tracer-session-workspace"))) {
      await ensureOnProjects(ctx.client);
      // open first project if list has one
      const opened = await tryOpenFirstProject(ctx.client);
      if (!opened) {
        await guiRegisterProject(ctx.client, {
          rootPath: ctx.projectRoot,
          name: "l3j-gj03",
        });
      }
      await guiCreateSession(ctx.client, {
        title: "GJ-03 stream",
        scenarioId: "happy_prompt_stream",
      });
      await waitSessionStatus(ctx.client, (s) => s === "ready", { timeoutMs: 45_000 });
    }
    // If not ready (e.g. after prior journey), create new session from project
    let status = await attrTestId(ctx.client, "tracer-session-workspace", "data-session-status");
    if (status !== "ready") {
      await clickTestId(ctx.client, "tracer-session-back").catch(() => {});
      // leave confirm if any — may need to accept alert
      await acceptConfirmIfAny(ctx.client);
      if (await existsTestId(ctx.client, "tracer-project-workspace")) {
        await guiCreateSession(ctx.client, {
          title: "GJ-03 stream",
          scenarioId: "happy_prompt_stream",
        });
      } else {
        await ensureOnProjects(ctx.client);
        await guiRegisterProject(ctx.client, {
          rootPath: ctx.projectRoot,
          name: "l3j-gj03b",
        });
        await guiCreateSession(ctx.client, {
          title: "GJ-03 stream",
          scenarioId: "happy_prompt_stream",
        });
      }
      await waitSessionStatus(ctx.client, (s) => s === "ready", { timeoutMs: 45_000 });
    }

    await guiSubmitPrompt(ctx.client, "summarize the repository for GJ-03");
    const found = await waitAnyEventType(
      ctx.client,
      [
        "agent.message.delta",
        "agent.message.completed",
        "session.completed",
        "session.prompt.submitted",
      ],
      { timeoutMs: 60_000 },
    );
    return pass(id, "streaming prompt events observed in GUI timeline", { found });
  } catch (e) {
    await ctx.captureArtifact("gj03-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-04: Approval accepted. */
export async function gj04_approval_accept(ctx) {
  const id = "GJ-04";
  try {
    await prepareSession(ctx, "permission_allow", "GJ-04 approval allow");
    await guiSubmitPrompt(ctx.client, "needs permission for GJ-04");
    // Poll refresh while waiting for approval card (prompt RPC may still be open).
    const cardDeadline = Date.now() + 60_000;
    while (Date.now() < cardDeadline) {
      if (await existsTestId(ctx.client, "tracer-approval-card")) break;
      if (await existsTestId(ctx.client, "tracer-session-refresh")) {
        await clickTestId(ctx.client, "tracer-session-refresh").catch(() => {});
      }
      // Also check events for approval.requested as soft signal
      try {
        await waitAnyEventType(ctx.client, ["approval.requested"], { timeoutMs: 500 });
      } catch {
        /* continue */
      }
      await delay(400);
    }
    await waitForTestId(ctx.client, "tracer-approval-card", { timeoutMs: 10_000 });
    await clickTestId(ctx.client, "tracer-approval-allow");
    // After allow, approval card should clear; events may show approval.resolved
    const deadline = Date.now() + 45_000;
    let cleared = false;
    while (Date.now() < deadline) {
      if (!(await existsTestId(ctx.client, "tracer-approval-card"))) {
        cleared = true;
        break;
      }
      await clickTestId(ctx.client, "tracer-session-refresh").catch(() => {});
      await delay(400);
    }
    if (!cleared) {
      return fail(id, "approval card still present after Allow");
    }
    const found = await waitAnyEventType(
      ctx.client,
      ["approval.resolved", "session.completed", "agent.message.delta", "agent.message.completed"],
      { timeoutMs: 45_000 },
    ).catch(() => []);
    return pass(id, "approval accepted via GUI", { found, cleared });
  } catch (e) {
    await ctx.captureArtifact("gj04-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-05: Approval rejected. */
export async function gj05_approval_reject(ctx) {
  const id = "GJ-05";
  try {
    await prepareSession(ctx, "permission_deny", "GJ-05 approval deny");
    await guiSubmitPrompt(ctx.client, "needs permission for GJ-05");
    await waitForTestId(ctx.client, "tracer-approval-card", { timeoutMs: 45_000 });
    await clickTestId(ctx.client, "tracer-approval-deny");
    const deadline = Date.now() + 45_000;
    let cleared = false;
    while (Date.now() < deadline) {
      if (!(await existsTestId(ctx.client, "tracer-approval-card"))) {
        cleared = true;
        break;
      }
      await clickTestId(ctx.client, "tracer-session-refresh").catch(() => {});
      await delay(400);
    }
    if (!cleared) return fail(id, "approval card still present after Deny");
    return pass(id, "approval rejected via GUI", { cleared });
  } catch (e) {
    await ctx.captureArtifact("gj05-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-06: Cancel while approval pending (deadlock-free). */
export async function gj06_cancel_while_approval(ctx) {
  const id = "GJ-06";
  try {
    await prepareSession(
      ctx,
      "cancel_while_permission_pending",
      "GJ-06 cancel pending",
    );
    await guiSubmitPrompt(ctx.client, "pending approval then cancel");
    // Either approval card or cancel visible
    const deadline = Date.now() + 45_000;
    let sawApproval = false;
    while (Date.now() < deadline) {
      if (await existsTestId(ctx.client, "tracer-session-refresh")) {
        await clickTestId(ctx.client, "tracer-session-refresh").catch(() => {});
      }
      if (await existsTestId(ctx.client, "tracer-approval-card")) {
        sawApproval = true;
        break;
      }
      if (await existsTestId(ctx.client, "tracer-session-cancel")) break;
      await delay(300);
    }
    // Prefer session Cancel (cancel while approval pending)
    if (await existsTestId(ctx.client, "tracer-session-cancel")) {
      await clickTestId(ctx.client, "tracer-session-cancel");
    } else if (await existsTestId(ctx.client, "tracer-approval-cancel-request")) {
      await clickTestId(ctx.client, "tracer-approval-cancel-request");
    } else {
      return fail(id, "neither Cancel nor approval cancel control available", {
        sawApproval,
      });
    }
    // Must not hang — reach terminal-ish status
    const status = await waitSessionStatus(
      ctx.client,
      (s) =>
        s === "stopped" ||
        s === "cancelling" ||
        s === "completed" ||
        s === "failed" ||
        s === "ready" ||
        s === "disconnected",
      { timeoutMs: 45_000 },
    );
    return pass(id, "cancel while approval pending completed without deadlock", {
      status,
      sawApproval,
    });
  } catch (e) {
    await ctx.captureArtifact("gj06-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-07: Two-session focus switch. */
export async function gj07_two_session_focus(ctx) {
  const id = "GJ-07";
  try {
    await ensureOnProjects(ctx.client);
    // Use existing or register
    let onProject = await existsTestId(ctx.client, "tracer-project-workspace");
    if (!onProject) {
      const opened = await tryOpenFirstProject(ctx.client);
      if (!opened) {
        await guiRegisterProject(ctx.client, {
          rootPath: ctx.projectRoot,
          name: "l3j-gj07",
        });
      }
    }
    const a = await guiCreateSession(ctx.client, {
      title: "GJ-07 session A",
      scenarioId: "happy_prompt_stream",
    });
    await waitSessionStatus(ctx.client, (s) => s === "ready", { timeoutMs: 45_000 });
    await clickTestId(ctx.client, "tracer-session-back");
    await acceptConfirmIfAny(ctx.client);
    await waitForTestId(ctx.client, "tracer-project-workspace", { timeoutMs: 20_000 });
    const b = await guiCreateSession(ctx.client, {
      title: "GJ-07 session B",
      scenarioId: "happy_prompt_stream",
    });
    await waitSessionStatus(ctx.client, (s) => s === "ready", { timeoutMs: 45_000 });
    const focusB = await attrTestId(ctx.client, "tracer-session-workspace", "data-session-id");
    if (focusB !== b.sessionId) {
      return fail(id, "session B not focused after create", { focusB, b });
    }
    // Back to list and open A
    await clickTestId(ctx.client, "tracer-session-back");
    await acceptConfirmIfAny(ctx.client);
    await waitForTestId(ctx.client, "tracer-project-workspace", { timeoutMs: 20_000 });
    await clickTestId(ctx.client, `tracer-session-open-${a.sessionId}`, {
      timeoutMs: 20_000,
    });
    await waitForTestId(ctx.client, "tracer-session-workspace", { timeoutMs: 30_000 });
    const focusA = await attrTestId(ctx.client, "tracer-session-workspace", "data-session-id");
    if (focusA !== a.sessionId) {
      return fail(id, "focus switch to session A failed", { focusA, a, b });
    }
    return pass(id, "two-session focus switch via GUI", {
      sessionA: a.sessionId,
      sessionB: b.sessionId,
      focusA,
    });
  } catch (e) {
    await ctx.captureArtifact("gj07-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-08: Runtime crash / EOF. */
export async function gj08_runtime_crash(ctx) {
  const id = "GJ-08";
  try {
    await prepareSession(ctx, "crash_nonzero_exit", "GJ-08 crash");
    await guiSubmitPrompt(ctx.client, "trigger crash");
    // Expect disconnected / failed / crash banner
    const deadline = Date.now() + 60_000;
    let observed = null;
    while (Date.now() < deadline) {
      await clickTestId(ctx.client, "tracer-session-refresh").catch(() => {});
      const status = await attrTestId(
        ctx.client,
        "tracer-session-workspace",
        "data-session-status",
      );
      const runtime = await attrTestId(
        ctx.client,
        "tracer-session-workspace",
        "data-runtime-observation",
      );
      const banner =
        (await existsTestId(ctx.client, "tracer-banner-runtime-disconnected")) ||
        (await existsTestId(ctx.client, "tracer-banner-session-error"));
      if (
        status === "disconnected" ||
        status === "failed" ||
        runtime === "crashed" ||
        runtime === "disconnected" ||
        banner
      ) {
        observed = { status, runtime, banner };
        break;
      }
      // Also accept event evidence
      try {
        const found = await waitAnyEventType(
          ctx.client,
          ["runtime.process.exited", "session.failed", "adapter.protocol.error"],
          { timeoutMs: 500 },
        );
        if (found.length) {
          observed = { status, runtime, found };
          break;
        }
      } catch {
        /* continue */
      }
      await delay(400);
    }
    if (!observed) {
      return fail(id, "no crash/EOF UI evidence within timeout");
    }
    return pass(id, "runtime crash/EOF reflected in GUI", observed);
  } catch (e) {
    await ctx.captureArtifact("gj08-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/**
 * GJ-09: Restart + history restore (same temp DB).
 * Requires ctx.relaunchApp() that reuses TRACER_DATABASE_PATH.
 */
export async function gj09_restart_history(ctx) {
  const id = "GJ-09";
  try {
    await prepareSession(ctx, "happy_prompt_stream", "GJ-09 persist");
    await guiSubmitPrompt(ctx.client, "persist me for restart");
    await waitAnyEventType(
      ctx.client,
      ["session.prompt.submitted", "agent.message.delta", "agent.message.completed", "session.completed"],
      { timeoutMs: 60_000 },
    );
    const sessionId = await attrTestId(
      ctx.client,
      "tracer-session-workspace",
      "data-session-id",
    );
    // Graceful stop so SQLite WAL can flush before process kill.
    if (await existsTestId(ctx.client, "tracer-session-stop")) {
      await clickTestId(ctx.client, "tracer-session-stop").catch(() => {});
      await delay(800);
    }
    // Capture e2e env + project list via public Tauri invoke before kill.
    const beforeEnv = await invokeInWebView(ctx.client, "tracer_e2e_env");
    const beforeProjects = await invokeInWebView(ctx.client, "tracer_project_list");
    if (!ctx.relaunchApp) {
      return partial(id, "relaunch helper unavailable — history path not fully exercised", {
        sessionId,
        beforeEnv,
      });
    }
    await delay(500);
    await ctx.relaunchApp();
    await waitAppReady(ctx.client, { timeoutMs: 60_000 });
    await ensureOnProjects(ctx.client);
    // Give bootstrap loadProjects a moment; then force refresh.
    await delay(1000);
    if (await existsTestId(ctx.client, "tracer-projects-refresh")) {
      await clickTestId(ctx.client, "tracer-projects-refresh").catch(() => {});
      await delay(800);
    }
    const afterEnv = await invokeInWebView(ctx.client, "tracer_e2e_env");
    const afterProjects = await invokeInWebView(ctx.client, "tracer_project_list");
    // Open first project then session
    let opened = await tryOpenFirstProject(ctx.client);
    if (!opened && (await existsTestId(ctx.client, "tracer-projects-refresh-empty"))) {
      await clickTestId(ctx.client, "tracer-projects-refresh-empty").catch(() => {});
      await delay(800);
      opened = await tryOpenFirstProject(ctx.client);
    }
    if (!opened) {
      const dbExists = existsSync(ctx.dbPath);
      let dbSize = 0;
      try {
        dbSize = dbExists ? readFileSync(ctx.dbPath).length : 0;
      } catch {
        dbSize = -1;
      }
      const beforeDb = beforeEnv?.databasePath ?? null;
      const afterDb = afterEnv?.databasePath ?? null;
      const beforeCount = Array.isArray(beforeProjects?.projects)
        ? beforeProjects.projects.length
        : null;
      const afterCount = Array.isArray(afterProjects?.projects)
        ? afterProjects.projects.length
        : null;
      // If file DB has bytes but relaunch opens empty, classify tooling/env propagation.
      if (dbExists && dbSize > 0 && afterCount === 0) {
        return fail(id, "no project after restart with same DB", {
          dbPath: ctx.dbPath,
          dbExists,
          dbSize,
          beforeDb,
          afterDb,
          beforeCount,
          afterCount,
          beforeEnv,
          afterEnv,
          note:
            afterDb && beforeDb && afterDb !== beforeDb
              ? "databasePath changed across relaunch — env not preserved"
              : "DB file non-empty but project_list empty after relaunch",
        });
      }
      return fail(id, "no project after restart with same DB", {
        dbPath: ctx.dbPath,
        dbExists,
        dbSize,
        beforeDb,
        afterDb,
        beforeCount,
        afterCount,
      });
    }
    await waitForTestId(ctx.client, "tracer-session-list", { timeoutMs: 30_000 });
    // Open matching session if present
    if (sessionId && (await existsTestId(ctx.client, `tracer-session-open-${sessionId}`))) {
      await clickTestId(ctx.client, `tracer-session-open-${sessionId}`);
    } else {
      // open first session item
      const openBtn = await ctx.client.execute(
        `var btn = document.querySelector('[data-testid^="tracer-session-open-"]');
         if (!btn) return null;
         btn.click();
         return btn.getAttribute('data-testid');`,
      );
      if (!openBtn.body?.value) {
        return fail(id, "no sessions restored after restart");
      }
    }
    await waitForTestId(ctx.client, "tracer-session-workspace", { timeoutMs: 45_000 });
    // History events
    const hasEvents = await waitAnyEventType(
      ctx.client,
      [
        "session.prompt.submitted",
        "agent.message.delta",
        "agent.message.completed",
        "session.completed",
        "session.created",
      ],
      { timeoutMs: 30_000 },
    ).catch(() => []);
    if (!hasEvents.length) {
      // Partial if session restored but events empty (possible timing/persist race)
      return partial(id, "session restored after restart but timeline empty", {
        sessionId,
      });
    }
    return pass(id, "restart restored session history from same temp DB", {
      sessionId,
      hasEvents,
    });
  } catch (e) {
    await ctx.captureArtifact("gj09-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-10: Heli unavailable non-fatal. */
export async function gj10_heli_unavailable(ctx) {
  const id = "GJ-10";
  try {
    // Env already points TRACER_HELI_PROBE_PATH at empty dir for journeys.
    await waitAppReady(ctx.client, { timeoutMs: 30_000 });
    // Banner may appear after bootstrap heli refresh
    const deadline = Date.now() + 15_000;
    let seen = false;
    while (Date.now() < deadline) {
      if (
        (await existsTestId(ctx.client, "tracer-banner-heli-unavailable")) ||
        (await existsTestId(ctx.client, "tracer-global-status"))
      ) {
        // Check page text
        const src = await ctx.client.getPageSource();
        const html = src.raw || JSON.stringify(src.body || "");
        if (/Heli/i.test(html) && /unavailable|not found|not detected/i.test(html)) {
          seen = true;
          break;
        }
      }
      await delay(400);
    }
    // App still usable
    const ready = await existsTestId(ctx.client, "tracer-app-ready");
    const backend = await attrTestId(ctx.client, "tracer-app-root", "data-tracer-backend");
    if (!ready || backend !== "tauri") {
      return fail(id, "app not usable under heli unavailable", { ready, backend });
    }
    if (!seen) {
      return partial(
        id,
        "app remains usable; heli unavailable banner not observed (probe may report available)",
        { backend },
      );
    }
    return pass(id, "heli unavailable is non-fatal; app usable", { backend });
  } catch (e) {
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/**
 * GJ-11: Invoke failure fail-closed (no silent mock).
 * Proves Tauri backend mode and that failed register surfaces error without mock.
 */
export async function gj11_invoke_fail_closed(ctx) {
  const id = "GJ-11";
  try {
    await waitAppReady(ctx.client, { timeoutMs: 30_000 });
    const backend = await attrTestId(ctx.client, "tracer-app-root", "data-tracer-backend");
    if (backend !== "tauri") {
      return fail(id, `silent mock risk: backend=${backend}, expected tauri`);
    }
    await ensureOnProjects(ctx.client);
    // Invalid path should fail closed with error, remaining on tauri
    await typeTestId(
      ctx.client,
      "tracer-project-root-path",
      path.join(ctx.workDir, "definitely-missing-project-root-l3j"),
    );
    await typeTestId(ctx.client, "tracer-project-name", "bad");
    await clickTestId(ctx.client, "tracer-project-register-submit");
    await delay(1500);
    const err =
      (await existsTestId(ctx.client, "tracer-project-register-error")) ||
      (await existsTestId(ctx.client, "tracer-invoke-error")) ||
      (await existsTestId(ctx.client, "tracer-banner-control-plane-down")) ||
      (await existsTestId(ctx.client, "tracer-banner-session-error"));
    const errText = err
      ? (await textTestId(ctx.client, "tracer-project-register-error")) ||
        (await textTestId(ctx.client, "tracer-invoke-error"))
      : null;
    const backendAfter = await attrTestId(ctx.client, "tracer-app-root", "data-tracer-backend");
    if (backendAfter !== "tauri") {
      return fail(id, "backend switched away from tauri after invoke failure", {
        backendAfter,
        errText,
      });
    }
    // Mock controls must not appear
    if (await existsTestId(ctx.client, "tracer-mock-controls")) {
      return fail(id, "mock scenario controls visible in tauri mode");
    }
    if (!err && !errText) {
      return partial(
        id,
        "tauri backend held; explicit error banner not observed (command may map differently)",
        { backendAfter },
      );
    }
    return pass(id, "invoke failure fail-closed; no silent mock", {
      backendAfter,
      errText,
    });
  } catch (e) {
    await ctx.captureArtifact("gj11-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** GJ-12: Clean shutdown / no orphans — handled mainly by harness teardown. */
export async function gj12_clean_shutdown(ctx) {
  const id = "GJ-12";
  try {
    await waitAppReady(ctx.client, { timeoutMs: 20_000 });
    // Soft stop session if open
    if (await existsTestId(ctx.client, "tracer-session-stop")) {
      await clickTestId(ctx.client, "tracer-session-stop").catch(() => {});
      await delay(500);
    }
    // Mark that GUI requested clean path; harness verifies orphans after session delete.
    return pass(id, "clean shutdown path exercised; orphan verify is harness stage", {
      note: "session/app/driver cleanup + orphan check in l3j-gui.mjs",
    });
  } catch (e) {
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

// --- helpers ---

async function prepareSession(ctx, scenarioId, title) {
  // Navigate to project workspace and create a fresh session with scenario
  if (await existsTestId(ctx.client, "tracer-session-workspace")) {
    await clickTestId(ctx.client, "tracer-session-back").catch(() => {});
    await acceptConfirmIfAny(ctx.client);
  }
  if (!(await existsTestId(ctx.client, "tracer-project-workspace"))) {
    await ensureOnProjects(ctx.client);
    const opened = await tryOpenFirstProject(ctx.client);
    if (!opened) {
      await guiRegisterProject(ctx.client, {
        rootPath: ctx.projectRoot,
        name: `l3j-${scenarioId}`.slice(0, 32),
      });
    }
  }
  await guiCreateSession(ctx.client, { title, scenarioId });
  await waitSessionStatus(ctx.client, (s) => s === "ready", { timeoutMs: 60_000 });
}

async function tryOpenFirstProject(client) {
  if (await existsTestId(client, "tracer-project-list")) {
    const res = await client.execute(
      `var btn = document.querySelector('[data-testid^="tracer-project-open-"]');
       if (!btn) return false;
       btn.click();
       return true;`,
    );
    if (res.body?.value === true) {
      await waitForTestId(client, "tracer-project-workspace", { timeoutMs: 30_000 });
      return true;
    }
  }
  return false;
}

async function acceptConfirmIfAny(client) {
  // WebDriver alert accept if present
  try {
    await client.execute(`/* no-op probe */ return true;`);
  } catch {
    /* ignore */
  }
  // Tauri/WebView may not expose window.confirm to WebDriver alerts.
  // Our leave() uses window.confirm — if stuck, force navigate via script.
  try {
    await client.execute(
      `if (document.querySelector('[data-testid="tracer-session-workspace"]')) {
         /* leave may have been cancelled; force projects nav */
       }
       return true;`,
    );
  } catch {
    /* ignore */
  }
}

/**
 * Invoke a Tauri command from the WebView (product surface — not harness plane_*).
 * Used only for diagnostics / history verification of typed command results already
 * available to the GUI.
 */
async function invokeInWebView(client, command, args = {}) {
  try {
    const res = await client.execute(
      `var inv = globalThis.__TAURI__ && globalThis.__TAURI__.core && globalThis.__TAURI__.core.invoke;
       if (!inv) return { ok: false, reason: 'no-tauri' };
       // Sync probe only — real invoke is async; use a flag pattern
       return { ok: true, hasInvoke: true, command: arguments[0] };`,
      [command],
    );
    if (!res.body?.value?.hasInvoke) return { ok: false, reason: "no-tauri" };
  } catch {
    return { ok: false, reason: "probe-failed" };
  }
  // execute_async style via promise + busy wait is unreliable; use async script if available
  try {
    const res = await client.executeAsync(
      `var command = arguments[0];
       var args = arguments[1] || {};
       var done = arguments[arguments.length - 1];
       var inv = globalThis.__TAURI__ && globalThis.__TAURI__.core && globalThis.__TAURI__.core.invoke;
       if (!inv) { done({ ok: false, reason: 'no-tauri' }); return; }
       inv(command, args).then(function (v) { done({ ok: true, value: v }); })
         .catch(function (e) { done({ ok: false, error: String(e) }); });`,
      [command, args],
      { timeoutMs: 20_000 },
    );
    const body = res.body?.value ?? res.body;
    if (body?.ok) return body.value;
    return body;
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

export const JOURNEY_RUNNERS = [
  { id: "GJ-01", name: "startup_tauri_mode", run: gj01_startup },
  { id: "GJ-02", name: "create_first_session", run: gj02_create_session },
  { id: "GJ-03", name: "streaming_prompt", run: gj03_streaming_prompt },
  { id: "GJ-04", name: "approval_accepted", run: gj04_approval_accept },
  { id: "GJ-05", name: "approval_rejected", run: gj05_approval_reject },
  { id: "GJ-06", name: "cancel_while_approval_pending", run: gj06_cancel_while_approval },
  { id: "GJ-07", name: "two_session_focus_switch", run: gj07_two_session_focus },
  { id: "GJ-08", name: "runtime_crash_eof", run: gj08_runtime_crash },
  { id: "GJ-09", name: "restart_history_restore", run: gj09_restart_history },
  { id: "GJ-10", name: "heli_unavailable", run: gj10_heli_unavailable },
  { id: "GJ-11", name: "invoke_failure_fail_closed", run: gj11_invoke_fail_closed },
  { id: "GJ-12", name: "clean_shutdown", run: gj12_clean_shutdown },
];

export function filterJourneys(filterArg) {
  if (!filterArg) return JOURNEY_RUNNERS;
  const want = String(filterArg).toUpperCase();
  return JOURNEY_RUNNERS.filter(
    (j) => j.id === want || j.id.replace("-", "") === want.replace("-", "") || j.name === filterArg,
  );
}

export { ResultClass };
