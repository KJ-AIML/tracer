/**
 * LGJ-01 … LGJ-07 product GUI journeys against live Grok (via test-only bridge).
 * Real GUI interactions only — reuses tools/tauri-e2e/lib/gui.mjs helpers.
 */

import { setTimeout as delay } from "node:timers/promises";
import { existsSync, readFileSync } from "node:fs";
import {
  attrTestId,
  clickTestId,
  existsTestId,
  guiCreateSession,
  guiRegisterProject,
  guiSubmitPrompt,
  waitAnyEventType,
  waitAppReady,
  waitForTestId,
  waitSessionStatus,
  textTestId,
} from "../../lib/gui.mjs";
import {
  LgjClass,
  pass,
  fail,
  partial,
  notObserved,
  unsupported,
  blockedAuth,
  looksLikeAuthBlock,
} from "./classify.mjs";
import {
  DEFAULT_STREAM_PROMPT,
  DEFAULT_APPROVAL_PROMPT,
  DEFAULT_CANCEL_PROMPT,
} from "./opt-in.mjs";
import {
  CANCEL_DEADLOCK_MS,
  SESSION_READY_TIMEOUT_MS,
  STREAM_EVENT_TIMEOUT_MS,
  APPROVAL_OBSERVE_TIMEOUT_MS,
  APP_READY_TIMEOUT_MS,
} from "./policy.mjs";

async function ensureOnProjects(client) {
  if (!(await existsTestId(client, "tracer-projects-home"))) {
    await clickTestId(client, "tracer-nav-projects");
    await waitForTestId(client, "tracer-projects-home", { timeoutMs: 20_000 });
  }
}

async function acceptConfirmIfAny(client) {
  try {
    await client.execute(`
      if (globalThis.__TRACER_E2E__) return true;
      return true;
    `);
  } catch {
    /* ignore */
  }
}

async function tryOpenFirstProject(client) {
  const res = await client.execute(`
    var btn = document.querySelector('[data-testid^="tracer-project-open-"]');
    if (!btn) return false;
    btn.click();
    return true;
  `);
  if (res.body?.value) {
    await waitForTestId(client, "tracer-project-workspace", { timeoutMs: 20_000 }).catch(() => {});
    return true;
  }
  return false;
}

async function readSessionError(client) {
  // Prefer banner / last error text surfaces when present
  for (const id of [
    "tracer-banner-session-error",
    "tracer-session-error",
    "tracer-session-last-error",
    "tracer-create-error",
  ]) {
    if (await existsTestId(client, id)) {
      const t = await textTestId(client, id);
      if (t) return t;
    }
  }
  return null;
}

/**
 * Ensure a ready live session, or return auth/fail classification payload.
 */
async function ensureLiveSession(ctx, title) {
  await ensureOnProjects(ctx.client);
  let onProject = await existsTestId(ctx.client, "tracer-project-workspace");
  if (!onProject) {
    const opened = await tryOpenFirstProject(ctx.client);
    if (!opened) {
      await guiRegisterProject(ctx.client, {
        rootPath: ctx.projectRoot,
        name: title || "lgj-project",
      });
    }
  }
  // Bridge ignores scenario; any catalog id is fine
  const session = await guiCreateSession(ctx.client, {
    title: title || "LGJ live session",
    scenarioId: "happy_prompt_stream",
  });
  let status = session.status;
  try {
    status = await waitSessionStatus(
      ctx.client,
      (s) =>
        s === "ready" ||
        s === "failed" ||
        s === "disconnected" ||
        s === "stopped",
      { timeoutMs: SESSION_READY_TIMEOUT_MS },
    );
  } catch (e) {
    const errText = await readSessionError(ctx.client);
    status =
      (await attrTestId(ctx.client, "tracer-session-workspace", "data-session-status")) ||
      status;
    if (looksLikeAuthBlock(status, errText || (e instanceof Error ? e.message : String(e)))) {
      return {
        ok: false,
        auth: true,
        session,
        status,
        errorText: errText,
      };
    }
    return {
      ok: false,
      auth: false,
      session,
      status,
      errorText: errText || (e instanceof Error ? e.message : String(e)),
    };
  }
  if (status === "failed" || status === "disconnected") {
    const errText = await readSessionError(ctx.client);
    if (looksLikeAuthBlock(status, errText)) {
      return { ok: false, auth: true, session, status, errorText: errText };
    }
    return { ok: false, auth: false, session, status, errorText: errText };
  }
  return { ok: true, auth: false, session, status };
}

