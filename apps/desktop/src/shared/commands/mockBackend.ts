/**
 * Deterministic mock command backend for browser dev + unit tests.
 * Mirrors TAURI_COMMAND_CONTRACT_V1 / control-plane presentation shapes.
 * No network, no credentials, no raw ACP.
 */

import type {
  ProjectSummary,
  SessionSummary,
  TracerEventEnvelope,
} from "../types/tracer";
import type {
  AuthenticationState,
  HeliStatusView,
  PendingApprovalView,
  PresentationSnapshot,
} from "../types/snapshot";
import { emptyPresentationSnapshot } from "../types/snapshot";
import type { SessionStatus } from "@tracer/event-types";
import { TracerInvokeError, type TracerCommandName } from "./invoke";

export type MockScenario =
  | "default"
  | "runtime_unavailable"
  | "authentication_required"
  | "prompt_streaming"
  | "approval_request"
  | "approval_accepted"
  | "approval_rejected"
  | "cancel_pending_approval"
  | "completed_run"
  | "runtime_crash"
  | "session_history_restore"
  | "heli_unavailable";

const DEMO_PROJECT_ID = "11111111-1111-1111-1111-111111111111";
const DEMO_SESSION_ID = "22222222-2222-2222-2222-222222222222";
const DEMO_APPROVAL_ID = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

export const MOCK_BACKEND_IDS = {
  projectId: DEMO_PROJECT_ID,
  sessionId: DEMO_SESSION_ID,
  approvalId: DEMO_APPROVAL_ID,
} as const;

export interface MockBackendState {
  projects: ProjectSummary[];
  sessionsByProject: Record<string, SessionSummary[]>;
  snapshot: PresentationSnapshot;
  events: TracerEventEnvelope[];
  heliUnavailable: boolean;
  controlPlaneDown: boolean;
  seq: number;
}

function demoProject(): ProjectSummary {
  return {
    projectId: DEMO_PROJECT_ID,
    name: "demo-repo",
    rootPath: "<user-selected-absolute-path>",
    status: "ready",
    lastOpenedAt: "2026-07-17T12:00:00.000Z",
  };
}

function demoSession(status: SessionStatus = "ready"): SessionSummary {
  return {
    sessionId: DEMO_SESSION_ID,
    projectId: DEMO_PROJECT_ID,
    title: "Demo session",
    status,
    createdAt: "2026-07-17T12:00:00.000Z",
    updatedAt: "2026-07-17T12:05:00.000Z",
  };
}

function heliOk(): HeliStatusView {
  return {
    available: true,
    workspaceRoot: "<heli-workspace-root>",
    mode: "concurrent",
    summary: "Heli workspace detected (mock)",
    warnings: [],
  };
}

function heliMissing(): HeliStatusView {
  return {
    available: false,
    workspaceRoot: null,
    mode: null,
    summary: "Heli workspace not found (non-fatal)",
    warnings: ["Heli unavailable — agent coordination features limited"],
  };
}

function baseSnapshot(partial: Partial<PresentationSnapshot> = {}): PresentationSnapshot {
  return emptyPresentationSnapshot({
    activeProjectId: DEMO_PROJECT_ID,
    activeSessionId: DEMO_SESSION_ID,
    sessionStatus: "ready",
    runtimeObservation: "ready",
    authState: "not_required",
    heli: heliOk(),
    latestSequence: 0,
    promptInFlight: false,
    ...partial,
  });
}

function pushEvent(
  state: MockBackendState,
  type: string,
  payload?: Record<string, unknown>,
): void {
  state.seq += 1;
  state.events.push({
    eventId: `evt-${state.seq}`,
    sessionId: DEMO_SESSION_ID,
    sequence: state.seq,
    type,
    timestamp: new Date().toISOString(),
    payload,
  });
  state.snapshot = {
    ...state.snapshot,
    latestSequence: state.seq,
  };
}

function syncSessionStatus(state: MockBackendState, status: SessionStatus): void {
  state.snapshot = { ...state.snapshot, sessionStatus: status };
  const list = state.sessionsByProject[DEMO_PROJECT_ID] ?? [];
  state.sessionsByProject = {
    ...state.sessionsByProject,
    [DEMO_PROJECT_ID]: list.map((s) =>
      s.sessionId === DEMO_SESSION_ID
        ? { ...s, status, updatedAt: new Date().toISOString() }
        : s,
    ),
  };
}

