#!/usr/bin/env node
/**
 * Soak-only burst ACP fake (VS1-H3).
 *
 * Emits N agent_message_chunk session/update notifications on prompt so the
 * control-plane bridge (BRIDGE_CAPACITY=256) saturates under load.
 *
 * No network. No credentials. Not live Grok parity.
 *
 * Env:
 *   TRACER_SOAK_BURST_COUNT   number of stream chunks (default 600)
 *   TRACER_SOAK_BURST_DELAY_MS optional inter-chunk delay ms (default 0)
 *   TRACER_SOAK_SCENARIO       happy_burst (default) | permission_hold
 */
import readline from "node:readline";

function argValue(flag) {
  const argv = process.argv.slice(2);
  const i = argv.indexOf(flag);
  if (i >= 0 && argv[i + 1]) return argv[i + 1];
  const pref = `${flag}=`;
  const hit = argv.find((a) => a.startsWith(pref));
  return hit ? hit.slice(pref.length) : null;
}

const burstCount = Math.max(
  1,
  Number(process.env.TRACER_SOAK_BURST_COUNT ?? 600) || 600,
);
// Default 0ms; set TRACER_SOAK_BURST_DELAY_MS=1 if host stdio buffering stalls the parent reader.
const chunkDelayMs = Math.max(
  0,
  Number(process.env.TRACER_SOAK_BURST_DELAY_MS ?? 0) || 0,
);
// Prefer soak env, then --scenario from control-plane spawn, then default.
const scenario =
  process.env.TRACER_SOAK_SCENARIO ||
  argValue("--scenario") ||
  process.env.TRACER_FAKE_ACP_SCENARIO ||
  "happy_burst";

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function write(obj) {
  process.stdout.write(`${JSON.stringify(obj)}\n`);
}

function ok(id, result) {
  write({ jsonrpc: "2.0", id, result });
}

function err(id, code, message) {
  write({ jsonrpc: "2.0", id, error: { code, message } });
}

function sessionUpdate(sessionId, update) {
  write({
    jsonrpc: "2.0",
    method: "session/update",
    params: { sessionId, update },
  });
}

const SESSION_ID = "soak-burst-session";
const state = {
  initialized: false,
  sessionId: null,
  cancelled: false,
  promptActive: false,
  permissionPending: null,
  closed: false,
};

function log(msg) {
  process.stderr.write(
    `[soak-burst-fake] scenario=${scenario} burst=${burstCount} ${msg}\n`,
  );
}

function initResult() {
  return {
    protocolVersion: 1,
    agentCapabilities: {
      loadSession: true,
      promptCapabilities: {
        image: false,
        audio: false,
        embeddedContext: true,
      },
      mcpCapabilities: { http: false, sse: false },
      sessionCapabilities: {},
      auth: {},
      _meta: {
        "tracer/fake": true,
        "tracer/cancellation": true,
        "tracer/promptStreaming": true,
        "tracer/toolCalls": true,
        "tracer/approvals": true,
        "tracer/capabilities": {
          promptStreaming: true,
          cancellation: true,
          planUpdates: true,
          toolCalls: true,
          approvals: true,
          fileChangeNotifications: false,
          terminalOutput: false,
        },
      },
    },
    authMethods: [
      {
        id: "fake-auth",
        name: "Fake Auth",
        description: "Soak no-op auth",
      },
    ],
    _meta: {
      fakeRuntime: true,
      evidence: "fake-runtime",
      soakBurst: true,
      burstCount,
    },
  };
}

async function runBurstPrompt(msg) {
  const sid = state.sessionId;
  state.promptActive = true;
  state.cancelled = false;
  log(`prompt start emitting ${burstCount} chunks`);

  for (let i = 0; i < burstCount; i++) {
    if (state.cancelled || state.closed) break;
    sessionUpdate(sid, {
      sessionUpdate: "agent_message_chunk",
      content: { type: "text", text: `chunk-${i} ` },
    });
    if (chunkDelayMs > 0) await sleep(chunkDelayMs);
  }

  if (!state.closed) {
    ok(msg.id, {
      stopReason: state.cancelled ? "cancelled" : "end_turn",
      _meta: {
        fakeRuntime: true,
        soakBurst: true,
        emitted: burstCount,
        cancelled: state.cancelled,
      },
    });
  }
  state.promptActive = false;
  log(`prompt done cancelled=${state.cancelled}`);
}