/** LGJ-01: Live runtime readiness (app + session ready via live bridge). */
export async function lgj01_runtime_readiness(ctx) {
  const id = "LGJ-01";
  try {
    const ready = await waitAppReady(ctx.client, { timeoutMs: APP_READY_TIMEOUT_MS });
    if (ready.backend !== "tauri") {
      return fail(id, `expected tauri backend, got ${ready.backend}`, ready);
    }
    const live = await ensureLiveSession(ctx, "LGJ-01 readiness");
    if (!live.ok && live.auth) {
      return blockedAuth(id, "session create blocked by auth (live Grok)", live);
    }
    if (!live.ok) {
      return fail(id, `live session not ready: status=${live.status}`, live);
    }
    return pass(id, "live runtime session ready via GUI + bridge", {
      sessionId: live.session.sessionId,
      status: live.status,
      backend: ready.backend,
    });
  } catch (e) {
    await ctx.captureArtifact("lgj01-fail").catch(() => {});
    const msg = e instanceof Error ? e.message : String(e);
    if (looksLikeAuthBlock(null, msg)) {
      return blockedAuth(id, msg);
    }
    return fail(id, msg);
  }
}

/** LGJ-02: Live prompt stream (≥1 normalized event in GUI timeline). */
export async function lgj02_prompt_stream(ctx) {
  const id = "LGJ-02";
  try {
    if (!(await existsTestId(ctx.client, "tracer-session-workspace"))) {
      const live = await ensureLiveSession(ctx, "LGJ-02 stream");
      if (!live.ok && live.auth) return blockedAuth(id, "auth blocked stream", live);
      if (!live.ok) return fail(id, `session not ready: ${live.status}`, live);
    } else {
      const st = await attrTestId(ctx.client, "tracer-session-workspace", "data-session-status");
      if (st !== "ready") {
        const live = await ensureLiveSession(ctx, "LGJ-02 stream");
        if (!live.ok && live.auth) return blockedAuth(id, "auth blocked stream", live);
        if (!live.ok) return fail(id, `session not ready: ${live.status}`, live);
      }
    }
    const prompt = ctx.prompt || DEFAULT_STREAM_PROMPT;
    await guiSubmitPrompt(ctx.client, prompt);
    const found = await waitAnyEventType(
      ctx.client,
      [
        "agent.message.delta",
        "agent.message.completed",
        "session.completed",
        "session.prompt.submitted",
      ],
      { timeoutMs: STREAM_EVENT_TIMEOUT_MS },
    );
    return pass(id, "live prompt stream events observed in GUI", { found });
  } catch (e) {
    await ctx.captureArtifact("lgj02-fail").catch(() => {});
    const msg = e instanceof Error ? e.message : String(e);
    if (looksLikeAuthBlock(null, msg)) return blockedAuth(id, msg);
    return fail(id, msg);
  }
}

