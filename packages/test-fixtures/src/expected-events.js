import fs from "node:fs";
import { expectedEventsPath, findRepoRoot } from "./paths.js";
import { isValidEvidence } from "./provenance.js";

/** W0-B conceptual names forbidden as product types (TEST_STRATEGY / expected packs). */
export const FORBIDDEN_PRODUCT_TYPE_ALIASES = Object.freeze([
  "message.agent.delta",
  "permission.requested",
  "turn.started",
  "runtime.initialized",
]);

/** Normative W0-A type strings commonly asserted. */
export const NORMATIVE_EVENT_TYPES = Object.freeze([
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
  "adapter.protocol.error",
  "adapter.protocol.unknown",
  "storage.error",
]);

export function loadExpectedEvents(scenarioId, repoRoot = findRepoRoot()) {
  const file = expectedEventsPath(scenarioId, repoRoot);
  if (!fs.existsSync(file)) {
    throw new Error(`Missing expected-events pack: ${file}`);
  }
  const pack = JSON.parse(fs.readFileSync(file, "utf8"));
  if (pack.schemaVersion !== 1) {
    throw new Error(`${scenarioId}: unsupported schemaVersion ${pack.schemaVersion}`);
  }
  if (pack.scenarioId !== scenarioId) {
    throw new Error(
      `${scenarioId}: pack scenarioId mismatch (${pack.scenarioId})`,
    );
  }
  if (pack.evidence && !isValidEvidence(pack.evidence)) {
    throw new Error(`${scenarioId}: invalid evidence ${pack.evidence}`);
  }
  return pack;
}

/**
 * Extract type strings mentioned in orderedTypeConstraints (for documentation / light checks).
 */
export function collectConstraintTypes(pack) {
  const types = new Set();
  function walk(node) {
    if (!node) return;
    if (Array.isArray(node)) {
      node.forEach(walk);
      return;
    }
    if (typeof node !== "object") return;
    if (typeof node.type === "string") types.add(node.type);
    if (node.anyOf) walk(node.anyOf);
    if (node.orderedTypeConstraints) walk(node.orderedTypeConstraints);
    if (node.expectEvents) walk(node.expectEvents);
    if (node.preconditionEvents) walk(node.preconditionEvents);
    if (node.fallback?.expectEvents) walk(node.fallback.expectEvents);
    if (node.optionalFollowOn?.ifTransportDead) walk(node.optionalFollowOn.ifTransportDead);
  }
  walk(pack.orderedTypeConstraints);
  walk(pack.preconditionEvents);
  if (pack.fallback) walk(pack.fallback);
  if (pack.optionalFollowOn) walk(pack.optionalFollowOn);
  return [...types];
}

/**
 * Assert pack does not treat forbidden aliases as required product types.
 */
export function assertNormativeNamesOnly(pack) {
  const types = collectConstraintTypes(pack);
  const forbidden = pack.forbiddenProductTypeAliases || FORBIDDEN_PRODUCT_TYPE_ALIASES;
  const bad = types.filter((t) => forbidden.includes(t));
  if (bad.length) {
    throw new Error(
      `${pack.scenarioId}: expected-events uses forbidden product aliases: ${bad.join(", ")}`,
    );
  }
  if (pack.assertNormativeNamesOnly === false) return;
  for (const t of types) {
    if (FORBIDDEN_PRODUCT_TYPE_ALIASES.includes(t)) {
      throw new Error(`${pack.scenarioId}: forbidden alias ${t}`);
    }
  }
}

/**
 * Map ACP wire observations → suggested W0-A product types for harness notes.
 * Control plane / adapter (W1-B/D/F) perform real normalization; this is a test aid only.
 */
export function mapWireObservationToProductTypes(obs) {
  /** @type {string[]} */
  const types = [];
  if (obs.initializeOk) types.push("runtime.process.ready");
  if (obs.sessionNewOk) types.push("session.ready");
  if (obs.sessionNewAuthError) {
    /* process ready allowed; session.ready forbidden */
  }
  if (obs.promptSubmitted) types.push("session.prompt.submitted");
  if (obs.agentMessageChunks > 0) types.push("agent.message.delta");
  if (obs.toolCalls > 0) types.push("tool.started");
  if (obs.toolCompleted > 0) types.push("tool.completed");
  if (obs.toolFailed > 0) types.push("tool.failed");
  if (obs.permissionRequests > 0) types.push("approval.requested");
  if (obs.permissionResolved) types.push("approval.resolved");
  if (obs.malformedFrames > 0) types.push("adapter.protocol.error");
  if (obs.unknownVendor > 0) types.push("adapter.protocol.unknown");
  if (obs.duplicateResponseIds > 0) types.push("adapter.protocol.error");
  if (obs.promptStopReason === "cancelled") types.push("session.cancelled");
  if (obs.promptStopReason === "end_turn" && !obs.sessionNewAuthError) {
    types.push("session.completed");
  }
  if (obs.exitCode != null && obs.exitCode !== 0) {
    types.push("runtime.process.exited");
    types.push("runtime.process.failed");
  } else if (obs.exited) {
    types.push("runtime.process.exited");
  }
  if (obs.eofWithoutPromptResult) {
    types.push("runtime.process.exited");
    types.push("session.failed");
  }
  return types;
}
