import {
  EVENT_PROTOCOL_VERSION,
  type ErrorCategory,
  type ErrorClass,
  type EventEnvelope,
  type SessionStatus,
  KNOWN_EVENT_TYPES,
  SESSION_STATUSES,
} from "./types.js";

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export class EnvelopeValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "EnvelopeValidationError";
  }
}

export function isUuid(value: unknown): value is string {
  return typeof value === "string" && UUID_RE.test(value);
}

export function isKnownEventType(type: string): boolean {
  return (KNOWN_EVENT_TYPES as readonly string[]).includes(type);
}

export function isSessionStatus(value: string): value is SessionStatus {
  return (SESSION_STATUSES as readonly string[]).includes(value);
}

export function isTerminalStatus(status: SessionStatus): boolean {
  return (
    status === "completed" ||
    status === "failed" ||
    status === "disconnected" ||
    status === "stopped"
  );
}

const PROCESS: ErrorClass[] = [
  "RuntimeExecutableNotFound",
  "RuntimeSpawnFailed",
  "RuntimeNotReady",
  "RuntimeDisconnected",
  "RuntimeCrashed",
];
const PROTOCOL: ErrorClass[] = [
  "ProtocolInitializeFailed",
  "ProtocolParseError",
  "ProtocolViolation",
];
const AUTH: ErrorClass[] = ["AuthenticationRequired", "AuthenticationFailed"];
const PERM: ErrorClass[] = ["ApprovalUnknown", "PermissionDenied"];

export function errorCategoryOf(errorClass: string): ErrorCategory {
  if ((PROCESS as string[]).includes(errorClass)) return "process";
  if ((PROTOCOL as string[]).includes(errorClass)) return "protocol";
  if ((AUTH as string[]).includes(errorClass)) return "authentication";
  if ((PERM as string[]).includes(errorClass)) return "permission";
  if (errorClass === "StorageError") return "storage";
  if (
    errorClass === "CapabilityMismatch" ||
    errorClass === "CapabilityUnsupported"
  ) {
    return "capability";
  }
  if (errorClass === "InternalAdapterError" || errorClass === "InvalidArgument") {
    return "internal";
  }
  return "operation";
}

/** Required envelope fields per protocol §2. */
const REQUIRED = [
  "eventVersion",
  "eventId",
  "sequence",
  "timestamp",
  "projectId",
  "sessionId",
  "agentRunId",
  "type",
  "payload",
] as const;

export function parseEnvelope(input: unknown): EventEnvelope {
  if (input === null || typeof input !== "object" || Array.isArray(input)) {
    throw new EnvelopeValidationError("envelope must be a JSON object");
  }
  const obj = input as Record<string, unknown>;
  for (const key of REQUIRED) {
    if (!(key in obj)) {
      throw new EnvelopeValidationError(`missing required field: ${key}`);
    }
  }
  if (obj.eventVersion !== EVENT_PROTOCOL_VERSION) {
    throw new EnvelopeValidationError(
      `unsupported eventVersion: ${String(obj.eventVersion)}`,
    );
  }
  if (!isUuid(obj.eventId)) {
    throw new EnvelopeValidationError("eventId must be a UUID string");
  }
  if (!isUuid(obj.projectId)) {
    throw new EnvelopeValidationError("projectId must be a UUID string");
  }
  if (!isUuid(obj.sessionId)) {
    throw new EnvelopeValidationError("sessionId must be a UUID string");
  }
  if (obj.agentRunId !== null && !isUuid(obj.agentRunId)) {
    throw new EnvelopeValidationError("agentRunId must be UUID or null");
  }
  if (typeof obj.sequence !== "number" || !Number.isInteger(obj.sequence) || obj.sequence < 1) {
    throw new EnvelopeValidationError("sequence must be an integer >= 1");
  }
  if (typeof obj.timestamp !== "string" || obj.timestamp.length === 0) {
    throw new EnvelopeValidationError("timestamp must be a non-empty RFC3339 string");
  }
  if (typeof obj.type !== "string" || obj.type.length === 0) {
    throw new EnvelopeValidationError("type must be a non-empty string");
  }
  if (
    obj.payload === null ||
    typeof obj.payload !== "object" ||
    Array.isArray(obj.payload)
  ) {
    throw new EnvelopeValidationError("payload must be an object");
  }

  // Tolerate unknown fields — return the object as EventEnvelope.
  return obj as EventEnvelope;
}

export function validateSequenceOrder(
  sequences: number[],
  start = 1,
): void {
  if (start < 1) {
    throw new EnvelopeValidationError(`sequence start must be >= 1, got ${start}`);
  }
  let expected = start;
  for (const seq of sequences) {
    if (seq < 1) {
      throw new EnvelopeValidationError(`sequence must be >= 1, got ${seq}`);
    }
    if (seq !== expected) {
      throw new EnvelopeValidationError(
        `sequence gap or reorder: expected ${expected}, got ${seq}`,
      );
    }
    expected += 1;
  }
}

export function validateSessionEventStream(events: EventEnvelope[]): void {
  for (let i = 0; i < events.length; i++) {
    parseEnvelope(events[i]);
    if (i === 0) {
      if (events[i].sequence !== 1) {
        throw new EnvelopeValidationError(
          `first event sequence must be 1, got ${events[i].sequence}`,
        );
      }
    } else {
      if (events[i].sessionId !== events[i - 1].sessionId) {
        throw new EnvelopeValidationError("mixed sessionId in stream");
      }
      if (events[i].sequence !== events[i - 1].sequence + 1) {
        throw new EnvelopeValidationError(
          `sequence gap: expected ${events[i - 1].sequence + 1}, got ${events[i].sequence}`,
        );
      }
    }
  }
}

/** JSON round-trip that preserves unknown fields. */
export function roundTripEnvelope(env: EventEnvelope): EventEnvelope {
  const json = JSON.stringify(env);
  return parseEnvelope(JSON.parse(json) as unknown);
}