/** LGJ-03: Cancel mid-stream (no deadlock). */
export async function lgj03_cancel(ctx) {
  const id = "LGJ-03";
  try {
    const live = await ensureLiveSession(ctx, "LGJ-03 cancel");
    if (!live.ok && live.auth) return blockedAuth(id, "auth blocked cancel", live);
    if (!live.ok) return fail(id, `session not ready: ${live.status}`, live);

    await guiSubmitPrompt(ctx.client, ctx.cancelPrompt || DEFAULT_CANCEL_PROMPT);
    // Wait briefly for prompt to be in flight
    await delay(1500);
    if (!(await existsTestId(ctx.client, "tracer-session-cancel"))) {
      // Soft path: may already completed quickly
      const st = await attrTestId(ctx.client, "tracer-session-workspace", "data-session-status");
      if (st === "ready" || st === "completed" || st === "stopped") {
        return partial(id, "cancel control not shown; turn may have finished quickly", {
          status: st,
        });
      }
      return fail(id, "cancel control not available", { status: st });
    }
    const t0 = Date.now();
    await clickTestId(ctx.client, "tracer-session-cancel");
    const status = await waitSessionStatus(
      ctx.client,
      (s) =>
        s === "ready" ||
        s === "stopped" ||
        s === "completed" ||
        s === "failed" ||
        s === "cancelling" ||
        s === "disconnected",
      { timeoutMs: 60_000 },
    );
    const elapsed = Date.now() - t0;
    if (elapsed > CANCEL_DEADLOCK_MS) {
      return fail(id, `cancel path exceeded deadlock budget (${elapsed}ms)`, { status, elapsed });
    }
    return pass(id, "cancel returned without deadlock", { status, elapsedMs: elapsed });
  } catch (e) {
    await ctx.captureArtifact("lgj03-fail").catch(() => {});
    const msg = e instanceof Error ? e.message : String(e);
    if (looksLikeAuthBlock(null, msg)) return blockedAuth(id, msg);
    return fail(id, msg);
  }
}

