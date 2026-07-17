/**
 * Fake ACP JSON-RPC 2.0 NDJSON agent loop.
 * No network. No provider credentials. Deterministic scenario scripts.
 */

import readline from "node:readline";
import {
  FIXED,
  buildInitializeResult,
  assertScenarioId,
  tracerCapabilitiesFor,
} from "./scenarios.js";

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function writeLine(stream, obj) {
  stream.write(`${JSON.stringify(obj)}\n`);
}

function writeRaw(stream, text) {
  stream.write(text.endsWith("\n") ? text : `${text}\n`);
}

function ok(id, result) {
  return { jsonrpc: "2.0", id, result };
}

function err(id, code, message, data) {
  const body = { jsonrpc: "2.0", id, error: { code, message } };
  if (data !== undefined) body.error.data = data;
  return body;
}

function sessionUpdate(sessionId, update) {
  return {
    jsonrpc: "2.0",
    method: "session/update",
    params: { sessionId, update },
  };
}

/**
 * @param {object} options
 * @param {string} options.scenarioId
 * @param {NodeJS.ReadableStream} [options.stdin]
 * @param {NodeJS.WritableStream} [options.stdout]
 * @param {NodeJS.WritableStream} [options.stderr]
 * @param {number} [options.chunkDelayMs]
 * @param {number} [options.cancelDelayMs]
 * @param {() => void} [options.onExit]
 */