export function createMockBackendState(scenario: MockScenario = "default"): MockBackendState {
  const state: MockBackendState = {
    projects: [demoProject()],
    sessionsByProject: { [DEMO_PROJECT_ID]: [demoSession("ready")] },
    snapshot: baseSnapshot(),
    events: [],
    heliUnavailable: false,
    controlPlaneDown: false,
    seq: 0,
  };
  applyScenario(state, scenario);
  return state;
}

export function applyScenario(state: MockBackendState, scenario: MockScenario): void {
  switch (scenario) {
    case "default":
    case "prompt_streaming":
      break;
    case "runtime_unavailable":
      state.snapshot = baseSnapshot({
        sessionStatus: "failed",
        runtimeObservation: "unavailable",
        lastError: {
          errorClass: "RuntimeExecutableNotFound",
          message: "Agent runtime executable not found (mock)",
        },
      });
      syncSessionStatus(state, "failed");
      break;
    case "authentication_required":
      state.snapshot = baseSnapshot({
        sessionStatus: "ready",
        runtimeObservation: "protocol_ready",
        authState: "unauthenticated",
        lastError: {
          errorClass: "AuthenticationRequired",
          message: "Sign in required to use this agent runtime",
        },
      });
      break;
    case "approval_request":
    case "approval_accepted":
    case "approval_rejected":
    case "cancel_pending_approval": {
      const approval: PendingApprovalView = {
        approvalId: DEMO_APPROVAL_ID,
        sessionId: DEMO_SESSION_ID,
        action: "run_command",
        description: "Mock approval request (fail-closed)",
        risk: "unknown",
        createdAt: "2026-07-17T12:06:00.000Z",
      };
      state.snapshot = baseSnapshot({
        sessionStatus: "awaiting_approval",
        runtimeObservation: "awaiting_approval",
        pendingApprovals: [approval],
        promptInFlight: true,
        latestSequence: 2,
      });
      syncSessionStatus(state, "awaiting_approval");
      state.seq = 2;
      state.events = [
        {
          eventId: "evt-1",
          sessionId: DEMO_SESSION_ID,
          sequence: 1,
          type: "session.prompt.submitted",
          timestamp: "2026-07-17T12:05:30.000Z",
          payload: { text: "do something" },
        },
        {
          eventId: "evt-2",
          sessionId: DEMO_SESSION_ID,
          sequence: 2,
          type: "approval.requested",
          timestamp: "2026-07-17T12:06:00.000Z",
          payload: {
            approvalId: DEMO_APPROVAL_ID,
            action: approval.action,
            description: approval.description,
            risk: approval.risk,
          },
        },
      ];
      if (scenario === "approval_accepted") {
        resolveApprovalInternal(state, "allow");
      } else if (scenario === "approval_rejected") {
        resolveApprovalInternal(state, "deny");
      } else if (scenario === "cancel_pending_approval") {
        resolveApprovalInternal(state, "cancel");
      }
      break;
    }
    case "completed_run":
      state.snapshot = baseSnapshot({
        sessionStatus: "completed",
        runtimeObservation: "stopped",
        latestSequence: 3,
        promptInFlight: false,
      });
      syncSessionStatus(state, "completed");
      state.seq = 3;
      state.events = [
        {
          eventId: "evt-1",
          sessionId: DEMO_SESSION_ID,
          sequence: 1,
          type: "session.prompt.submitted",
          timestamp: "2026-07-17T12:05:00.000Z",
        },
        {
          eventId: "evt-2",
          sessionId: DEMO_SESSION_ID,
          sequence: 2,
          type: "agent.message.delta",
          timestamp: "2026-07-17T12:05:01.000Z",
          payload: { text: "Hello from mock stream" },
        },
        {
          eventId: "evt-3",
          sessionId: DEMO_SESSION_ID,
          sequence: 3,
          type: "session.completed",
          timestamp: "2026-07-17T12:05:02.000Z",
        },
      ];
      break;
    case "runtime_crash":
      state.snapshot = baseSnapshot({
        sessionStatus: "disconnected",
        runtimeObservation: "disconnected",
        lastError: {
          errorClass: "RuntimeCrashed",
          message: "Runtime process exited unexpectedly (mock)",
        },
        promptInFlight: false,
      });
      syncSessionStatus(state, "disconnected");
      pushEvent(state, "runtime.process.exited", { expected: false });
      break;
    case "session_history_restore":
      state.snapshot = baseSnapshot({
        sessionStatus: "completed",
        runtimeObservation: "stopped",
        latestSequence: 4,
      });
      syncSessionStatus(state, "completed");
      state.seq = 4;
      state.events = [
        {
          eventId: "evt-1",
          sessionId: DEMO_SESSION_ID,
          sequence: 1,
          type: "session.created",
          timestamp: "2026-07-17T12:00:00.000Z",
        },
        {
          eventId: "evt-2",
          sessionId: DEMO_SESSION_ID,
          sequence: 2,
          type: "session.prompt.submitted",
          timestamp: "2026-07-17T12:01:00.000Z",
          payload: { text: "summarize" },
        },
        {
          eventId: "evt-3",
          sessionId: DEMO_SESSION_ID,
          sequence: 3,
          type: "agent.message.completed",
          timestamp: "2026-07-17T12:01:30.000Z",
          payload: { text: "Restored summary" },
        },
        {
          eventId: "evt-4",
          sessionId: DEMO_SESSION_ID,
          sequence: 4,
          type: "session.completed",
          timestamp: "2026-07-17T12:01:31.000Z",
        },
      ];
      break;
    case "heli_unavailable":
      state.heliUnavailable = true;
      state.snapshot = baseSnapshot({ heli: heliMissing() });
      break;
    default:
      break;
  }
}