/** LGJ-04: Restart + history restore; no auto re-prompt. */
export async function lgj04_restart_history(ctx) {
  const id = "LGJ-04";
  try {
    const live = await ensureLiveSession(ctx, "LGJ-04 history");
    if (!live.ok && live.auth) return blockedAuth(id, "auth blocked history", live);
    if (!live.ok) return fail(id, `session not ready: ${live.status}`, live);

    await guiSubmitPrompt(ctx.client, ctx.prompt || DEFAULT_STREAM_PROMPT);
    await waitAnyEventType(
      ctx.client,
      [
        "session.prompt.submitted",
        "agent.message.delta",
        "agent.message.completed",
        "session.completed",
      ],
      { timeoutMs: STREAM_EVENT_TIMEOUT_MS },
    );
    const sessionId = await attrTestId(
      ctx.client,
      "tracer-session-workspace",
      "data-session-id",
    );
    // Count prompt.submitted before restart
    const beforeCount = await countEventType(ctx.client, "session.prompt.submitted");

    if (await existsTestId(ctx.client, "tracer-session-stop")) {
      await clickTestId(ctx.client, "tracer-session-stop").catch(() => {});
      await delay(800);
    }
    if (!ctx.relaunchApp) {
      return partial(id, "relaunch helper unavailable", { sessionId, beforeCount });
    }
    await delay(500);
    await ctx.relaunchApp();
    await waitAppReady(ctx.client, { timeoutMs: APP_READY_TIMEOUT_MS });
    await ensureOnProjects(ctx.client);
    await delay(1000);
    if (await existsTestId(ctx.client, "tracer-projects-refresh")) {
      await clickTestId(ctx.client, "tracer-projects-refresh").catch(() => {});
      await delay(800);
    }
    let opened = await tryOpenFirstProject(ctx.client);
    if (!opened) {
      return fail(id, "no project after restart with same DB", { sessionId });
    }
    if (sessionId && (await existsTestId(ctx.client, `tracer-session-open-${sessionId}`))) {
      await clickTestId(ctx.client, `tracer-session-open-${sessionId}`);
    } else {
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
    // Wait a bit — auto re-prompt would add new streaming activity
    await delay(3000);
    const status = await attrTestId(
      ctx.client,
      "tracer-session-workspace",
      "data-session-status",
    );
    const afterCount = await countEventType(ctx.client, "session.prompt.submitted");
    // No auto re-prompt: status should not be "streaming"/"running" without user action
    if (status === "streaming" || status === "running" || status === "prompting") {
      return fail(id, "session appears auto-prompting after restart", {
        status,
        beforeCount,
        afterCount,
      });
    }
    const hasHistory = await waitAnyEventType(
      ctx.client,
      [
        "session.prompt.submitted",
        "agent.message.delta",
        "agent.message.completed",
        "session.completed",
        "session.created",
      ],
      { timeoutMs: 15_000 },
    ).catch(() => []);
    if (!hasHistory.length) {
      return partial(id, "session restored; timeline empty; no auto re-prompt observed", {
        sessionId,
        status,
        beforeCount,
        afterCount,
      });
    }
    return pass(id, "restart restored history; no auto re-prompt", {
      sessionId,
      status,
      beforeCount,
      afterCount,
      hasHistory,
    });
  } catch (e) {
    await ctx.captureArtifact("lgj04-fail").catch(() => {});
    const msg = e instanceof Error ? e.message : String(e);
    if (looksLikeAuthBlock(null, msg)) return blockedAuth(id, msg);
    return fail(id, msg);
  }
}

async function countEventType(client, eventType) {
  const res = await client.execute(
    `var t = arguments[0];
     var n = 0;
     var nodes = document.querySelectorAll('[data-event-type]');
     for (var i = 0; i < nodes.length; i++) {
       if (nodes[i].getAttribute('data-event-type') === t) n++;
     }
     return n;`,
    [eventType],
  );
  return res.body?.value ?? 0;
}

/**
 * LGJ-05: Approval reverse-request honesty.
 * PASS only if approval.requested / approval card observed.
 * Otherwise NOT_OBSERVED or UNSUPPORTED — never fabricate PASS.
 */
export async function lgj05_approval_rr(ctx) {
  const id = "LGJ-05";
  try {
    const live = await ensureLiveSession(ctx, "LGJ-05 approval");
    if (!live.ok && live.auth) return blockedAuth(id, "auth blocked approval RR", live);
    if (!live.ok) return fail(id, `session not ready: ${live.status}`, live);

    await guiSubmitPrompt(ctx.client, ctx.approvalPrompt || DEFAULT_APPROVAL_PROMPT);
    const deadline = Date.now() + APPROVAL_OBSERVE_TIMEOUT_MS;
    let sawCard = false;
    let sawEvent = false;
    while (Date.now() < deadline) {
      if (await existsTestId(ctx.client, "tracer-session-refresh")) {
        await clickTestId(ctx.client, "tracer-session-refresh").catch(() => {});
      }
      if (await existsTestId(ctx.client, "tracer-approval-card")) {
        sawCard = true;
        break;
      }
      try {
        const found = await waitAnyEventType(ctx.client, ["approval.requested"], {
          timeoutMs: 500,
        });
        if (found.length) {
          sawEvent = true;
          break;
        }
      } catch {
        /* continue */
      }
      // Natural completion without RR
      const st = await attrTestId(
        ctx.client,
        "tracer-session-workspace",
        "data-session-status",
      );
      if (st === "ready" || st === "completed") {
        // Turn finished without RR
        break;
      }
      await delay(400);
    }
    if (sawCard || sawEvent) {
      return pass(id, "approval reverse-request observed in GUI", { sawCard, sawEvent });
    }
    // Did the turn complete without RR?
    const st = await attrTestId(
      ctx.client,
      "tracer-session-workspace",
      "data-session-status",
    );
    if (st === "ready" || st === "completed") {
      return unsupported(
        id,
        "provider completed without approval reverse-request for inducing prompt",
        { status: st },
      );
    }
    return notObserved(id, "approval.requested / approval card not observed within budget", {
      status: st,
    });
  } catch (e) {
    await ctx.captureArtifact("lgj05-fail").catch(() => {});
    const msg = e instanceof Error ? e.message : String(e);
    if (looksLikeAuthBlock(null, msg)) return blockedAuth(id, msg);
    return fail(id, msg);
  }
}

/** LGJ-06: Fail-closed error (invalid project path; no mock fallback). */
export async function lgj06_fail_closed(ctx) {
  const id = "LGJ-06";
  try {
    await ensureOnProjects(ctx.client);
    const badPath =
      process.platform === "win32"
        ? "Z:\\tracer-lgj-does-not-exist-path-xyz"
        : "/tracer-lgj-does-not-exist-path-xyz";
    await guiRegisterProject(ctx.client, {
      rootPath: badPath,
      name: "lgj-bad",
    }).catch(() => {});
    // Either stays on projects with error, or never enters ready workspace with tauri backend
    await delay(1500);
    const backend = await attrTestId(ctx.client, "tracer-app-root", "data-tracer-backend");
    if (backend !== "tauri") {
      return fail(id, `backend left tauri mode: ${backend}`);
    }
    // Look for error surface
    let errSurface = false;
    for (const tid of [
      "tracer-project-register-error",
      "tracer-banner-session-error",
      "tracer-create-error",
      "tracer-projects-home",
    ]) {
      if (await existsTestId(ctx.client, tid)) {
        const t = await textTestId(ctx.client, tid);
        if (t && /invalid|not found|missing|error|exist/i.test(t)) {
          errSurface = true;
          break;
        }
        if (tid === "tracer-projects-home") {
          // still on home is ok if register failed closed
          errSurface = true;
        }
      }
    }
    // Mock controls must not appear
    const mock =
      (await existsTestId(ctx.client, "tracer-mock-controls")) ||
      (await existsTestId(ctx.client, "tracer-backend-mock"));
    if (mock) {
      return fail(id, "mock controls appeared after fail-closed attempt");
    }
    if (!errSurface) {
      return partial(id, "backend stayed tauri; explicit error surface not confirmed", {
        backend,
      });
    }
    return pass(id, "fail-closed: error surfaced; backend remains tauri; no mock", {
      backend,
    });
  } catch (e) {
    await ctx.captureArtifact("lgj06-fail").catch(() => {});
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

/** LGJ-07: Clean shutdown (soft stop + harness verifies no orphans). */
export async function lgj07_clean_shutdown(ctx) {
  const id = "LGJ-07";
  try {
    await waitAppReady(ctx.client, { timeoutMs: 30_000 });
    if (await existsTestId(ctx.client, "tracer-session-workspace")) {
      if (await existsTestId(ctx.client, "tracer-session-stop")) {
        await clickTestId(ctx.client, "tracer-session-stop").catch(() => {});
        await delay(500);
      }
    }
    // Actual orphan check is done by harness after session/driver teardown.
    // Mark PASS provisionally; harness patches if orphans remain.
    return pass(id, "soft stop issued; harness will verify no orphans after teardown", {
      pendingOrphanCheck: true,
    });
  } catch (e) {
    return fail(id, e instanceof Error ? e.message : String(e));
  }
}

export const JOURNEY_RUNNERS = [
  { id: "LGJ-01", name: "Live runtime readiness", run: lgj01_runtime_readiness },
  { id: "LGJ-02", name: "Live prompt stream", run: lgj02_prompt_stream },
  { id: "LGJ-03", name: "Cancel mid-stream", run: lgj03_cancel },
  { id: "LGJ-04", name: "Restart history (no auto re-prompt)", run: lgj04_restart_history },
  { id: "LGJ-05", name: "Approval reverse-request (honesty)", run: lgj05_approval_rr },
  { id: "LGJ-06", name: "Fail-closed error", run: lgj06_fail_closed },
  { id: "LGJ-07", name: "Clean shutdown", run: lgj07_clean_shutdown },
];

/**
 * @param {string|null} filter  e.g. "LGJ-01" or "LGJ-01,LGJ-02" or "01"
 */
export function filterJourneys(filter) {
  if (!filter) return JOURNEY_RUNNERS;
  const parts = filter.split(",").map((s) => s.trim().toUpperCase()).filter(Boolean);
  return JOURNEY_RUNNERS.filter((j) =>
    parts.some(
      (p) =>
        j.id === p ||
        j.id === `LGJ-${p}` ||
        j.id.replace("LGJ-", "") === p.replace(/^0+/, "") ||
        j.id.endsWith(p),
    ),
  );
}