async function runPermissionHold(msg) {
  const sid = state.sessionId;
  state.promptActive = true;
  state.cancelled = false;
  const permId = 9001;
  sessionUpdate(sid, {
    sessionUpdate: "agent_message_chunk",
    content: { type: "text", text: "awaiting approval for soak race" },
  });
  write({
    jsonrpc: "2.0",
    id: permId,
    method: "session/request_permission",
    params: {
      sessionId: sid,
      toolCall: {
        toolCallId: "soak-tool-1",
        title: "Soak permission",
        kind: "edit",
        status: "pending",
      },
      options: [
        { optionId: "allow-once", name: "Allow once", kind: "allow_once" },
        { optionId: "reject-once", name: "Reject", kind: "reject_once" },
      ],
    },
  });
  state.permissionPending = {
    id: permId,
    resolved: false,
    outcome: null,
    optionId: null,
  };
  const started = Date.now();
  while (!state.closed && state.permissionPending && !state.permissionPending.resolved) {
    if (state.cancelled) break;
    if (Date.now() - started > 120_000) break;
    await sleep(10);
  }
  const allowed =
    state.permissionPending?.outcome === "selected" &&
    (state.permissionPending?.optionId === "allow-once" ||
      state.permissionPending?.optionId === "allow-always");
  state.permissionPending = null;
  if (!state.closed) {
    ok(msg.id, {
      stopReason: state.cancelled ? "cancelled" : allowed ? "end_turn" : "rejected",
      _meta: { fakeRuntime: true, soakPermission: true },
    });
  }
  state.promptActive = false;
}

async function handle(msg) {
  if (!msg || typeof msg !== "object") return;

  // Client response to reverse-request
  if (msg.id != null && msg.method == null && (msg.result !== undefined || msg.error)) {
    if (state.permissionPending) {
      state.permissionPending.resolved = true;
      if (msg.error) {
        state.permissionPending.outcome = "cancelled";
      } else {
        const outcome = msg.result?.outcome;
        if (outcome?.outcome === "selected") {
          state.permissionPending.outcome = "selected";
          state.permissionPending.optionId = outcome.optionId;
        } else {
          state.permissionPending.outcome = "cancelled";
        }
      }
    }
    return;
  }

  const method = msg.method;
  if (!method) return;

  switch (method) {
    case "initialize":
      state.initialized = true;
      ok(msg.id, initResult());
      log("initialize ok");
      return;
    case "authenticate":
      ok(msg.id, {});
      return;
    case "session/new":
    case "session/load": {
      if (!state.initialized) {
        err(msg.id, -32002, "Runtime not initialized");
        return;
      }
      state.sessionId = SESSION_ID;
      ok(msg.id, {
        sessionId: SESSION_ID,
        _meta: { fakeRuntime: true, soakBurst: true },
      });
      log("session/new ok");
      return;
    }
    case "session/prompt":
      if (!state.sessionId) {
        err(msg.id, -32001, "Session not found");
        return;
      }
      if (scenario === "permission_hold") {
        return runPermissionHold(msg);
      }
      return runBurstPrompt(msg);
    case "session/cancel":
      state.cancelled = true;
      if (state.permissionPending && !state.permissionPending.resolved) {
        state.permissionPending.resolved = true;
        state.permissionPending.outcome = "cancelled";
      }
      log("cancel accepted");
      return;
    default:
      if (msg.id != null) err(msg.id, -32601, `Method not found: ${method}`);
  }
}

log("start");
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
let chain = Promise.resolve();
rl.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  let msg;
  try {
    msg = JSON.parse(trimmed);
  } catch (e) {
    log(`bad json: ${e.message}`);
    return;
  }
  chain = chain.then(() => handle(msg)).catch((e) => log(`handler error: ${e}`));
});
rl.on("close", () => {
  state.closed = true;
  log("stdin closed");
  process.exitCode = 0;
});
