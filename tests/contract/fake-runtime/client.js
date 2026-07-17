/**
 * Minimal ACP NDJSON client for driving the fake runtime in contract tests.
 */

import { spawn } from "node:child_process";
import readline from "node:readline";
import path from "node:path";
import process from "node:process";

/**
 * @param {object} opts
 * @param {string} opts.repoRoot
 * @param {string} opts.scenarioId
 * @param {number} [opts.chunkDelayMs]
 * @param {number} [opts.cancelDelayMs]
 * @param {string[]} [opts.extraArgs]
 */
export function spawnFakeRuntime(opts) {
  const bin = path.join(
    opts.repoRoot,
    "tools",
    "fake-acp-runtime",
    "bin",
    "fake-acp-runtime.js",
  );
  const args = [bin, "--scenario", opts.scenarioId];
  if (opts.chunkDelayMs != null) {
    args.push("--chunk-delay-ms", String(opts.chunkDelayMs));
  }
  if (opts.cancelDelayMs != null) {
    args.push("--cancel-delay-ms", String(opts.cancelDelayMs));
  }
  if (opts.extraArgs) args.push(...opts.extraArgs);

  const child = spawn(process.execPath, args, {
    cwd: opts.repoRoot,
    stdio: ["pipe", "pipe", "pipe"],
    env: {
      ...process.env,
      // Ensure no accidental live smoke flags / provider keys
      TRACER_LIVE_SMOKE: "",
      XAI_API_KEY: "",
      GROK_API_KEY: "",
    },
  });

  /** @type {object[]} */
  const messages = [];
  /** @type {string[]} */
  const rawLines = [];
  /** @type {string[]} */
  const stderrLines = [];

  let malformedFrames = 0;
  let closed = false;
  /** @type {number|null} */
  let exitCode = null;
  let resolveExit;
  const exitPromise = new Promise((resolve) => {
    resolveExit = resolve;
  });

  const rl = readline.createInterface({
    input: child.stdout,
    crlfDelay: Infinity,
  });

  rl.on("line", (line) => {
    rawLines.push(line);
    const trimmed = line.trim();
    if (!trimmed) return;
    try {
      const msg = JSON.parse(trimmed);
      messages.push(msg);
    } catch {
      malformedFrames += 1;
      messages.push({ _malformed: true, raw: line });
    }
  });

  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => {
    for (const line of String(chunk).split(/\r?\n/)) {
      if (line) stderrLines.push(line);
    }
  });

  child.on("exit", (code) => {
    closed = true;
    exitCode = code;
    resolveExit(code);
  });

  function send(obj) {
    if (closed) throw new Error("child already exited");
    child.stdin.write(`${JSON.stringify(obj)}\n`);
  }

  function waitFor(predicate, timeoutMs = 5000) {
    const start = Date.now();
    return new Promise((resolve, reject) => {
      const tick = () => {
        for (const m of messages) {
          if (predicate(m)) return resolve(m);
        }
        if (closed && Date.now() - start > 50) {
          // allow a brief drain after exit
          for (const m of messages) {
            if (predicate(m)) return resolve(m);
          }
        }
        if (Date.now() - start > timeoutMs) {
          return reject(
            new Error(
              `timeout waiting for message after ${timeoutMs}ms (got ${messages.length} msgs)`,
            ),
          );
        }
        setTimeout(tick, 10);
      };
      tick();
    });
  }

  async function waitForResponse(id, timeoutMs = 5000) {
    return waitFor(
      (m) =>
        m &&
        !m._malformed &&
        m.id === id &&
        (m.result !== undefined || m.error !== undefined),
      timeoutMs,
    );
  }

  function closeStdin() {
    try {
      child.stdin.end();
    } catch {
      /* ignore */
    }
  }

  function kill(signal = "SIGTERM") {
    try {
      child.kill(signal);
    } catch {
      /* ignore */
    }
  }

  return {
    child,
    send,
    waitFor,
    waitForResponse,
    closeStdin,
    kill,
    exitPromise,
    get messages() {
      return messages;
    },
    get rawLines() {
      return rawLines;
    },
    get stderrLines() {
      return stderrLines;
    },
    get malformedFrames() {
      return malformedFrames;
    },
    get exitCode() {
      return exitCode;
    },
  };
}

/**
 * Drive a standard initialize → authenticate → session/new flow.
 */
export async function handshake(client, { authenticate = true } = {}) {
  client.send({
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: {
      protocolVersion: 1,
      clientCapabilities: {
        fs: { readTextFile: true, writeTextFile: true },
        terminal: false,
      },
      _meta: {
        clientType: "tracer-contract-harness",
        clientIdentifier: "tracer-w1-g",
        clientVersion: "0.1.0",
      },
    },
  });
  const init = await client.waitForResponse(1);
  if (init.error) throw new Error(`initialize failed: ${JSON.stringify(init.error)}`);

  if (authenticate) {
    client.send({
      jsonrpc: "2.0",
      id: 2,
      method: "authenticate",
      params: { methodId: "fake-auth" },
    });
    const auth = await client.waitForResponse(2);
    if (auth.error) throw new Error(`authenticate failed: ${JSON.stringify(auth.error)}`);
  }

  client.send({
    jsonrpc: "2.0",
    id: 3,
    method: "session/new",
    params: { cwd: "{{PROJECT_ROOT}}", mcpServers: [] },
  });
  const session = await client.waitForResponse(3);
  return { init, session };
}

export function observeMessages(messages) {
  const obs = {
    initializeOk: false,
    sessionNewOk: false,
    sessionNewAuthError: false,
    promptSubmitted: false,
    promptStopReason: null,
    agentMessageChunks: 0,
    toolCalls: 0,
    toolCompleted: 0,
    toolFailed: 0,
    permissionRequests: 0,
    permissionResolved: false,
    malformedFrames: 0,
    unknownVendor: 0,
    duplicateResponseIds: 0,
    responseIds: /** @type {Map<any, number>} */ (new Map()),
    eofWithoutPromptResult: false,
    exited: false,
    exitCode: null,
    capabilitiesMeta: null,
  };

  for (const m of messages) {
    if (m._malformed) {
      obs.malformedFrames += 1;
      continue;
    }
    if (m.id === 1 && m.result) {
      obs.initializeOk = true;
      obs.capabilitiesMeta = m.result?.agentCapabilities?._meta ?? m.result?._meta ?? null;
    }
    if (m.id === 3 && m.result?.sessionId) obs.sessionNewOk = true;
    if (m.id === 3 && m.error?.message?.includes("Authentication required")) {
      obs.sessionNewAuthError = true;
    }
    if (m.method === "session/request_permission") obs.permissionRequests += 1;
    if (m.method === "x.ai/unknown_vendor_extension") obs.unknownVendor += 1;
    if (m.method === "session/update") {
      const u = m.params?.update;
      if (u?.sessionUpdate === "agent_message_chunk") obs.agentMessageChunks += 1;
      if (u?.sessionUpdate === "tool_call") obs.toolCalls += 1;
      if (u?.sessionUpdate === "tool_call_update") {
        if (u.status === "completed") obs.toolCompleted += 1;
        if (u.status === "failed") obs.toolFailed += 1;
      }
    }
    if (m.id != null && (m.result !== undefined || m.error !== undefined)) {
      const n = (obs.responseIds.get(m.id) || 0) + 1;
      obs.responseIds.set(m.id, n);
      if (n > 1) obs.duplicateResponseIds += 1;
      if (m.result?.stopReason) {
        obs.promptSubmitted = true;
        obs.promptStopReason = m.result.stopReason;
      }
    }
  }
  return obs;
}
