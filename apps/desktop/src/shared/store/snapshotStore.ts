/**
 * Snapshot-backed journey store (VS1-H2).
 *
 * Owns the vertical-slice core journey via typed commands + presentation snapshots.
 * React receives PresentationSnapshot + derived view model only.
 * Never parses ACP, never writes SQLite, never owns process lifecycle.
 */

import type { RuntimeObservation, SessionStatus } from "@tracer/ui";
import type {
  ProjectSummary,
  RouteKey,
  SessionSummary,
  TracerEventEnvelope,
} from "../types/tracer";
import type {
  AuthenticationState,
  HeliStatusView,
  PendingApprovalView,
  PresentationSnapshot,
} from "../types/snapshot";
import {
  emptyPresentationSnapshot,
  isAuthBlocking,
  mapErrorClassToFailure,
  mapRuntimeObservation,
  type NormalizedFailureKind,
} from "../types/snapshot";
import {
  getMockBackend,
  invokeTracer,
  resolveInvokeBackend,
  setInvokeMode,
  setMockBackend,
  TracerInvokeError,
} from "../commands/invoke";
import {
  createMockBackend,
  MOCK_BACKEND_IDS,
  type MockScenario,
} from "../commands/mockBackend";

export type GlobalBannerKind =
  | "none"
  | "runtime_missing"
  | "storage_error"
  | "control_plane_down"
  | "heli_unavailable";

export type LoadPhase = "idle" | "loading" | "ready" | "failed";

/** UI-facing app state derived from commands + snapshots. */
export interface AppViewState {
  route: RouteKey;
  loadPhase: LoadPhase;
  projects: ProjectSummary[];
  sessionsByProject: Record<string, SessionSummary[]>;
  /** Authoritative presentation snapshot from control plane / mock. */
  snapshot: PresentationSnapshot;
  /** Normalized events for timeline (from events_list, not ACP). */
  events: TracerEventEnvelope[];
  /** Mapped runtime pill observation. */
  runtimeObservation: RuntimeObservation;
  /** Session status from snapshot (ready default for empty shell). */
  sessionStatus: SessionStatus;
  authState: AuthenticationState;
  pendingApprovals: PendingApprovalView[];
  heli: HeliStatusView;
  globalBanner: GlobalBannerKind;
  sideTab: "plan" | "approvals" | "changes" | "runtime";
  composerText: string;
  lastErrorMessage: string | null;
  lastFailureKind: NormalizedFailureKind | null;
  /** True when using mock backend (browser / tests). */
  demoRuntime: boolean;
  /** Busy flag for in-flight command (not process ownership). */
  commandBusy: boolean;
}

export type AppAction =
  | { type: "navigate"; route: RouteKey }
  | { type: "setSideTab"; tab: AppViewState["sideTab"] }
  | { type: "setComposerText"; text: string }
  | { type: "setGlobalBanner"; banner: GlobalBannerKind }
  | { type: "hydrate"; partial: Partial<AppViewState> }
  | { type: "applySnapshot"; snapshot: PresentationSnapshot }
  | { type: "setEvents"; events: TracerEventEnvelope[] }
  | { type: "setProjects"; projects: ProjectSummary[] }
  | { type: "setSessions"; projectId: string; sessions: SessionSummary[] }
  | { type: "setLoadPhase"; phase: LoadPhase }
  | { type: "setCommandBusy"; busy: boolean }
  | { type: "setError"; message: string | null; kind?: NormalizedFailureKind | null }
  | { type: "resetLocal" };

export function createInitialAppState(): AppViewState {
  return {
    route: { name: "projects" },
    loadPhase: "idle",
    projects: [],
    sessionsByProject: {},
    snapshot: emptyPresentationSnapshot(),
    events: [],
    runtimeObservation: "not_started",
    sessionStatus: "ready",
    authState: "not_required",
    pendingApprovals: [],
    heli: emptyPresentationSnapshot().heli,
    globalBanner: "none",
    sideTab: "plan",
    composerText: "",
    lastErrorMessage: null,
    lastFailureKind: null,
    demoRuntime: resolveInvokeBackend() === "mock",
    commandBusy: false,
  };
}

