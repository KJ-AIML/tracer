/**
 * Presentation snapshot + command DTOs aligned with control-plane / TAURI_COMMAND_CONTRACT_V1.
 * React consumes only these typed shapes — never raw ACP, never SQLite rows.
 */

import type { SessionStatus } from "@tracer/event-types";
import type { RuntimeObservation } from "@tracer/ui";

/** Snapshot schema version (control plane SNAPSHOT_VERSION). */
export const PRESENTATION_SNAPSHOT_VERSION = 1 as const;

/** Auth state from Event Protocol / control plane (orthogonal to session status). */
export type AuthenticationState =
  | "not_required"
  | "unauthenticated"
  | "in_progress"
  | "authenticated"
  | "failed"
  | "expired";

/** Pending approval view (typed; no raw ACP). */
export interface PendingApprovalView {
  approvalId: string;
  sessionId: string;
  action: string;
  description: string;
  risk: string;
  createdAt: string;
}

/** Read-only Heli projection — missing is non-fatal. */
export interface HeliStatusView {
  available: boolean;
  workspaceRoot?: string | null;
  mode?: string | null;
  summary: string;
  warnings: string[];
}

/** Structured last-error payload from control plane (presentation only). */
export interface SnapshotLastError {
  errorClass?: string;
  message?: string;
  [key: string]: unknown;
}

/**
 * Versioned presentation snapshot for the shell.
 * Shell restores from this when live events are missed.
 */
export interface PresentationSnapshot {
  version: number;
  activeProjectId: string | null;
  activeSessionId: string | null;
  sessionStatus: SessionStatus | null;
  /**
   * Control-plane runtime observation string.
   * Mapped to UI RuntimeObservation via mapRuntimeObservation().
   */
  runtimeObservation: string;
  authState: AuthenticationState;
  pendingApprovals: PendingApprovalView[];
  heli: HeliStatusView;
  lastError: SnapshotLastError | null;
  capabilities: Record<string, unknown> | null;
  latestSequence: number;
  promptInFlight: boolean;
}

export function emptyPresentationSnapshot(
  overrides: Partial<PresentationSnapshot> = {},
): PresentationSnapshot {
  return {
    version: PRESENTATION_SNAPSHOT_VERSION,
    activeProjectId: null,
    activeSessionId: null,
    sessionStatus: null,
    runtimeObservation: "unknown",
    authState: "not_required",
    pendingApprovals: [],
    heli: {
      available: false,
      workspaceRoot: null,
      mode: null,
      summary: "not probed",
      warnings: [],
    },
    lastError: null,
    capabilities: null,
    latestSequence: 0,
    promptInFlight: false,
    ...overrides,
  };
}

/**
 * Map control-plane runtimeObservation (+ auth) → UI RuntimePill observation.
 * Control plane emits process/session gate strings; UI has a fixed catalog.
 */
export function mapRuntimeObservation(
  observation: string,
  authState: AuthenticationState = "not_required",
): RuntimeObservation {
  if (
    authState === "unauthenticated" ||
    authState === "in_progress" ||
    authState === "failed" ||
    authState === "expired"
  ) {
    return "sign_in_required";
  }

  switch (observation) {
    case "ready":
    case "protocol_ready":
    case "running":
    case "awaiting_approval":
    case "cancelling":
      return "ready";
    case "starting":
      return "starting";
    case "stopped":
    case "completed":
      return "stopped";
    case "disconnected":
    case "crashed":
    case "failed":
      return "crashed";
    case "unavailable":
      return "unavailable";
    case "not_started":
    case "unknown":
    default:
      return "not_started";
  }
}

/** Whether auth blocks prompting (STATE_MATRIX §5). */
export function isAuthBlocking(auth: AuthenticationState): boolean {
  return (
    auth === "unauthenticated" ||
    auth === "in_progress" ||
    auth === "failed" ||
    auth === "expired"
  );
}

/**
 * Map command errorClass → global banner kind or session presentation cues.
 */
export type NormalizedFailureKind =
  | "runtime_missing"
  | "runtime_crashed"
  | "runtime_disconnected"
  | "authentication_required"
  | "authentication_failed"
  | "storage_error"
  | "control_plane_down"
  | "invalid_state"
  | "generic_failed";

export function mapErrorClassToFailure(errorClass: string): NormalizedFailureKind {
  switch (errorClass) {
    case "RuntimeExecutableNotFound":
    case "RuntimeSpawnFailed":
      return "runtime_missing";
    case "RuntimeCrashed":
      return "runtime_crashed";
    case "RuntimeDisconnected":
    case "RuntimeNotReady":
      return "runtime_disconnected";
    case "AuthenticationRequired":
      return "authentication_required";
    case "AuthenticationFailed":
      return "authentication_failed";
    case "StorageError":
      return "storage_error";
    case "InvalidState":
      return "invalid_state";
    case "InternalError":
    case "Unsupported":
      return "control_plane_down";
    default:
      return "generic_failed";
  }
}
