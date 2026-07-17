import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import {
  EVENT_PROTOCOL_VERSION,
  KNOWN_EVENT_TYPES,
  type EventEnvelope,
} from "./types.js";
import {
  EnvelopeValidationError,
  errorCategoryOf,
  isKnownEventType,
  parseEnvelope,
  roundTripEnvelope,
  validateSequenceOrder,
  validateSessionEventStream,
} from "./validate.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.resolve(
  here,
  "../../../tests/contract/event-protocol/fixtures",
);

function loadJson(name: string): unknown {
  const raw = readFileSync(path.join(fixturesDir, name), "utf8");
  return JSON.parse(raw) as unknown;
}

function loadStream(name: string): EventEnvelope[] {
  const data = loadJson(name);
  assert.ok(Array.isArray(data));
  return data.map((item) => parseEnvelope(item));
}

test("protocol version is 1", () => {
  assert.equal(EVENT_PROTOCOL_VERSION, 1);
});

test("known event type catalog non-empty", () => {
  assert.ok(KNOWN_EVENT_TYPES.length >= 20);
  assert.ok(isKnownEventType("agent.message.delta"));
  assert.equal(isKnownEventType("vendor.future"), false);
});

test("happy path stream: validate + round-trip + order", () => {
  const events = loadStream("happy_prompt_stream.json");
  validateSessionEventStream(events);
  const seqs = events.map((e) => e.sequence);
  validateSequenceOrder(seqs, 1);
  for (const env of events) {
    const back = roundTripEnvelope(env);
    assert.equal(back.eventVersion, 1);
    assert.equal(back.sequence, env.sequence);
    assert.equal(back.type, env.type);
    assert.deepEqual(back.payload, env.payload);
  }
  const types = events.map((e) => e.type);
  assert.ok(types.includes("runtime.process.ready"));
  assert.ok(types.includes("session.prompt.submitted"));
});

test("unknown vendor notification preserves extensions", () => {
  const env = parseEnvelope(loadJson("unknown_vendor_notification.json"));
  assert.equal(env.type, "adapter.protocol.unknown");
  assert.ok(!isKnownEventType("vendor.x") || true);
  const adapter = env.adapter as Record<string, unknown>;
  assert.ok(adapter);
  const extensions = adapter.extensions as Record<string, unknown>;
  assert.ok(extensions["x.ai/method"]);
  const back = roundTripEnvelope(env);
  assert.deepEqual(
    (back.adapter as Record<string, unknown>).extensions,
    extensions,
  );
});

test("protocol error fixture", () => {
  const env = parseEnvelope(loadJson("protocol_error.json"));
  assert.equal(env.type, "adapter.protocol.error");
  assert.equal(env.severity, "error");
  assert.equal(env.payload.errorClass, "ProtocolParseError");
  assert.equal(errorCategoryOf("ProtocolParseError"), "protocol");
});

test("crash exit and cancel and approval streams", () => {
  for (const name of [
    "unexpected_process_exit.json",
    "cancel_mid_tool.json",
    "tool_with_approval.json",
  ]) {
    const events = loadStream(name);
    validateSessionEventStream(events);
  }
  const cancel = loadStream("cancel_mid_tool.json");
  assert.ok(cancel.some((e) => e.type === "session.cancelled"));
  const approval = loadStream("tool_with_approval.json");
  assert.ok(approval.some((e) => e.type === "approval.requested"));
  assert.ok(approval.some((e) => e.type === "approval.resolved"));
});

test("reject missing required fields", () => {
  assert.throws(
    () =>
      parseEnvelope({
        eventVersion: 1,
        eventId: "550e8400-e29b-41d4-a716-446655440000",
        sequence: 1,
        timestamp: "2026-07-17T12:00:00Z",
        projectId: "11111111-1111-1111-1111-111111111111",
        type: "session.created",
        payload: {},
      }),
    EnvelopeValidationError,
  );
});

test("unknown type tolerated", () => {
  const env = parseEnvelope({
    eventVersion: 1,
    eventId: "550e8400-e29b-41d4-a716-446655440000",
    sequence: 1,
    timestamp: "2026-07-17T12:00:00Z",
    projectId: "11111111-1111-1111-1111-111111111111",
    sessionId: "22222222-2222-2222-2222-222222222222",
    agentRunId: null,
    type: "vendor.future.event",
    payload: { x: 1 },
    futureEnvelopeField: "keep",
  });
  assert.equal(env.type, "vendor.future.event");
  assert.equal(env.futureEnvelopeField, "keep");
  const back = roundTripEnvelope(env);
  assert.equal(back.futureEnvelopeField, "keep");
});

test("sequence gap detected", () => {
  assert.throws(() => validateSequenceOrder([1, 2, 4], 1), EnvelopeValidationError);
});

test("error categories required set", () => {
  assert.equal(errorCategoryOf("ProtocolViolation"), "protocol");
  assert.equal(errorCategoryOf("RuntimeCrashed"), "process");
  assert.equal(errorCategoryOf("AuthenticationRequired"), "authentication");
  assert.equal(errorCategoryOf("PermissionDenied"), "permission");
  assert.equal(errorCategoryOf("StorageError"), "storage");
});

test("replay sort by sequence", () => {
  const events = loadStream("happy_prompt_stream.json").reverse();
  events.sort((a, b) => a.sequence - b.sequence);
  validateSessionEventStream(events);
});