function deriveFromSnapshot(
  state: AppViewState,
  snapshot: PresentationSnapshot,
): AppViewState {
  const sessionStatus = snapshot.sessionStatus ?? "ready";
  const runtimeObservation = mapRuntimeObservation(
    snapshot.runtimeObservation,
    snapshot.authState,
  );
  const lastErrorMessage =
    snapshot.lastError?.message != null
      ? String(snapshot.lastError.message)
      : state.lastErrorMessage;

  let globalBanner = state.globalBanner;
  if (!snapshot.heli.available && snapshot.heli.summary) {
    // Heli unavailable is non-fatal — soft banner only if nothing more severe.
    if (globalBanner === "none") {
      globalBanner = "heli_unavailable";
    }
  }

  return {
    ...state,
    snapshot,
    sessionStatus,
    runtimeObservation,
    authState: snapshot.authState,
    pendingApprovals: snapshot.pendingApprovals,
    heli: snapshot.heli,
    lastErrorMessage,
    globalBanner,
    sideTab:
      sessionStatus === "awaiting_approval" && snapshot.pendingApprovals.length > 0
        ? "approvals"
        : state.sideTab,
  };
}

export function appReducer(state: AppViewState, action: AppAction): AppViewState {
  switch (action.type) {
    case "navigate":
      return { ...state, route: action.route };
    case "setSideTab":
      return { ...state, sideTab: action.tab };
    case "setComposerText":
      return { ...state, composerText: action.text };
    case "setGlobalBanner":
      return { ...state, globalBanner: action.banner };
    case "hydrate":
      return { ...state, ...action.partial };
    case "applySnapshot":
      return deriveFromSnapshot(state, action.snapshot);
    case "setEvents":
      return { ...state, events: action.events };
    case "setProjects":
      return { ...state, projects: action.projects };
    case "setSessions":
      return {
        ...state,
        sessionsByProject: {
          ...state.sessionsByProject,
          [action.projectId]: action.sessions,
        },
      };
    case "setLoadPhase":
      return { ...state, loadPhase: action.phase };
    case "setCommandBusy":
      return { ...state, commandBusy: action.busy };
    case "setError":
      return {
        ...state,
        lastErrorMessage: action.message,
        lastFailureKind: action.kind ?? state.lastFailureKind,
      };
    case "resetLocal":
      return createInitialAppState();
    default:
      return state;
  }
}

/** Composer enablement per STATE_MATRIX §3 + auth orthogonal gate. */
export function isComposerEnabled(
  status: SessionStatus,
  runtime: RuntimeObservation,
  auth: AuthenticationState = "not_required",
): boolean {
  if (isAuthBlocking(auth)) return false;
  if (runtime === "crashed" || runtime === "unavailable" || runtime === "sign_in_required") {
    return false;
  }
  return status === "ready";
}

