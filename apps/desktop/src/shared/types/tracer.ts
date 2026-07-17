/**
 * Shell-facing types for Tracer (W1-A + W1.1 domain adoption).
 *
 * Session status and event envelope subsets come from `@tracer/event-types` (W1-B).
 * Presentation kinds remain UI-owned (`@tracer/ui`).
 * REPLACE_WHEN_W1F_CONTROL_PLANE_AVAILABLE — wire invoke + tracer://events.
 *
 * No raw vendor-event / ACP parsing lives here (ADR-002).
 */

export type {
  PresentationKind,
  RuntimeObservation,
} from "@tracer/ui";

export type { SessionStatus } from "@tracer/event-types";

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
  status: import("@tracer/event-types").SessionStatus;
  createdAt: string;
  updatedAt: string;
}

/**
 * Minimal normalized event envelope (protocol v1 subset for mock store).
 * Canonical full envelope lives in `@tracer/event-types`; this is the mock-store view.
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