export async function runFakeRuntime(options) {
  const scenario = assertScenarioId(options.scenarioId);
  const stdin = options.stdin ?? process.stdin;
  const stdout = options.stdout ?? process.stdout;
  const stderr = options.stderr ?? process.stderr;
  const chunkDelayMs = Number(options.chunkDelayMs ?? 0);
  // slow_cancel_ack default: long delay so parent force-kills; injectable for tests
  const cancelDelayMs = Number(
    options.cancelDelayMs ??
      (scenario.cancelAck === "delayed" ? 60_000 : 0),
  );

  const state = {
    initialized: false,
    authenticated: false,
    sessionId: null,
    promptActive: false,
    promptId: null,
    cancelled: false,
    permissionPending: null,
    closed: false,
    exitCode: 0,
  };

  const caps = tracerCapabilitiesFor(scenario.capProfile);

  function log(msg) {
    // stderr is logs only — never JSON-RPC
    stderr.write(`[fake-acp-runtime] scenario=${scenario.id} ${msg}\n`);
  }

  function emit(msg) {
    if (state.closed) return;
    writeLine(stdout, msg);
  }

  function finish(code = 0) {
    if (state.closed) return;
    state.closed = true;
    state.exitCode = code;
    try {
      stdout.end?.();
    } catch {
      /* ignore */
    }
    if (options.onExit) {
      options.onExit(code);
    } else if (typeof process !== "undefined" && process.exitCode === undefined) {
      process.exitCode = code;
    }
  }

  async function maybeDelay() {
    if (chunkDelayMs > 0) await sleep(chunkDelayMs);
  }

  async function handleInitialize(msg) {
    state.initialized = true;
    const result = buildInitializeResult(scenario.capProfile);
    emit(ok(msg.id, result));
    log("initialize ok");
  }

  async function handleAuthenticate(msg) {
    // Fake auth is always a no-op success (CI). Live credentials never read.
    state.authenticated = true;
    emit(ok(msg.id, {}));
    log("authenticate no-op success (fake)");
  }

  async function handleSessionNew(msg) {
    if (!state.initialized) {
      emit(err(msg.id, -32002, "Runtime not initialized"));
      return;
    }
    if (scenario.authRequiredOnSessionNew && !state.authenticated) {
      // Wire shape aligned with live-scrubbed session-new-auth-required.json
      emit(
        err(msg.id, -32000, "Authentication required", "no auth method id provided"),
      );
      log("session/new auth required");
      return;
    }
    state.sessionId = FIXED.sessionId;
    state.authenticated = true;
    emit(
      ok(msg.id, {
        sessionId: state.sessionId,
        _meta: {
          fakeRuntime: true,
          evidence: scenario.evidence,
        },
      }),
    );
    log(`session/new ok sessionId=${state.sessionId}`);
  }

  async function handleSessionLoad(msg) {
    if (scenario.authRequiredOnSessionNew && !state.authenticated) {
      emit(
        err(msg.id, -32000, "Authentication required", "no auth method id provided"),
      );
      return;
    }
    const sid = msg.params?.sessionId ?? FIXED.sessionId;
    state.sessionId = sid;
    emit(ok(msg.id, { sessionId: sid }));
  }

  function requireSession(msg) {
    if (!state.sessionId) {
      emit(err(msg.id, -32001, "Session not found"));
      return false;
    }
    return true;
  }

  async function runStreamTools(msg) {
    const sid = state.sessionId;
    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_thought_chunk",
        content: { type: "text", text: "Scanning top-level entries…" },
      }),
    );
    if (state.cancelled) return finalizePrompt(msg, "cancelled");

    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "tool_call",
        toolCallId: FIXED.toolCallList,
        title: "List project files",
        kind: "search",
        status: "pending",
      }),
    );
    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "tool_call_update",
        toolCallId: FIXED.toolCallList,
        status: "in_progress",
      }),
    );
    if (state.cancelled) return finalizePrompt(msg, "cancelled");

    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "tool_call_update",
        toolCallId: FIXED.toolCallList,
        status: "completed",
        content: [
          {
            type: "content",
            content: { type: "text", text: "LICENSE\nREADME.md\nsrc/" },
          },
        ],
      }),
    );
    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "This repository contains " },
      }),
    );
    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: {
          type: "text",
          text: "a small application skeleton with a license and source tree.",
        },
      }),
    );
    if (state.cancelled) return finalizePrompt(msg, "cancelled");
    return finalizePrompt(msg, "end_turn");
  }

  function finalizePrompt(msg, stopReason) {
    if (state.closed) return;
    state.promptActive = false;
    emit(
      ok(msg.id, {
        stopReason,
        _meta: {
          usage: { inputTokens: 120, outputTokens: 48 },
          fakeRuntime: true,
          cancelled: stopReason === "cancelled",
        },
      }),
    );
    log(`session/prompt done stopReason=${stopReason}`);
  }

  async function waitForPermissionOrCancel() {
    const started = Date.now();
    const budget = 120_000;
    while (!state.closed && state.permissionPending) {
      if (state.cancelled) return "cancelled";
      if (state.permissionPending.resolved) {
        return state.permissionPending.outcome;
      }
      if (Date.now() - started > budget) return "timeout";
      await sleep(5);
    }
    return "gone";
  }

  async function runPermission(msg) {
    const sid = state.sessionId;
    const permId = 42;
    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "Preparing an edit…" },
      }),
    );
    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "tool_call",
        toolCallId: FIXED.toolCallEdit,
        title: "Edit README.md",
        kind: "edit",
        status: "pending",
        locations: [{ path: "README.md" }],
      }),
    );

    state.permissionPending = {
      id: permId,
      resolved: false,
      outcome: null,
    };

    // Reverse-request (agent → client)
    emit({
      jsonrpc: "2.0",
      id: permId,
      method: "session/request_permission",
      params: {
        sessionId: sid,
        toolCall: {
          toolCallId: FIXED.toolCallEdit,
          title: "Edit README.md",
          kind: "edit",
          status: "pending",
          locations: [{ path: "README.md" }],
        },
        options: [
          { optionId: "allow-once", name: "Allow once", kind: "allow_once" },
          {
            optionId: "allow-always",
            name: "Allow always for this session",
            kind: "allow_always",
          },
          { optionId: "reject-once", name: "Reject", kind: "reject_once" },
        ],
      },
    });
    log("session/request_permission issued");

    const outcome = await waitForPermissionOrCancel();

    if (outcome === "cancelled" || state.cancelled) {
      state.permissionPending = null;
      emit(
        sessionUpdate(sid, {
          sessionUpdate: "tool_call_update",
          toolCallId: FIXED.toolCallEdit,
          status: "failed",
          content: [
            {
              type: "content",
              content: { type: "text", text: "Cancelled while permission pending" },
            },
          ],
        }),
      );
      return finalizePrompt(msg, "cancelled");
    }

    const selected = state.permissionPending?.optionId ?? null;
    const allowed =
      outcome === "selected" &&
      selected &&
      (selected === "allow-once" || selected === "allow-always");

    state.permissionPending = null;

    if (!allowed) {
      emit(
        sessionUpdate(sid, {
          sessionUpdate: "tool_call_update",
          toolCallId: FIXED.toolCallEdit,
          status: "failed",
          content: [
            {
              type: "content",
              content: { type: "text", text: "Permission denied" },
            },
          ],
        }),
      );
      // Fail closed: no silent success completion of the tool path
      return finalizePrompt(msg, "end_turn");
    }

    emit(
      sessionUpdate(sid, {
        sessionUpdate: "tool_call_update",
        toolCallId: FIXED.toolCallEdit,
        status: "completed",
        content: [
          {
            type: "content",
            content: { type: "text", text: "README.md updated (fake)" },
          },
        ],
      }),
    );
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "Edit applied." },
      }),
    );
    return finalizePrompt(msg, "end_turn");
  }

  async function runCancelStream(msg) {
    const sid = state.sessionId;
    // Emit a long-ish stream so the harness can cancel mid-flight.
    for (let i = 0; i < 40; i++) {
      if (state.cancelled || state.closed) break;
      emit(
        sessionUpdate(sid, {
          sessionUpdate: "agent_message_chunk",
          content: { type: "text", text: `chunk-${i} ` },
        }),
      );
      await sleep(chunkDelayMs > 0 ? chunkDelayMs : 15);
    }
    if (state.cancelled) {
      return finalizePrompt(msg, "cancelled");
    }
    return finalizePrompt(msg, "end_turn");
  }

  async function runCancelPermission(msg) {
    // Same as permission but leave reverse-request open until cancel.
    return runPermission(msg);
  }

  async function runMalformed(msg) {
    const sid = state.sessionId;
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "before-malformed " },
      }),
    );
    await maybeDelay();
    // Invalid JSON line — adapter must surface protocol error
    writeRaw(stdout, "{this is not valid jsonrpc\n");
    log("emitted malformed frame");
    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "after-malformed " },
      }),
    );
    return finalizePrompt(msg, "end_turn");
  }

  async function runVendorUnknown(msg) {
    const sid = state.sessionId;
    emit({
      jsonrpc: "2.0",
      method: "x.ai/unknown_vendor_extension",
      params: {
        sessionId: sid,
        payload: { note: "unmapped vendor notification for adapter.protocol.unknown" },
      },
    });
    log("emitted unknown vendor notification");
    await maybeDelay();
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "continued after vendor noise" },
      }),
    );
    return finalizePrompt(msg, "end_turn");
  }

  async function runEof(msg) {
    const sid = state.sessionId;
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "partial… " },
      }),
    );
    await maybeDelay();
    log("EOF mid-prompt: closing stdout without prompt result");
    // Close stdout without completing prompt — client sees pipe EOF
    state.promptActive = false;
    state.closed = true;
    try {
      stdout.end();
    } catch {
      /* ignore */
    }
    if (options.onExit) options.onExit(0);
    else process.exitCode = 0;
  }

  async function runCrash(msg) {
    const sid = state.sessionId;
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "about to crash " },
      }),
    );
    await maybeDelay();
    log("crash_nonzero_exit: exiting 1");
    const code = scenario.exitOnCrash ?? 1;
    state.promptActive = false;
    state.closed = true;
    if (options.onExit) {
      options.onExit(code);
    } else {
      process.exit(code);
    }
  }

  async function runDuplicateId(msg) {
    const sid = state.sessionId;
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text: "hello " },
      }),
    );
    // First legitimate response
    emit(
      ok(msg.id, {
        stopReason: "end_turn",
        _meta: { usage: { inputTokens: 1, outputTokens: 1 }, fakeRuntime: true },
      }),
    );
    // Duplicate response id — ProtocolViolation for adapter
    emit(
      ok(msg.id, {
        stopReason: "end_turn",
        _meta: {
          usage: { inputTokens: 1, outputTokens: 1 },
          fakeRuntime: true,
          duplicate: true,
        },
      }),
    );
    log("emitted duplicate response id");
    state.promptActive = false;
  }

  async function runMinimalComplete(msg) {
    const sid = state.sessionId;
    // No streaming when promptStreaming is false: single message complete via one chunk
    // (adapter may synthesize agent.message.completed)
    emit(
      sessionUpdate(sid, {
        sessionUpdate: "agent_message_chunk",
        content: {
          type: "text",
          text: "Minimal capability response without tools or plans.",
        },
      }),
    );
    return finalizePrompt(msg, "end_turn");
  }

  async function handlePrompt(msg) {
    if (!requireSession(msg)) return;
    if (state.promptActive) {
      emit(err(msg.id, -32003, "Prompt already active"));
      return;
    }
    state.promptActive = true;
    state.cancelled = false;
    state.promptId =
      msg.params?._meta?.promptId ?? FIXED.promptId;

    log(`session/prompt start mode=${scenario.promptMode}`);

    switch (scenario.promptMode) {
      case "stream_tools":
        return runStreamTools(msg);
      case "permission":
        return runPermission(msg);
      case "cancel_stream":
        return runCancelStream(msg);
      case "cancel_permission":
        return runCancelPermission(msg);
      case "malformed":
        return runMalformed(msg);
      case "vendor_unknown":
        return runVendorUnknown(msg);
      case "eof":
        return runEof(msg);
      case "crash":
        return runCrash(msg);
      case "duplicate_id":
        return runDuplicateId(msg);
      case "minimal_complete":
        return runMinimalComplete(msg);
      case "idle_until_eof":
        // Hold prompt open until cancel or stdin EOF (used rarely)
        while (!state.closed && !state.cancelled) {
          await sleep(20);
        }
        if (!state.closed) return finalizePrompt(msg, state.cancelled ? "cancelled" : "end_turn");
        return;
      default:
        return finalizePrompt(msg, "end_turn");
    }
  }

  async function handleCancel(msg) {
    const sid = msg.params?.sessionId ?? state.sessionId;
    log(`session/cancel received sessionId=${sid} ack=${scenario.cancelAck}`);

    if (scenario.cancelAck === "unsupported" || !caps.cancellation) {
      // Advertise no cancellation: ignore cooperative cancel (parent must process-stop).
      log("cancel ignored (unsupported)");
      return;
    }

    if (scenario.cancelAck === "delayed") {
      log(`cancel delayed ${cancelDelayMs}ms`);
      await sleep(cancelDelayMs);
    }

    state.cancelled = true;

    // If permission reverse-request is open, treat cancel as reject of that request
    // by synthesizing a cancelled client response path internally when no response arrives.
    if (state.permissionPending && !state.permissionPending.resolved) {
      state.permissionPending.resolved = true;
      state.permissionPending.outcome = "cancelled";
      state.permissionPending.optionId = null;
    }
  }

  async function handlePermissionResponse(msg) {
    // Client answered session/request_permission
    if (!state.permissionPending) return;
    if (msg.id !== state.permissionPending.id && msg.id != null) {
      // still accept if only one pending
    }
    if (msg.error) {
      state.permissionPending.resolved = true;
      state.permissionPending.outcome = "cancelled";
      return;
    }
    const outcome = msg.result?.outcome;
    if (outcome?.outcome === "selected") {
      state.permissionPending.resolved = true;
      state.permissionPending.outcome = "selected";
      state.permissionPending.optionId = outcome.optionId;
    } else if (outcome?.outcome === "cancelled") {
      state.permissionPending.resolved = true;
      state.permissionPending.outcome = "cancelled";
    } else {
      // permissive parse of optionId at top level
      state.permissionPending.resolved = true;
      state.permissionPending.outcome = "selected";
      state.permissionPending.optionId =
        msg.result?.optionId ?? outcome?.optionId ?? "reject-once";
    }
    log(
      `permission response outcome=${state.permissionPending.outcome} option=${state.permissionPending.optionId}`,
    );
  }

  async function dispatch(msg) {
    if (!msg || typeof msg !== "object") return;

    // Client response to our reverse-request
    if (msg.id != null && msg.method == null && (msg.result !== undefined || msg.error)) {
      await handlePermissionResponse(msg);
      return;
    }

    const method = msg.method;
    if (!method) return;

    switch (method) {
      case "initialize":
        return handleInitialize(msg);
      case "authenticate":
        return handleAuthenticate(msg);
      case "session/new":
        return handleSessionNew(msg);
      case "session/load":
        return handleSessionLoad(msg);
      case "session/prompt":
        return handlePrompt(msg);
      case "session/cancel":
        return handleCancel(msg);
      default:
        if (msg.id != null) {
          emit(err(msg.id, -32601, `Method not found: ${method}`));
        } else {
          log(`ignored notification method=${method}`);
        }
    }
  }

  log(`start caps=${JSON.stringify(caps)}`);

  const rl = readline.createInterface({ input: stdin, crlfDelay: Infinity });

  const pending = [];

  rl.on("line", (line) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    let msg;
    try {
      msg = JSON.parse(trimmed);
    } catch (e) {
      log(`client sent non-json line: ${e.message}`);
      return;
    }
    // Serialize handlers to keep scenario order deterministic
    const run = Promise.resolve()
      .then(() => dispatch(msg))
      .catch((e) => {
        log(`handler error: ${e?.stack || e}`);
      });
    pending.push(run);
  });

  await new Promise((resolve) => {
    rl.on("close", resolve);
  });

  await Promise.all(pending);

  if (!state.closed) {
    log("stdin closed — clean shutdown");
    finish(0);
  }

  return { exitCode: state.exitCode, scenarioId: scenario.id };
}
