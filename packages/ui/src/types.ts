/**
 * Session statuses from W0-A catalog / STATE_MATRIX.
 * Temporary UI copy until @tracer/event-types (W1-B) is available.
 * REPLACE_WHEN_W1B_EVENT_TYPES_AVAILABLE
 */
export type SessionStatus =
  | "creating"
  | "starting_runtime"
  | "ready"
  | "running"
  | "awaiting_approval"
  | "cancelling"
  | "completed"
  | "failed"
  | "disconnected"
  | "stopped";

/**
 * Runtime process observations for RuntimePill (SESSION_SCREEN_SPEC §3.3).
 * REPLACE_WHEN_W1B_EVENT_TYPES_AVAILABLE
 */
export type RuntimeObservation =
  | "not_started"
  | "starting"
  | "ready"
  | "sign_in_required"
  | "stopped"
  | "crashed"
  | "unavailable";

export type ColorRole = "neutral" | "info" | "success" | "warning" | "danger";

/**
 * Product presentation containers (STATE_MATRIX §12 shorthand).
 */
export type PresentationKind =
  | "empty"
  | "loading"
  | "running"
  | "failed"
  | "disconnected"
  | "completed"
  | "cancelled"
  | "approval";
