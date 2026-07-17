import { describe, expect, it } from "vitest";
import {
  getRuntimeObservationPresentation,
  getSessionStatusPresentation,
  SESSION_STATUS_PRESENTATION,
} from "./statusCatalog";
import type { SessionStatus } from "./types";

const ALL_STATUSES: SessionStatus[] = [
  "creating",
  "starting_runtime",
  "ready",
  "running",
  "awaiting_approval",
  "cancelling",
  "completed",
  "failed",
  "disconnected",
  "stopped",
];

describe("session status presentation (a11y: not color-only)", () => {
  it("covers every W0-A session status with a non-empty text label", () => {
    for (const status of ALL_STATUSES) {
      const p = getSessionStatusPresentation(status);
      expect(p.label.trim().length).toBeGreaterThan(0);
      expect(p.iconHint.trim().length).toBeGreaterThan(0);
      expect(p.colorRole).toBeTruthy();
    }
  });

  it("uses normative English labels from SESSION_SCREEN_SPEC", () => {
    expect(SESSION_STATUS_PRESENTATION.running.label).toBe("Running");
    expect(SESSION_STATUS_PRESENTATION.awaiting_approval.label).toBe("Waiting for approval");
    expect(SESSION_STATUS_PRESENTATION.disconnected.label).toBe("Disconnected");
    expect(SESSION_STATUS_PRESENTATION.completed.label).toBe("Completed");
    expect(SESSION_STATUS_PRESENTATION.stopped.label).toBe("Stopped");
  });

  it("runtime pill labels always include Runtime: prefix text", () => {
    const ready = getRuntimeObservationPresentation("ready");
    expect(ready.label).toMatch(/Runtime:/);
    const crashed = getRuntimeObservationPresentation("crashed");
    expect(crashed.label).toMatch(/Runtime:/);
  });
});
