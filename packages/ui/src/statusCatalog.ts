import type { ColorRole, RuntimeObservation, SessionStatus } from "./types";

export interface StatusPresentation {
  label: string;
  iconHint: string;
  colorRole: ColorRole;
}

/** Normative StatusChip labels (SESSION_SCREEN_SPEC §3.2). Status is never color-only. */
export const SESSION_STATUS_PRESENTATION: Record<SessionStatus, StatusPresentation> = {
  creating: { label: "Creating session", iconHint: "spinner", colorRole: "neutral" },
  starting_runtime: { label: "Starting runtime", iconHint: "spinner", colorRole: "neutral" },
  ready: { label: "Ready", iconHint: "check", colorRole: "success" },
  running: { label: "Running", iconHint: "activity", colorRole: "info" },
  awaiting_approval: { label: "Waiting for approval", iconHint: "shield", colorRole: "warning" },
  cancelling: { label: "Cancelling", iconHint: "spinner", colorRole: "warning" },
  completed: { label: "Completed", iconHint: "check-circle", colorRole: "success" },
  failed: { label: "Failed", iconHint: "error", colorRole: "danger" },
  disconnected: { label: "Disconnected", iconHint: "unlink", colorRole: "danger" },
  stopped: { label: "Stopped", iconHint: "stop", colorRole: "neutral" },
};

export interface RuntimePresentation {
  label: string;
  iconHint: string;
  colorRole: ColorRole;
}

/** RuntimePill labels (SESSION_SCREEN_SPEC §3.3). */
export const RUNTIME_OBSERVATION_PRESENTATION: Record<RuntimeObservation, RuntimePresentation> = {
  not_started: { label: "Runtime: not started", iconHint: "plug", colorRole: "neutral" },
  starting: { label: "Runtime: starting", iconHint: "spinner", colorRole: "neutral" },
  ready: { label: "Runtime: ready", iconHint: "check", colorRole: "success" },
  sign_in_required: { label: "Runtime: sign-in required", iconHint: "shield", colorRole: "warning" },
  stopped: { label: "Runtime: stopped", iconHint: "stop", colorRole: "neutral" },
  crashed: { label: "Runtime: crashed", iconHint: "error", colorRole: "danger" },
  unavailable: { label: "Runtime: unavailable", iconHint: "error", colorRole: "danger" },
};

export function getSessionStatusPresentation(status: SessionStatus): StatusPresentation {
  return SESSION_STATUS_PRESENTATION[status];
}

export function getRuntimeObservationPresentation(
  observation: RuntimeObservation,
): RuntimePresentation {
  return RUNTIME_OBSERVATION_PRESENTATION[observation];
}
