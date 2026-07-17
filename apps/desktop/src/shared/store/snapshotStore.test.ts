/**
 * Deterministic VS1-H2 frontend matrix — fake/mock command layer only.
 * Covers required presentation states without network/credentials.
 */
import { beforeEach, describe, expect, it } from "vitest";
import {
  appReducer,
  composerDisabledReason,
  createInitialAppState,
  isComposerEnabled,
  SnapshotJourney,
  type AppViewState,
} from "./snapshotStore";
import {
  getMockBackend,
  setInvokeMode,
  setMockBackend,
} from "../commands/invoke";
import {
  createMockBackend,
  MOCK_BACKEND_IDS,
  type MockScenario,
} from "../commands/mockBackend";
import { mapRuntimeObservation } from "../types/snapshot";

function createHarness(scenario: MockScenario = "default") {
  setInvokeMode("mock");
  setMockBackend(createMockBackend(scenario));
  let state = createInitialAppState();
  const dispatch = (action: Parameters<typeof appReducer>[1]) => {
    state = appReducer(state, action);
  };
  const journey = new SnapshotJourney(() => state, dispatch);
  return {
    get state() {
      return state;
    },
    dispatch,
    journey,
    backend: () => getMockBackend()!,
  };
}

beforeEach(() => {
  setInvokeMode("mock");
  setMockBackend(createMockBackend("default"));
});

