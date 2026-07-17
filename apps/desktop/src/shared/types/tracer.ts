/**
 * Temporary typed interfaces for Tracer shell (W1-A).
 *
 * REPLACE_WHEN_W1B_EVENT_TYPES_AVAILABLE — switch to @tracer/event-types.
 * REPLACE_WHEN_W1F_CONTROL_PLANE_AVAILABLE — wire invoke + tracer://events.
 *
 * No raw vendor-event / ACP parsing lives here (ADR-002).
 */

export type {
  PresentationKind,
  RuntimeObservation,
  SessionStatus,
} from "@tracer/ui";

/** Project list item shape from TAURI_COMMAND_CONTRACT_V1 `tracer_project_list`. */
export interface ProjectSummary {
  projectId: string;
  name: string;
  /** User-local runtime data only — never commit machine-specific absolute paths. */
  rootPath: string;
  status: "ready" | "missing" | "invalid";
  lastOpenedAt?: string;
}

/** Session list item from `tracer_session_list`. */
export interface SessionSummary {
  sessionId: string;
  projectId: string;
  title: string;
  status: import("@tracer/ui").SessionStatus;
  createdAt: string;
  updatedAt: string;
}

/**
 * Minimal normalized event envelope (protocol v1 subset for mock store).
 * REPLACE_WHEN_W1B_EVENT_TYPES_AVAILABLE
 */
export interface TracerEventEnvelope {
  eventId: string;
  sessionId: string;
  sequence: number;
  type: string;
  timestamp: string;
  /** Opaque payload — UI must not require vendor-specific keys. */
  payload?: Record<string, unknown>;
}

/** Structured command error (TAURI_COMMAND_CONTRACT_V1 §3.2). */
export interface CommandError {
  errorClass: string;
  message: string;
  retryable: boolean;
  details?: Record<string, unknown>;
}

export type RouteKey =
  | { name: "projects" }
  | { name: "project"; projectId: string }
  | { name: "session"; projectId: string; sessionId: string }
  | { name: "about" }
  | { name: "presentation-gallery" };
