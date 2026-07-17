/**
 * Mock store only — no ACP, no real storage, no process control.
 * Drives shell placeholders and presentation containers for W1-A smoke.
 */

import type { RuntimeObservation, SessionStatus } from "@tracer/ui";
import type {
  ProjectSummary,
  RouteKey,
  SessionSummary,
  TracerEventEnvelope,
} from "../types/tracer";

export type GlobalBannerKind =
  | "none"
  | "runtime_missing"
  | "storage_error"
  | "control_plane_down";

export interface MockState {
  route: RouteKey;
  projects: ProjectSummary[];
  sessionsByProject: Record<string, SessionSummary[]>;
  activeSessionStatus: SessionStatus;
  runtimeObservation: RuntimeObservation;
  globalBanner: GlobalBannerKind;
  events: TracerEventEnvelope[];
  /** Demo runtime labeling (STATE_MATRIX honesty for synthetic). */
  demoRuntime: boolean;
  sideTab: "plan" | "approvals" | "changes" | "runtime";
  composerText: string;
  lastError: string | null;
}

const DEMO_PROJECT_ID = "11111111-1111-1111-1111-111111111111";
const DEMO_SESSION_ID = "22222222-2222-2222-2222-222222222222";

const initialProjects: ProjectSummary[] = [
  {
    projectId: DEMO_PROJECT_ID,
    name: "demo-repo",
    rootPath: "<user-selected-absolute-path>",
    status: "ready",
    lastOpenedAt: "2026-07-17T12:00:00.000Z",
  },
];

const initialSessions: SessionSummary[] = [
  {
    sessionId: DEMO_SESSION_ID,
    projectId: DEMO_PROJECT_ID,
    title: "Demo session (mock)",
    status: "ready",
    createdAt: "2026-07-17T12:00:00.000Z",
    updatedAt: "2026-07-17T12:05:00.000Z",
  },
];

export function createInitialMockState(): MockState {
  return {
    route: { name: "projects" },
    projects: initialProjects,
    sessionsByProject: { [DEMO_PROJECT_ID]: initialSessions },
    activeSessionStatus: "ready",
    runtimeObservation: "ready",
    globalBanner: "none",
    events: [],
    demoRuntime: true,
    sideTab: "plan",
    composerText: "",
    lastError: null,
  };
}

export type MockAction =
  | { type: "navigate"; route: RouteKey }
  | { type: "setSessionStatus"; status: SessionStatus }
  | { type: "setRuntimeObservation"; observation: RuntimeObservation }
  | { type: "setGlobalBanner"; banner: GlobalBannerKind }
  | { type: "setSideTab"; tab: MockState["sideTab"] }
  | { type: "setComposerText"; text: string }
  | { type: "clearProjects" }
  | { type: "restoreProjects" }
  | { type: "simulatePromptSubmit" }
  | { type: "simulateApproval" }
  | { type: "simulateDisconnect" }
  | { type: "simulateComplete" }
  | { type: "simulateCancel" }
  | { type: "simulateFail"; message: string }
  | { type: "resetDemo" };

export function mockReducer(state: MockState, action: MockAction): MockState {
  switch (action.type) {
    case "navigate":
      return { ...state, route: action.route };
    case "setSessionStatus":
      return { ...state, activeSessionStatus: action.status, lastError: null };
    case "setRuntimeObservation":
      return { ...state, runtimeObservation: action.observation };
    case "setGlobalBanner":
      return { ...state, globalBanner: action.banner };
    case "setSideTab":
      return { ...state, sideTab: action.tab };
    case "setComposerText":
      return { ...state, composerText: action.text };
    case "clearProjects":
      return { ...state, projects: [], route: { name: "projects" } };
    case "restoreProjects":
      return {
        ...state,
        projects: initialProjects,
        sessionsByProject: { [DEMO_PROJECT_ID]: initialSessions },
      };
    case "simulatePromptSubmit":
      return {
        ...state,
        activeSessionStatus: "running",
        runtimeObservation: "ready",
        composerText: "",
        events: [
          ...state.events,
          {
            eventId: `evt-prompt-${state.events.length + 1}`,
            sessionId: DEMO_SESSION_ID,
            sequence: state.events.length + 1,
            type: "session.prompt.submitted",
            timestamp: new Date().toISOString(),
            payload: { text: state.composerText || "(empty mock prompt)" },
          },
        ],
      };
    case "simulateApproval":
      return {
        ...state,
        activeSessionStatus: "awaiting_approval",
        sideTab: "approvals",
        events: [
          ...state.events,
          {
            eventId: `evt-approval-${state.events.length + 1}`,
            sessionId: DEMO_SESSION_ID,
            sequence: state.events.length + 1,
            type: "approval.requested",
            timestamp: new Date().toISOString(),
            payload: {
              action: "run_command",
              description: "Mock approval request (fail-closed)",
              risk: "unknown",
            },
          },
        ],
      };
    case "simulateDisconnect":
      return {
        ...state,
        activeSessionStatus: "disconnected",
        runtimeObservation: "crashed",
        lastError: "Runtime process exited unexpectedly (mock).",
      };
    case "simulateComplete":
      return {
        ...state,
        activeSessionStatus: "completed",
        runtimeObservation: "ready",
      };
    case "simulateCancel":
      return {
        ...state,
        activeSessionStatus: "stopped",
        runtimeObservation: "stopped",
      };
    case "simulateFail":
      return {
        ...state,
        activeSessionStatus: "failed",
        lastError: action.message,
      };
    case "resetDemo":
      return createInitialMockState();
    default:
      return state;
  }
}

export const MOCK_IDS = {
  projectId: DEMO_PROJECT_ID,
  sessionId: DEMO_SESSION_ID,
} as const;

/** Composer enablement per STATE_MATRIX §3. */
export function isComposerEnabled(status: SessionStatus, runtime: RuntimeObservation): boolean {
  if (runtime === "crashed" || runtime === "unavailable" || runtime === "sign_in_required") {
    return false;
  }
  return status === "ready";
}

export function composerDisabledReason(
  status: SessionStatus,
  runtime: RuntimeObservation,
): string | null {
  if (runtime === "sign_in_required") return "Sign in required";
  if (runtime === "crashed" || runtime === "unavailable") return "Runtime unavailable";
  switch (status) {
    case "ready":
      return null;
    case "creating":
      return "Creating session…";
    case "starting_runtime":
      return "Starting runtime…";
    case "running":
      return "Agent is working…";
    case "awaiting_approval":
      return "Approval required";
    case "cancelling":
      return "Cancelling…";
    case "completed":
      return "Session completed";
    case "failed":
      return "Session failed";
    case "disconnected":
      return "Runtime disconnected";
    case "stopped":
      return "Session stopped";
    default:
      return "Prompt unavailable";
  }
}

export function isCancelVisible(status: SessionStatus): boolean {
  return status === "running" || status === "awaiting_approval";
}