export function composerDisabledReason(
  status: SessionStatus,
  runtime: RuntimeObservation,
  auth: AuthenticationState = "not_required",
): string | null {
  if (auth === "unauthenticated" || auth === "expired") return "Sign in required";
  if (auth === "in_progress") return "Signing in…";
  if (auth === "failed") return "Sign-in failed";
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

export function isCancelEnabled(status: SessionStatus): boolean {
  return status === "running" || status === "awaiting_approval";
}

function mapInvokeError(err: unknown): {
  message: string;
  kind: NormalizedFailureKind;
  banner: GlobalBannerKind;
} {
  if (err instanceof TracerInvokeError) {
    const kind = mapErrorClassToFailure(err.errorClass);
    let banner: GlobalBannerKind = "none";
    if (kind === "runtime_missing") banner = "runtime_missing";
    else if (kind === "storage_error") banner = "storage_error";
    else if (kind === "control_plane_down") banner = "control_plane_down";
    return { message: err.message, kind, banner };
  }
  return {
    message: err instanceof Error ? err.message : String(err),
    kind: "generic_failed",
    banner: "none",
  };
}

/**
 * Journey controller — imperative async ops over invokeTracer.
 * UI dispatches local actions; journey methods call commands then refresh snapshot.
 */
export class SnapshotJourney {
  constructor(
    private getState: () => AppViewState,
    private dispatch: (action: AppAction) => void,
  ) {}

  /** Bootstrap: prefer Tauri; install mock backend for browser/tests. */
  static bootstrap(options?: { forceMock?: boolean; scenario?: MockScenario }): void {
    if (options?.forceMock) {
      setInvokeMode("mock");
      setMockBackend(createMockBackend(options.scenario ?? "default"));
      return;
    }
    setInvokeMode("auto");
    if (resolveInvokeBackend() === "mock" && !getMockBackend()) {
      setMockBackend(createMockBackend(options?.scenario ?? "default"));
    }
  }

  /** Initial snapshot + project list (application opens). */
  async bootstrapLoad(): Promise<void> {
    this.dispatch({ type: "setLoadPhase", phase: "loading" });
    try {
      await this.refreshSnapshot();
      await this.refreshHeli();
      await this.loadProjects();
      this.dispatch({ type: "setLoadPhase", phase: "ready" });
      this.dispatch({
        type: "hydrate",
        partial: { demoRuntime: resolveInvokeBackend() === "mock" },
      });
    } catch (err) {
      const mapped = mapInvokeError(err);
      this.dispatch({
        type: "setError",
        message: mapped.message,
        kind: mapped.kind,
      });
      if (mapped.banner !== "none") {
        this.dispatch({ type: "setGlobalBanner", banner: mapped.banner });
      }
      this.dispatch({ type: "setLoadPhase", phase: "failed" });
    }
  }

  /** Missed notifications recover through snapshot refresh. */
  async refreshSnapshot(): Promise<PresentationSnapshot> {
    const snapshot = await invokeTracer<PresentationSnapshot>(
      "tracer_presentation_snapshot",
    );
    this.dispatch({ type: "applySnapshot", snapshot });
    return snapshot;
  }

  /** Heli probe — unavailable must not fail the app. */
  async refreshHeli(): Promise<HeliStatusView> {
    try {
      const heli = await invokeTracer<HeliStatusView>("tracer_heli_status");
      const snap = { ...this.getState().snapshot, heli };
      this.dispatch({ type: "applySnapshot", snapshot: snap });
      if (!heli.available) {
        const current = this.getState().globalBanner;
        if (current === "none") {
          this.dispatch({ type: "setGlobalBanner", banner: "heli_unavailable" });
        }
      }
      return heli;
    } catch {
      // Non-fatal: synthesize unavailable view without failing journey.
      const heli: HeliStatusView = {
        available: false,
        workspaceRoot: null,
        mode: null,
        summary: "Heli status unavailable (non-fatal)",
        warnings: [],
      };
      const snap = { ...this.getState().snapshot, heli };
      this.dispatch({ type: "applySnapshot", snapshot: snap });
      return heli;
    }
  }

  async loadProjects(): Promise<ProjectSummary[]> {
    const result = await invokeTracer<{ projects: ProjectSummary[] }>(
      "tracer_project_list",
    );
    this.dispatch({ type: "setProjects", projects: result.projects });
    return result.projects;
  }

  async loadSessions(projectId: string): Promise<SessionSummary[]> {
    const result = await invokeTracer<{ sessions: SessionSummary[] }>(
      "tracer_session_list",
      { projectId },
    );
    this.dispatch({
      type: "setSessions",
      projectId,
      sessions: result.sessions,
    });
    return result.sessions;
  }

  async createSession(projectId: string, title?: string): Promise<SessionSummary> {
    this.dispatch({ type: "setCommandBusy", busy: true });
    try {
      const result = await invokeTracer<{ session: SessionSummary }>(
        "tracer_session_create",
        { projectId, title },
      );
      await this.refreshSnapshot();
      await this.loadSessions(projectId);
      this.dispatch({
        type: "navigate",
        route: {
          name: "session",
          projectId,
          sessionId: result.session.sessionId,
        },
      });
      await this.loadEvents(result.session.sessionId);
      return result.session;
    } catch (err) {
      this.applyCommandFailure(err);
      throw err;
    } finally {
      this.dispatch({ type: "setCommandBusy", busy: false });
    }
  }

  async openSession(projectId: string, sessionId: string): Promise<void> {
    this.dispatch({ type: "setCommandBusy", busy: true });
    try {
      this.dispatch({
        type: "navigate",
        route: { name: "session", projectId, sessionId },
      });
      // Reopen persisted history via events_list + snapshot.
      await this.refreshSnapshot();
      await this.loadEvents(sessionId);
    } catch (err) {
      this.applyCommandFailure(err);
      throw err;
    } finally {
      this.dispatch({ type: "setCommandBusy", busy: false });
    }
  }

  async loadEvents(sessionId: string, afterSequence = 0): Promise<TracerEventEnvelope[]> {
    const result = await invokeTracer<{
      events: TracerEventEnvelope[];
      latestSequence: number;
    }>("tracer_events_list", { sessionId, afterSequence, limit: 200 });

    const prev = afterSequence > 0 ? this.getState().events : [];
    const merged = afterSequence > 0 ? [...prev, ...result.events] : result.events;
    this.dispatch({ type: "setEvents", events: merged });
    return result.events;
  }

  async submitPrompt(sessionId: string, text: string): Promise<void> {
    this.dispatch({ type: "setCommandBusy", busy: true });
    try {
      await invokeTracer("tracer_session_submit_prompt", { sessionId, text });
      this.dispatch({ type: "setComposerText", text: "" });
      await this.refreshSnapshot();
      await this.loadEvents(sessionId);
    } catch (err) {
      this.applyCommandFailure(err);
      await this.refreshSnapshot().catch(() => undefined);
      throw err;
    } finally {
      this.dispatch({ type: "setCommandBusy", busy: false });
    }
  }

  async cancelSession(sessionId: string): Promise<void> {
    this.dispatch({ type: "setCommandBusy", busy: true });
    try {
      // Optimistic cancelling state via snapshot refresh after command.
      await invokeTracer("tracer_session_cancel", {
        sessionId,
        scope: "active_run",
      });
      await this.refreshSnapshot();
      await this.loadEvents(sessionId);
    } catch (err) {
      this.applyCommandFailure(err);
      await this.refreshSnapshot().catch(() => undefined);
      throw err;
    } finally {
      this.dispatch({ type: "setCommandBusy", busy: false });
    }
  }

  async stopSession(sessionId: string, force = false): Promise<void> {
    this.dispatch({ type: "setCommandBusy", busy: true });
    try {
      await invokeTracer("tracer_session_stop", { sessionId, force });
      await this.refreshSnapshot();
      await this.loadEvents(sessionId);
    } catch (err) {
      this.applyCommandFailure(err);
      throw err;
    } finally {
      this.dispatch({ type: "setCommandBusy", busy: false });
    }
  }

  async resolveApproval(
    sessionId: string,
    approvalId: string,
    decision: "allow" | "deny" | "cancel",
  ): Promise<void> {
    this.dispatch({ type: "setCommandBusy", busy: true });
    try {
      await invokeTracer("tracer_approval_resolve", {
        sessionId,
        approvalId,
        decision,
      });
      await this.refreshSnapshot();
      await this.loadEvents(sessionId);
    } catch (err) {
      this.applyCommandFailure(err);
      await this.refreshSnapshot().catch(() => undefined);
      throw err;
    } finally {
      this.dispatch({ type: "setCommandBusy", busy: false });
    }
  }

  /** Inspect runtime availability (typed processes view). */
  async inspectRuntime(sessionId?: string): Promise<unknown> {
    return invokeTracer("tracer_runtime_status", sessionId ? { sessionId } : {});
  }

  private applyCommandFailure(err: unknown): void {
    const mapped = mapInvokeError(err);
    this.dispatch({
      type: "setError",
      message: mapped.message,
      kind: mapped.kind,
    });
    if (mapped.banner !== "none") {
      this.dispatch({ type: "setGlobalBanner", banner: mapped.banner });
    }
  }
}

/** Demo ids still used by mock backend / smoke controls. */
export const DEMO_IDS = MOCK_BACKEND_IDS;

/** Install scenario on active mock backend (tests / gallery). */
export function applyMockScenario(scenario: MockScenario): void {
  const backend = getMockBackend();
  if (backend) {
    backend.reset(scenario);
  } else {
    setMockBackend(createMockBackend(scenario));
  }
}