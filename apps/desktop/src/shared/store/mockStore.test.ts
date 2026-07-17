/**
 * Legacy mockStore helpers retained for presentation matrix pure functions.
 * Journey ownership moved to snapshotStore (VS1-H2).
 */
import { describe, expect, it } from "vitest";
import {
  composerDisabledReason,
  createInitialMockState,
  isComposerEnabled,
  mockReducer,
} from "./mockStore";

describe("mockStore (compat smoke)", () => {
  it("starts on projects with demo project and ready session", () => {
    const s = createInitialMockState();
    expect(s.route.name).toBe("projects");
    expect(s.projects.length).toBe(1);
    expect(s.activeSessionStatus).toBe("ready");
    expect(isComposerEnabled(s.activeSessionStatus, s.runtimeObservation)).toBe(true);
  });

  it("disables composer with visible reasons for non-ready statuses", () => {
    expect(composerDisabledReason("running", "ready")).toBe("Agent is working…");
    expect(composerDisabledReason("awaiting_approval", "ready")).toBe("Approval required");
    expect(composerDisabledReason("disconnected", "crashed")).toMatch(/Runtime/);
    expect(isComposerEnabled("running", "ready")).toBe(false);
  });

  it("never leaves running after disconnect simulation", () => {
    let s = createInitialMockState();
    s = mockReducer(s, { type: "simulatePromptSubmit" });
    expect(s.activeSessionStatus).toBe("running");
    s = mockReducer(s, { type: "simulateDisconnect" });
    expect(s.activeSessionStatus).toBe("disconnected");
    expect(s.runtimeObservation).toBe("crashed");
    expect(isComposerEnabled(s.activeSessionStatus, s.runtimeObservation)).toBe(false);
  });

  it("records only normalized event type strings (no ACP methods)", () => {
    let s = createInitialMockState();
    s = mockReducer(s, { type: "setComposerText", text: "hello" });
    s = mockReducer(s, { type: "simulatePromptSubmit" });
    s = mockReducer(s, { type: "simulateApproval" });
    for (const e of s.events) {
      expect(e.type.includes("/")).toBe(false);
      expect(e.type.startsWith("session.") || e.type.startsWith("approval.")).toBe(true);
    }
  });
});