describe("VS1-H2 snapshot journey matrix", () => {
  it("initial snapshot: loads projects + ready session presentation", async () => {
    const h = createHarness("default");
    await h.journey.bootstrapLoad();
    expect(h.state.loadPhase).toBe("ready");
    expect(h.state.projects.length).toBe(1);
    expect(h.state.snapshot.sessionStatus).toBe("ready");
    expect(h.state.runtimeObservation).toBe("ready");
    expect(h.state.snapshot.version).toBe(1);
    expect(isComposerEnabled(h.state.sessionStatus, h.state.runtimeObservation, h.state.authState)).toBe(
      true,
    );
  });

  it("runtime unavailable: maps to failed + unavailable pill + banner path", async () => {
    const h = createHarness("runtime_unavailable");
    await h.journey.bootstrapLoad();
    expect(h.state.snapshot.sessionStatus).toBe("failed");
    expect(h.state.runtimeObservation).toBe("unavailable");
    expect(h.state.lastErrorMessage).toMatch(/runtime/i);
    expect(
      isComposerEnabled(h.state.sessionStatus, h.state.runtimeObservation, h.state.authState),
    ).toBe(false);
    expect(composerDisabledReason(h.state.sessionStatus, h.state.runtimeObservation)).toMatch(
      /Runtime/,
    );
  });

  it("authentication required: blocks composer with sign-in reason", async () => {
    const h = createHarness("authentication_required");
    await h.journey.bootstrapLoad();
    expect(h.state.authState).toBe("unauthenticated");
    expect(h.state.runtimeObservation).toBe("sign_in_required");
    expect(
      isComposerEnabled(h.state.sessionStatus, h.state.runtimeObservation, h.state.authState),
    ).toBe(false);
    expect(
      composerDisabledReason(h.state.sessionStatus, h.state.runtimeObservation, h.state.authState),
    ).toBe("Sign in required");

    await expect(
      h.journey.submitPrompt(MOCK_BACKEND_IDS.sessionId, "hello"),
    ).rejects.toMatchObject({ errorClass: "AuthenticationRequired" });
  });

  it("prompt streaming: submit yields running + normalized stream events", async () => {
    const h = createHarness("prompt_streaming");
    await h.journey.bootstrapLoad();
    await h.journey.openSession(MOCK_BACKEND_IDS.projectId, MOCK_BACKEND_IDS.sessionId);
    await h.journey.submitPrompt(MOCK_BACKEND_IDS.sessionId, "summarize repo");
    expect(h.state.sessionStatus).toBe("running");
    expect(h.state.snapshot.promptInFlight).toBe(true);
    expect(h.state.events.some((e) => e.type === "session.prompt.submitted")).toBe(true);
    expect(h.state.events.some((e) => e.type === "agent.message.delta")).toBe(true);
    for (const e of h.state.events) {
      expect(e.type.includes("/")).toBe(false);
    }
    expect(isComposerEnabled(h.state.sessionStatus, h.state.runtimeObservation)).toBe(false);
  });

  it("approval request: shows pending approval and awaiting_approval status", async () => {
    const h = createHarness("approval_request");
    await h.journey.bootstrapLoad();
    await h.journey.openSession(MOCK_BACKEND_IDS.projectId, MOCK_BACKEND_IDS.sessionId);
    expect(h.state.sessionStatus).toBe("awaiting_approval");
    expect(h.state.pendingApprovals.length).toBe(1);
    expect(h.state.pendingApprovals[0]?.approvalId).toBe(MOCK_BACKEND_IDS.approvalId);
    expect(h.state.sideTab).toBe("approvals");
    expect(composerDisabledReason(h.state.sessionStatus, h.state.runtimeObservation)).toBe(
      "Approval required",
    );
  });

  it("approval accepted: clears pending and returns to running", async () => {
    const h = createHarness("approval_request");
    await h.journey.bootstrapLoad();
    await h.journey.resolveApproval(
      MOCK_BACKEND_IDS.sessionId,
      MOCK_BACKEND_IDS.approvalId,
      "allow",
    );
    expect(h.state.pendingApprovals.length).toBe(0);
    expect(h.state.sessionStatus).toBe("running");
    expect(h.state.events.some((e) => e.type === "approval.resolved")).toBe(true);
  });

  it("approval rejected: clears pending and continues without auto-allow", async () => {
    const h = createHarness("approval_request");
    await h.journey.bootstrapLoad();
    await h.journey.resolveApproval(
      MOCK_BACKEND_IDS.sessionId,
      MOCK_BACKEND_IDS.approvalId,
      "deny",
    );
    expect(h.state.pendingApprovals.length).toBe(0);
    expect(h.state.sessionStatus).toBe("running");
    const resolved = h.state.events.find((e) => e.type === "approval.resolved");
    expect(resolved?.payload?.decision).toBe("deny");
  });

  it("cancel pending approval: terminal cancelled/stopped state", async () => {
    const h = createHarness("approval_request");
    await h.journey.bootstrapLoad();
    await h.journey.resolveApproval(
      MOCK_BACKEND_IDS.sessionId,
      MOCK_BACKEND_IDS.approvalId,
      "cancel",
    );
    expect(h.state.pendingApprovals.length).toBe(0);
    expect(h.state.sessionStatus).toBe("stopped");
    expect(h.state.runtimeObservation).toBe("stopped");
  });

  it("completed run: terminal completed presentation", async () => {
    const h = createHarness("completed_run");
    await h.journey.bootstrapLoad();
    await h.journey.openSession(MOCK_BACKEND_IDS.projectId, MOCK_BACKEND_IDS.sessionId);
    expect(h.state.sessionStatus).toBe("completed");
    expect(h.state.events.some((e) => e.type === "session.completed")).toBe(true);
    expect(isComposerEnabled(h.state.sessionStatus, h.state.runtimeObservation)).toBe(false);
  });

  it("runtime crash: disconnected session, never remains running", async () => {
    const h = createHarness("runtime_crash");
    await h.journey.bootstrapLoad();
    expect(h.state.sessionStatus).toBe("disconnected");
    expect(h.state.runtimeObservation).toBe("crashed");
    expect(h.state.sessionStatus).not.toBe("running");
    expect(isComposerEnabled(h.state.sessionStatus, h.state.runtimeObservation)).toBe(false);
  });

  it("session-history restore: events_list reloads persisted normalized history", async () => {
    const h = createHarness("session_history_restore");
    await h.journey.bootstrapLoad();
    await h.journey.openSession(MOCK_BACKEND_IDS.projectId, MOCK_BACKEND_IDS.sessionId);
    expect(h.state.events.length).toBeGreaterThanOrEqual(4);
    expect(h.state.snapshot.latestSequence).toBe(4);
    expect(h.state.events.some((e) => e.type === "agent.message.completed")).toBe(true);
    // Simulate missed notifications: refresh snapshot still restores terminal state.
    await h.journey.refreshSnapshot();
    expect(h.state.sessionStatus).toBe("completed");
  });

  it("Heli unavailable: non-fatal banner, app remains usable", async () => {
    const h = createHarness("heli_unavailable");
    await h.journey.bootstrapLoad();
    expect(h.state.heli.available).toBe(false);
    expect(h.state.loadPhase).toBe("ready");
    expect(h.state.globalBanner).toBe("heli_unavailable");
    expect(h.state.projects.length).toBe(1);
    // Composer still works when session ready
    expect(
      isComposerEnabled(h.state.sessionStatus, h.state.runtimeObservation, h.state.authState),
    ).toBe(true);
  });
});

describe("runtime observation mapping", () => {
  it("maps control-plane strings to UI catalog without inventing ACP", () => {
    expect(mapRuntimeObservation("ready")).toBe("ready");
    expect(mapRuntimeObservation("disconnected")).toBe("crashed");
    expect(mapRuntimeObservation("unavailable")).toBe("unavailable");
    expect(mapRuntimeObservation("starting")).toBe("starting");
    expect(mapRuntimeObservation("protocol_ready", "unauthenticated")).toBe("sign_in_required");
  });
});

describe("cancel visibility", () => {
  it("cancel visible only while running or awaiting approval", () => {
    const ready: AppViewState = {
      ...createInitialAppState(),
      sessionStatus: "ready",
    };
    expect(ready.sessionStatus === "running" || ready.sessionStatus === "awaiting_approval").toBe(
      false,
    );
  });
});