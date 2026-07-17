import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  findRepoRoot,
  mapWireObservationToProductTypes,
  loadExpectedEvents,
} from "../../../packages/test-fixtures/src/index.js";
import { FIXED, LIVE_ONLY_SCENARIOS } from "../../../tools/fake-acp-runtime/src/index.js";
import { spawnFakeRuntime, handshake, observeMessages } from "./client.js";

const root = findRepoRoot();

async function withClient(scenarioId, fn, opts = {}) {
  const client = spawnFakeRuntime({
    repoRoot: root,
    scenarioId,
    chunkDelayMs: opts.chunkDelayMs ?? 0,
    cancelDelayMs: opts.cancelDelayMs,
  });
  try {
    return await fn(client);
  } finally {
    client.closeStdin();
    // Ensure process ends
    const code = await Promise.race([
      client.exitPromise,
      new Promise((r) => setTimeout(() => {
        client.kill();
        r(client.exitCode);
      }, 2000)),
    ]);
    return code;
  }
}

describe("fake-acp-runtime harness", () => {
  it("happy_prompt_stream: init, session, stream tools, complete", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "happy_prompt_stream",
    });
    try {
      const { init, session } = await handshake(client, { authenticate: true });
      assert.equal(init.result.protocolVersion, 1);
      assert.equal(init.result._meta.fakeRuntime, true);
      assert.equal(init.result._meta.notLiveParity, true);
      assert.equal(session.result.sessionId, FIXED.sessionId);

      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "Summarize the repository layout." }],
          _meta: { promptId: FIXED.promptId },
        },
      });
      const result = await client.waitForResponse(10, 8000);
      assert.equal(result.result.stopReason, "end_turn");

      const obs = observeMessages(client.messages);
      assert.equal(obs.initializeOk, true);
      assert.equal(obs.sessionNewOk, true);
      assert.ok(obs.agentMessageChunks >= 1);
      assert.ok(obs.toolCalls >= 1);
      assert.ok(obs.toolCompleted >= 1);

      const product = mapWireObservationToProductTypes(obs);
      assert.ok(product.includes("runtime.process.ready"));
      assert.ok(product.includes("agent.message.delta"));
      assert.ok(product.includes("session.completed"));

      // expected-events pack is product-level; wire obs should not claim live parity
      const pack = loadExpectedEvents("happy_prompt_stream", root);
      assert.equal(pack.evidence, "fake-runtime");
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("auth_required_session_new: process init ok, session not ready", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "auth_required_session_new",
    });
    try {
      // initialize without authenticate
      client.send({
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: { protocolVersion: 1, clientCapabilities: {} },
      });
      const init = await client.waitForResponse(1);
      assert.ok(init.result);
      assert.equal(init.result.protocolVersion, 1);

      client.send({
        jsonrpc: "2.0",
        id: 3,
        method: "session/new",
        params: { cwd: "{{PROJECT_ROOT}}", mcpServers: [] },
      });
      const session = await client.waitForResponse(3);
      assert.ok(session.error);
      assert.equal(session.error.code, -32000);
      assert.match(session.error.message, /Authentication required/i);

      const obs = observeMessages(client.messages);
      assert.equal(obs.initializeOk, true);
      assert.equal(obs.sessionNewAuthError, true);
      assert.equal(obs.sessionNewOk, false);

      const pack = loadExpectedEvents("auth_required_session_new", root);
      assert.equal(pack.processVsSessionGates.forbidSessionReady, true);
      assert.ok(pack.forbiddenTypes.includes("session.ready"));
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("permission_allow: reverse-request then complete tool", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "permission_allow",
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "edit" }],
        },
      });
      const perm = await client.waitFor(
        (m) => m.method === "session/request_permission",
        5000,
      );
      assert.equal(perm.params.toolCall.toolCallId, FIXED.toolCallEdit);
      client.send({
        jsonrpc: "2.0",
        id: perm.id,
        result: { outcome: { outcome: "selected", optionId: "allow-once" } },
      });
      const result = await client.waitForResponse(10, 5000);
      assert.equal(result.result.stopReason, "end_turn");
      const obs = observeMessages(client.messages);
      assert.ok(obs.permissionRequests >= 1);
      assert.ok(obs.toolCompleted >= 1);
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("permission_deny: fail closed, tool failed", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "permission_deny",
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "edit" }],
        },
      });
      const perm = await client.waitFor(
        (m) => m.method === "session/request_permission",
        5000,
      );
      client.send({
        jsonrpc: "2.0",
        id: perm.id,
        result: { outcome: { outcome: "selected", optionId: "reject-once" } },
      });
      await client.waitForResponse(10, 5000);
      const obs = observeMessages(client.messages);
      assert.ok(obs.toolFailed >= 1);
      // No tool completed for the edit call after deny
      const completedEdit = client.messages.some(
        (m) =>
          m.method === "session/update" &&
          m.params?.update?.toolCallId === FIXED.toolCallEdit &&
          m.params?.update?.status === "completed",
      );
      assert.equal(completedEdit, false);
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("cancel_mid_stream: stopReason cancelled", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "cancel_mid_stream",
      chunkDelayMs: 20,
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "long" }],
        },
      });
      await client.waitFor(
        (m) =>
          m.method === "session/update" &&
          m.params?.update?.sessionUpdate === "agent_message_chunk",
        5000,
      );
      client.send({
        jsonrpc: "2.0",
        method: "session/cancel",
        params: { sessionId: FIXED.sessionId },
      });
      const result = await client.waitForResponse(10, 5000);
      assert.equal(result.result.stopReason, "cancelled");
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("cancel_while_permission_pending: no deadlock", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "cancel_while_permission_pending",
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "edit" }],
        },
      });
      await client.waitFor((m) => m.method === "session/request_permission", 5000);
      client.send({
        jsonrpc: "2.0",
        method: "session/cancel",
        params: { sessionId: FIXED.sessionId },
      });
      const result = await client.waitForResponse(10, 5000);
      assert.equal(result.result.stopReason, "cancelled");
      const obs = observeMessages(client.messages);
      assert.ok(obs.toolFailed >= 1 || result.result.stopReason === "cancelled");
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("malformed_frame: emits non-JSON line", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "malformed_frame",
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "x" }],
        },
      });
      await client.waitForResponse(10, 5000);
      assert.ok(client.malformedFrames >= 1);
      const obs = observeMessages(client.messages);
      assert.ok(obs.malformedFrames >= 1);
      const product = mapWireObservationToProductTypes(obs);
      assert.ok(product.includes("adapter.protocol.error"));
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("unknown_vendor_notification: session continues", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "unknown_vendor_notification",
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "x" }],
        },
      });
      await client.waitFor(
        (m) => m.method === "x.ai/unknown_vendor_extension",
        5000,
      );
      const result = await client.waitForResponse(10, 5000);
      assert.equal(result.result.stopReason, "end_turn");
      const product = mapWireObservationToProductTypes(observeMessages(client.messages));
      assert.ok(product.includes("adapter.protocol.unknown"));
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("eof_mid_prompt: closes without successful completion result", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "eof_mid_prompt",
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "x" }],
        },
      });
      const code = await client.exitPromise;
      assert.equal(code, 0);
      const hasPromptResult = client.messages.some(
        (m) => m.id === 10 && m.result?.stopReason === "end_turn",
      );
      assert.equal(hasPromptResult, false);
      assert.ok(
        client.messages.some(
          (m) =>
            m.method === "session/update" &&
            m.params?.update?.sessionUpdate === "agent_message_chunk",
        ),
      );
    } finally {
      client.closeStdin();
    }
  });

  it("crash_nonzero_exit: exit code 1 mid-run", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "crash_nonzero_exit",
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "x" }],
        },
      });
      const code = await client.exitPromise;
      assert.equal(code, 1);
      const hasComplete = client.messages.some(
        (m) => m.id === 10 && m.result?.stopReason === "end_turn",
      );
      assert.equal(hasComplete, false);
    } finally {
      client.closeStdin();
    }
  });

  it("cancel_unsupported: ignores cancel; still streaming until natural end if not killed", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "cancel_unsupported",
      chunkDelayMs: 5,
    });
    try {
      const { init } = await handshake(client);
      const caps = init.result.agentCapabilities._meta["tracer/capabilities"];
      assert.equal(caps.cancellation, false);

      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "x" }],
        },
      });
      await client.waitFor(
        (m) =>
          m.method === "session/update" &&
          m.params?.update?.sessionUpdate === "agent_message_chunk",
        5000,
      );
      client.send({
        jsonrpc: "2.0",
        method: "session/cancel",
        params: { sessionId: FIXED.sessionId },
      });
      // Cooperative cancel ignored — may still complete end_turn
      const result = await client.waitForResponse(10, 10000);
      assert.equal(result.result.stopReason, "end_turn");
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("slow_cancel_ack: delayed cancel still eventually cancels when delay short", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "slow_cancel_ack",
      chunkDelayMs: 20,
      cancelDelayMs: 80,
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "x" }],
        },
      });
      await client.waitFor(
        (m) =>
          m.method === "session/update" &&
          m.params?.update?.sessionUpdate === "agent_message_chunk",
        5000,
      );
      client.send({
        jsonrpc: "2.0",
        method: "session/cancel",
        params: { sessionId: FIXED.sessionId },
      });
      const result = await client.waitForResponse(10, 5000);
      assert.equal(result.result.stopReason, "cancelled");
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("duplicate_response_id: two results for same id", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "duplicate_response_id",
    });
    try {
      await handshake(client);
      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "x" }],
        },
      });
      // wait until we have two responses with id 10
      await client.waitFor(
        () => client.messages.filter((m) => m.id === 10 && m.result).length >= 2,
        5000,
      );
      const obs = observeMessages(client.messages);
      assert.ok(obs.duplicateResponseIds >= 1);
      const product = mapWireObservationToProductTypes(obs);
      assert.ok(product.includes("adapter.protocol.error"));
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("capability_minimal: no tools/plans; single message path", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "capability_minimal",
    });
    try {
      const { init } = await handshake(client);
      const caps = init.result.agentCapabilities._meta["tracer/capabilities"];
      assert.equal(caps.promptStreaming, false);
      assert.equal(caps.toolCalls, false);
      assert.equal(caps.planUpdates, false);

      client.send({
        jsonrpc: "2.0",
        id: 10,
        method: "session/prompt",
        params: {
          sessionId: FIXED.sessionId,
          prompt: [{ type: "text", text: "hi" }],
        },
      });
      const result = await client.waitForResponse(10, 5000);
      assert.equal(result.result.stopReason, "end_turn");
      const obs = observeMessages(client.messages);
      assert.equal(obs.toolCalls, 0);
      assert.ok(obs.agentMessageChunks >= 1);
    } finally {
      client.closeStdin();
      await client.exitPromise;
    }
  });

  it("clean_shutdown_stdin_close: exit 0 after stdin EOF", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "clean_shutdown_stdin_close",
    });
    try {
      await handshake(client);
      client.closeStdin();
      const code = await client.exitPromise;
      assert.equal(code, 0);
    } finally {
      /* already closed */
    }
  });

  it("rejects live-only scenario ids", async () => {
    for (const id of LIVE_ONLY_SCENARIOS) {
      const client = spawnFakeRuntime({ repoRoot: root, scenarioId: id });
      const code = await client.exitPromise;
      assert.equal(code, 2, id);
    }
  });

  it("stderr is logs only (no jsonrpc envelopes)", async () => {
    const client = spawnFakeRuntime({
      repoRoot: root,
      scenarioId: "happy_prompt_stream",
    });
    try {
      await handshake(client);
      client.closeStdin();
      await client.exitPromise;
      for (const line of client.stderrLines) {
        assert.equal(line.trim().startsWith("{"), false, line);
      }
    } finally {
      /* done */
    }
  });
});
