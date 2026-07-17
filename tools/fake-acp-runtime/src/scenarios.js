/**
 * Scenario configuration for the deterministic fake ACP runtime.
 * IDs match tests/specifications/scenarios/catalog.yaml (standardCi).
 *
 * Evidence: all standard scenarios here are fake-runtime (or synthetic wire).
 * Live catalog IDs are rejected — never pretend live parity.
 */

import { FIXED, IMPLEMENTED_SCENARIOS } from "./constants.js";

/**
 * @typedef {'full'|'minimal'|'cancel_unsupported'} CapProfile
 * @typedef {'stream_tools'|'permission'|'cancel_stream'|'cancel_permission'|'malformed'|'vendor_unknown'|'eof'|'crash'|'duplicate_id'|'minimal_complete'|'idle_until_eof'} PromptMode
 */

/**
 * @type {Record<string, {
 *   id: string,
 *   evidence: string,
 *   authRequiredOnSessionNew: boolean,
 *   capProfile: CapProfile,
 *   promptMode: PromptMode,
 *   permissionDefault?: 'allow'|'deny'|null,
 *   cancelAck: 'immediate'|'delayed'|'ignore'|'unsupported',
 *   exitOnCrash?: number,
 *   notes?: string[],
 * }>}
 */
export const SCENARIOS = {
  happy_prompt_stream: {
    id: "happy_prompt_stream",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "stream_tools",
    cancelAck: "immediate",
    notes: [
      "Structural happy path; not live model parity",
      "session-prompt-stream.jsonl is synthetic reference only",
    ],
  },
  auth_required_session_new: {
    id: "auth_required_session_new",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: true,
    capProfile: "full",
    promptMode: "idle_until_eof",
    cancelAck: "immediate",
    notes: [
      "Mirrors live-scrubbed session-new-auth-required.json wire shape",
      "process initialize may succeed; session must not become prompt-ready",
    ],
  },
  permission_allow: {
    id: "permission_allow",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "permission",
    permissionDefault: "allow",
    cancelAck: "immediate",
  },
  permission_deny: {
    id: "permission_deny",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "permission",
    permissionDefault: "deny",
    cancelAck: "immediate",
  },
  cancel_mid_stream: {
    id: "cancel_mid_stream",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "cancel_stream",
    cancelAck: "immediate",
  },
  cancel_while_permission_pending: {
    id: "cancel_while_permission_pending",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "cancel_permission",
    cancelAck: "immediate",
  },
  malformed_frame: {
    id: "malformed_frame",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "malformed",
    cancelAck: "immediate",
  },
  unknown_vendor_notification: {
    id: "unknown_vendor_notification",
    evidence: "synthetic",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "vendor_unknown",
    cancelAck: "immediate",
  },
  eof_mid_prompt: {
    id: "eof_mid_prompt",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "eof",
    cancelAck: "immediate",
  },
  crash_nonzero_exit: {
    id: "crash_nonzero_exit",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "crash",
    cancelAck: "immediate",
    exitOnCrash: 1,
  },
  cancel_unsupported: {
    id: "cancel_unsupported",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "cancel_unsupported",
    promptMode: "cancel_stream",
    cancelAck: "unsupported",
  },
  slow_cancel_ack: {
    id: "slow_cancel_ack",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "cancel_stream",
    cancelAck: "delayed",
  },
  duplicate_response_id: {
    id: "duplicate_response_id",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "duplicate_id",
    cancelAck: "immediate",
  },
  capability_minimal: {
    id: "capability_minimal",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "minimal",
    promptMode: "minimal_complete",
    cancelAck: "immediate",
  },
  clean_shutdown_stdin_close: {
    id: "clean_shutdown_stdin_close",
    evidence: "fake-runtime",
    authRequiredOnSessionNew: false,
    capProfile: "full",
    promptMode: "idle_until_eof",
    cancelAck: "immediate",
  },
};

export function getScenario(id) {
  if (!id || typeof id !== "string") {
    return null;
  }
  return SCENARIOS[id] ?? null;
}

export function listScenarioIds() {
  return [...IMPLEMENTED_SCENARIOS];
}

export function assertScenarioId(id) {
  const scenario = getScenario(id);
  if (!scenario) {
    const known = IMPLEMENTED_SCENARIOS.join(", ");
    throw new Error(
      `Unknown or unsupported fake scenario "${id}". Implemented: ${known}. ` +
        `Live-only catalog IDs are never served by the fake (no live parity).`,
    );
  }
  return scenario;
}

/** Tracer-facing capability view derived from initialize wire. */
export function tracerCapabilitiesFor(capProfile) {
  switch (capProfile) {
    case "minimal":
      return {
        promptStreaming: false,
        cancellation: true,
        planUpdates: false,
        toolCalls: false,
        approvals: false,
        fileChangeNotifications: false,
        terminalOutput: false,
      };
    case "cancel_unsupported":
      return {
        promptStreaming: true,
        cancellation: false,
        planUpdates: true,
        toolCalls: true,
        approvals: true,
        fileChangeNotifications: false,
        terminalOutput: false,
      };
    case "full":
    default:
      return {
        promptStreaming: true,
        cancellation: true,
        planUpdates: true,
        toolCalls: true,
        approvals: true,
        fileChangeNotifications: false,
        terminalOutput: false,
      };
  }
}

export function buildInitializeResult(capProfile) {
  const tracerCaps = tracerCapabilitiesFor(capProfile);
  const full = capProfile === "full" || capProfile === "cancel_unsupported";

  return {
    protocolVersion: 1,
    agentCapabilities: {
      loadSession: full,
      promptCapabilities: {
        image: false,
        audio: false,
        embeddedContext: full,
      },
      mcpCapabilities: {
        http: false,
        sse: false,
      },
      sessionCapabilities: {},
      auth: {},
      _meta: {
        "tracer/fake": true,
        "tracer/cancellation": tracerCaps.cancellation,
        "tracer/promptStreaming": tracerCaps.promptStreaming,
        "tracer/planUpdates": tracerCaps.planUpdates,
        "tracer/toolCalls": tracerCaps.toolCalls,
        "tracer/approvals": tracerCaps.approvals,
        "tracer/capabilities": tracerCaps,
      },
    },
    authMethods: [
      {
        id: "fake-auth",
        name: "Fake Auth",
        description: "No-op authenticate for CI (not live credentials)",
      },
    ],
    _meta: {
      fakeRuntime: true,
      evidence: "fake-runtime",
      notLiveParity: true,
      defaultAuthMethodId: "fake-auth",
      currentWorkingDirectory: FIXED.projectRootPlaceholder,
      agentVersion: FIXED.agentVersion,
      agentId: FIXED.agentId,
      agentInstanceId: FIXED.agentInstanceId,
      hostname: FIXED.hostnamePlaceholder,
      cancelRewind: tracerCaps.cancellation,
      modelState: {
        currentModelId: "fake-model",
        availableModels: ["fake-model"],
      },
    },
  };
}

export { FIXED };