function resolveApprovalInternal(
  state: MockBackendState,
  decision: "allow" | "deny" | "cancel",
): void {
  if (decision === "cancel") {
    state.snapshot = {
      ...state.snapshot,
      sessionStatus: "stopped",
      runtimeObservation: "stopped",
      pendingApprovals: [],
      promptInFlight: false,
    };
    syncSessionStatus(state, "stopped");
    pushEvent(state, "approval.resolved", { decision, approvalId: DEMO_APPROVAL_ID });
    pushEvent(state, "session.cancelled", {});
    return;
  }
  state.snapshot = {
    ...state.snapshot,
    sessionStatus: "running",
    runtimeObservation: "running",
    pendingApprovals: [],
    promptInFlight: true,
  };
  syncSessionStatus(state, "running");
  pushEvent(state, "approval.resolved", { decision, approvalId: DEMO_APPROVAL_ID });
}

export interface MockBackend {
  readonly state: MockBackendState;
  reset(scenario?: MockScenario): void;
  setScenario(scenario: MockScenario): void;
  handle<TResult = unknown>(
    command: TracerCommandName,
    args?: Record<string, unknown>,
  ): Promise<TResult>;
}

function cryptoRandomId(): string {
  const s = Math.random().toString(16).slice(2).padEnd(12, "0");
  return `${s.slice(0, 8)}-${s.slice(0, 4)}-4${s.slice(1, 4)}-a${s.slice(1, 4)}-${s.slice(0, 12)}`;
}

