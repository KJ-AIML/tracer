/**
 * Tracer Event Protocol v1 — TypeScript surface (W1-B).
 * Normative: docs/contracts/TRACER_EVENT_PROTOCOL_V1.md
 */

export const EVENT_PROTOCOL_VERSION = 1 as const;

export type Severity = "info" | "warn" | "error";

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

export const SESSION_STATUSES: readonly SessionStatus[] = [
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
] as const;

export type AuthenticationState =
  | "not_required"
  | "unauthenticated"
  | "in_progress"
  | "authenticated"
  | "failed"
  | "expired";

export type ErrorCategory =
  | "protocol"
  | "process"
  | "authentication"
  | "permission"
  | "storage"
  | "internal"
  | "capability"
  | "operation";

export type ErrorClass =
  | "RuntimeExecutableNotFound"
  | "RuntimeSpawnFailed"
  | "RuntimeNotReady"
  | "RuntimeDisconnected"
  | "RuntimeCrashed"
  | "ProtocolInitializeFailed"
  | "ProtocolParseError"
  | "ProtocolViolation"
  | "CapabilityMismatch"
  | "CapabilityUnsupported"
  | "SessionNotFound"
  | "PromptRejected"
  | "CancellationFailed"
  | "Timeout"
  | "InvalidState"
  | "InvalidArgument"
  | "ApprovalUnknown"
  | "PermissionDenied"
  | "AuthenticationRequired"
  | "AuthenticationFailed"
  | "StorageError"
  | "InternalAdapterError";

export const KNOWN_EVENT_TYPES = [
  "runtime.process.started",
  "runtime.process.ready",
  "runtime.process.stderr",
  "runtime.process.exited",
  "runtime.process.failed",
  "session.created",
  "session.ready",
  "session.prompt.submitted",
  "session.status.changed",
  "session.completed",
  "session.failed",
  "session.cancelled",
  "agent.message.delta",
  "agent.message.completed",
  "agent.progress.delta",
  "agent.plan.updated",
  "tool.started",
  "tool.updated",
  "tool.completed",
  "tool.failed",
  "approval.requested",
  "approval.resolved",
  "file.changed",
  "file.diff.available",
  "terminal.output",
  "terminal.exited",
  "storage.error",
  "adapter.protocol.error",
  "adapter.protocol.unknown",
] as const;

export type KnownEventType = (typeof KNOWN_EVENT_TYPES)[number];

/** Known catalog type or any forward-compatible string. */
export type EventType = KnownEventType | (string & {});

export interface Capabilities {
  promptStreaming: boolean;
  cancellation: boolean;
  planUpdates: boolean;
  toolCalls: boolean;
  approvals: boolean;
  fileChangeNotifications: boolean;
  terminalOutput: boolean;
  sessionResume: boolean;
  /** Vendor keys preserved for debugging — not for product branching. */
  unknown?: Record<string, unknown>;
}

export interface AdapterMetadata {
  runtimeKind?: string;
  runtimeSessionId?: string;
  rawRef?: string;
  rawFragment?: unknown;
  runtimeMethod?: string;
  /** Unknown vendor metadata preservation. */
  extensions?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface EventEnvelope {
  eventVersion: number;
  eventId: string;
  sequence: number;
  timestamp: string;
  projectId: string;
  sessionId: string;
  agentRunId: string | null;
  type: EventType;
  payload: Record<string, unknown>;
  adapter?: AdapterMetadata | null;
  severity?: Severity;
  /** Forward-compatible unknown root fields. */
  [key: string]: unknown;
}

export interface TracerErrorShape {
  errorClass: ErrorClass | string;
  message: string;
  retryable: boolean;
  details?: Record<string, unknown>;
  category?: ErrorCategory;
}