export function createMockBackend(scenario: MockScenario = "default"): MockBackend {
  let state = createMockBackendState(scenario);

  const api: MockBackend = {
    get state() {
      return state;
    },
    reset(next: MockScenario = "default") {
      state = createMockBackendState(next);
    },
    setScenario(next: MockScenario) {
      applyScenario(state, next);
    },
    async handle<TResult = unknown>(
      command: TracerCommandName,
      args: Record<string, unknown> = {},
    ): Promise<TResult> {
      if (state.controlPlaneDown) {
        throw new TracerInvokeError({
          errorClass: "InternalError",
          message: "Control plane not responding (mock)",
          retryable: true,
        });
      }

      switch (command) {
        case "tracer_app_info":
          return {
            appVersion: "0.1.0-mock",
            eventProtocolVersion: 1,
            commandContractVersion: "1.0.0",
            platform: "mock",
            module: "VS1-H2",
          } as TResult;

        case "tracer_presentation_snapshot":
          return { ...state.snapshot } as TResult;

        case "tracer_heli_status": {
          const heli = state.heliUnavailable ? heliMissing() : state.snapshot.heli;
          state.snapshot = { ...state.snapshot, heli };
          return { ...heli } as TResult;
        }

        case "tracer_project_list":
          return { projects: [...state.projects] } as TResult;

        case "tracer_project_register": {
          const rootPath = String(args.rootPath ?? "");
          if (!rootPath) {
            throw new TracerInvokeError({
              errorClass: "InvalidArgument",
              message: "rootPath is required",
              retryable: false,
            });
          }
          const project: ProjectSummary = {
            projectId: cryptoRandomId(),
            name: String(args.name ?? "registered-project"),
            rootPath,
            status: "ready",
            lastOpenedAt: new Date().toISOString(),
          };
          state.projects = [...state.projects, project];
          return { project } as TResult;
        }

        case "tracer_project_get": {
          const projectId = String(args.projectId ?? "");
          const project = state.projects.find((p) => p.projectId === projectId);
          if (!project) {
            throw new TracerInvokeError({
              errorClass: "NotFound",
              message: `Project not found: ${projectId}`,
              retryable: false,
            });
          }
          return { project } as TResult;
        }

        case "tracer_session_list": {
          const projectId = String(args.projectId ?? "");
          const sessions = state.sessionsByProject[projectId] ?? [];
          return { sessions: [...sessions], nextCursor: null } as TResult;
        }

        case "tracer_session_create": {
          const projectId = String(args.projectId ?? "");
          if (!state.projects.some((p) => p.projectId === projectId)) {
            throw new TracerInvokeError({
              errorClass: "NotFound",
              message: `Project not found: ${projectId}`,
              retryable: false,
            });
          }
          if (state.snapshot.runtimeObservation === "unavailable") {
            throw new TracerInvokeError({
              errorClass: "RuntimeExecutableNotFound",
              message: "Agent runtime executable not found (mock)",
              retryable: false,
            });
          }
          const sessionId = cryptoRandomId();
          const session: SessionSummary = {
            sessionId,
            projectId,
            title: String(args.title ?? "New session"),
            status: "ready",
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          };
          const list = state.sessionsByProject[projectId] ?? [];
          state.sessionsByProject = {
            ...state.sessionsByProject,
            [projectId]: [...list, session],
          };
          state.snapshot = {
            ...state.snapshot,
            activeProjectId: projectId,
            activeSessionId: sessionId,
            sessionStatus: "ready",
            runtimeObservation: "ready",
            pendingApprovals: [],
            lastError: null,
            promptInFlight: false,
          };
          pushEvent(state, "session.created", { sessionId });
          pushEvent(state, "session.ready", { sessionId });
          return { session } as TResult;
        }

        case "tracer_session_get": {
          const sessionId = String(args.sessionId ?? "");
          for (const list of Object.values(state.sessionsByProject)) {
            const session = list.find((s) => s.sessionId === sessionId);
            if (session) {
              return {
                session: {
                  ...session,
                  runtimeKind: "acp-stdio",
                  authState: state.snapshot.authState,
                  lastError: state.snapshot.lastError,
                  processAlive: !["disconnected", "unavailable", "crashed"].includes(
                    state.snapshot.runtimeObservation,
                  ),
                  protocolReady: state.snapshot.runtimeObservation !== "unknown",
                  sessionReady: state.snapshot.sessionStatus === "ready",
                },
              } as TResult;
            }
          }
          throw new TracerInvokeError({
            errorClass: "NotFound",
            message: `Session not found: ${sessionId}`,
            retryable: false,
          });
        }

        case "tracer_session_submit_prompt": {
          const sessionId = String(args.sessionId ?? state.snapshot.activeSessionId ?? "");
          const text = String(args.text ?? "");
          if (state.snapshot.authState === "unauthenticated") {
            throw new TracerInvokeError({
              errorClass: "AuthenticationRequired",
              message: "Sign in required to use this agent runtime",
              retryable: false,
            });
          }
          if (state.snapshot.sessionStatus === "disconnected") {
            throw new TracerInvokeError({
              errorClass: "RuntimeDisconnected",
              message: "Runtime disconnected",
              retryable: false,
            });
          }
          if (state.snapshot.runtimeObservation === "unavailable") {
            throw new TracerInvokeError({
              errorClass: "RuntimeExecutableNotFound",
              message: "Runtime unavailable",
              retryable: false,
            });
          }
          if (state.snapshot.sessionStatus !== "ready") {
            throw new TracerInvokeError({
              errorClass: "InvalidState",
              message: `Cannot prompt while session is ${state.snapshot.sessionStatus}`,
              retryable: false,
            });
          }
          state.snapshot = {
            ...state.snapshot,
            activeSessionId: sessionId,
            sessionStatus: "running",
            runtimeObservation: "running",
            promptInFlight: true,
            lastError: null,
          };
          syncSessionStatus(state, "running");
          pushEvent(state, "session.prompt.submitted", { text });
          pushEvent(state, "agent.message.delta", { text: `Echo: ${text}` });
          return {
            promptId: cryptoRandomId(),
            agentRunId: cryptoRandomId(),
            accepted: true,
          } as TResult;
        }

        case "tracer_session_cancel": {
          const status = state.snapshot.sessionStatus;
          if (status === "awaiting_approval" || status === "running" || status === "cancelling") {
            state.snapshot = {
              ...state.snapshot,
              sessionStatus: "cancelling",
              runtimeObservation: "cancelling",
              pendingApprovals: [],
              promptInFlight: false,
            };
            syncSessionStatus(state, "cancelling");
            pushEvent(state, "session.cancelled", { scope: args.scope ?? "active_run" });
            state.snapshot = {
              ...state.snapshot,
              sessionStatus: "stopped",
              runtimeObservation: "stopped",
            };
            syncSessionStatus(state, "stopped");
            return { accepted: true, mode: "cooperative" } as TResult;
          }
          return { accepted: true, mode: "already_terminal" } as TResult;
        }

        case "tracer_session_stop": {
          state.snapshot = {
            ...state.snapshot,
            sessionStatus: "stopped",
            runtimeObservation: "stopped",
            promptInFlight: false,
            pendingApprovals: [],
          };
          syncSessionStatus(state, "stopped");
          pushEvent(state, "runtime.process.exited", { expected: true });
          return { stopped: true } as TResult;
        }

        case "tracer_events_list": {
          const sessionId = String(args.sessionId ?? "");
          const after = Number(args.afterSequence ?? 0);
          const limit = Number(args.limit ?? 200);
          const events = state.events
            .filter((e) => e.sessionId === sessionId && e.sequence > after)
            .slice(0, limit);
          return {
            events,
            latestSequence: state.snapshot.latestSequence,
          } as TResult;
        }

        case "tracer_approval_list_pending": {
          const sessionId = String(args.sessionId ?? "");
          const approvals = state.snapshot.pendingApprovals.filter(
            (a) => a.sessionId === sessionId || !sessionId,
          );
          return { approvals } as TResult;
        }

        case "tracer_approval_resolve": {
          const approvalId = String(args.approvalId ?? "");
          const decision = String(args.decision ?? "") as "allow" | "deny" | "cancel";
          const pending = state.snapshot.pendingApprovals.find((a) => a.approvalId === approvalId);
          if (!pending) {
            throw new TracerInvokeError({
              errorClass: "NotFound",
              message: `Approval not found: ${approvalId}`,
              retryable: false,
            });
          }
          if (!["allow", "deny", "cancel"].includes(decision)) {
            throw new TracerInvokeError({
              errorClass: "InvalidArgument",
              message: `Invalid decision: ${decision}`,
              retryable: false,
            });
          }
          resolveApprovalInternal(state, decision);
          return { resolved: true } as TResult;
        }

        case "tracer_runtime_status": {
          const sessionId =
            (args.sessionId as string | undefined) ?? state.snapshot.activeSessionId ?? undefined;
          return {
            processes: sessionId
              ? [
                  {
                    sessionId,
                    state: state.snapshot.runtimeObservation,
                    pid: null,
                    runtimeKind: "acp-stdio",
                    capabilities: state.snapshot.capabilities,
                    processAlive: !["disconnected", "unavailable", "stopped"].includes(
                      state.snapshot.runtimeObservation,
                    ),
                    protocolReady: true,
                    sessionReady: state.snapshot.sessionStatus === "ready",
                    authState: state.snapshot.authState,
                  },
                ]
              : [],
          } as TResult;
        }

        default:
          throw new TracerInvokeError({
            errorClass: "Unsupported",
            message: `Mock backend: unsupported command ${command}`,
            retryable: false,
            details: { command },
          });
      }
    },
  };

  return api;
}

export function setMockAuthState(backend: MockBackend, auth: AuthenticationState): void {
  backend.state.snapshot = {
    ...backend.state.snapshot,
    authState: auth,
  };
}

export function setMockControlPlaneDown(backend: MockBackend, down: boolean): void {
  backend.state.controlPlaneDown = down;
}