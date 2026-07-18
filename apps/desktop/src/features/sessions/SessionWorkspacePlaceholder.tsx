/**
 * Session workspace — VS1-H2 snapshot/command driven + W2.2-B GUI journey hooks.
 * Feature bodies for full editor/terminal/file explorer are out of scope.
 * Layout regions follow SESSION_SCREEN_SPEC; data from typed snapshots only.
 */
import type { Dispatch, ReactElement } from "react";
import {
  Banner,
  Button,
  PresentationContainer,
  RuntimePill,
  StatusChip,
  type SessionStatus,
} from "@tracer/ui";
import {
  applyMockScenario,
  composerDisabledReason,
  isCancelEnabled,
  isCancelVisible,
  isComposerEnabled,
  type AppAction,
  type AppViewState,
  type SnapshotJourney,
} from "../../shared/store/snapshotStore";
import type { MockScenario } from "../../shared/commands/mockBackend";

interface Props {
  state: AppViewState;
  projectId: string;
  sessionId: string;
  dispatch: Dispatch<AppAction>;
  journey: SnapshotJourney;
}

function presentationForStatus(status: SessionStatus): ReactElement | null {
  switch (status) {
    case "creating":
    case "starting_runtime":
    case "cancelling":
      return (
        <PresentationContainer
          kind="loading"
          title={
            status === "creating"
              ? "Creating session…"
              : status === "starting_runtime"
                ? "Starting runtime…"
                : "Cancelling…"
          }
          sessionStatus={status}
        />
      );
    case "running":
      return <PresentationContainer kind="running" sessionStatus={status} />;
    case "awaiting_approval":
      return null;
    case "failed":
      return <PresentationContainer kind="failed" sessionStatus={status} />;
    case "disconnected":
      return <PresentationContainer kind="disconnected" sessionStatus={status} />;
    case "completed":
      return <PresentationContainer kind="completed" sessionStatus={status} />;
    case "stopped":
      return <PresentationContainer kind="cancelled" sessionStatus={status} />;
    case "ready":
    default:
      return null;
  }
}

export function SessionWorkspacePlaceholder({
  state,
  projectId,
  sessionId,
  dispatch,
  journey,
}: Props): ReactElement {
  const status = state.sessionStatus;
  const runtime = state.runtimeObservation;
  const auth = state.authState;
  // Approval/cancel stay usable while a prompt invoke is outstanding (CP blocks
  // on agent run; concurrent resolve/cancel is required for deadlock-free GUI).
  const composerOn = isComposerEnabled(status, runtime, auth) && !state.commandBusy;
  const disabledReason =
    state.commandBusy
      ? "Working…"
      : composerDisabledReason(status, runtime, auth);
  const hasPendingApproval = state.pendingApprovals.length > 0;
  const cancelVisible = isCancelVisible(status) || hasPendingApproval;
  const cancelEnabled = isCancelEnabled(status) || hasPendingApproval;
  const pending = state.pendingApprovals[0];
  const backend = state.demoRuntime ? "mock" : "tauri";

  const leave = (): void => {
    // Skip blocking confirm under L3-J / automated WebDriver (window.__TRACER_E2E__).
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const e2e = Boolean((globalThis as any).__TRACER_E2E__);
    if (!e2e && (status === "running" || status === "awaiting_approval")) {
      const ok = window.confirm(
        "Session is still active (running or awaiting approval). Leave anyway? (Runtime lifecycle stays with control plane.)",
      );
      if (!ok) return;
    }
    dispatch({ type: "navigate", route: { name: "project", projectId } });
  };

  const runScenario = async (scenario: MockScenario): Promise<void> => {
    if (!state.demoRuntime) return;
    applyMockScenario(scenario);
    await journey.refreshSnapshot();
    await journey.loadEvents(sessionId);
  };

  return (
    <div
      className="session-workspace"
      data-testid="tracer-session-workspace"
      data-session-id={sessionId}
      data-session-status={status}
      data-runtime-observation={runtime}
      data-tracer-backend={backend}
    >
      <header className="session-workspace__header" data-testid="tracer-session-header">
        <div className="layout-row">
          <Button variant="ghost" data-testid="tracer-session-back" onClick={leave}>
            ← Sessions
          </Button>
          <strong data-testid="tracer-session-heading">Session</strong>
          <span data-testid="tracer-session-status" data-status={status}>
            <StatusChip status={status} sublabel={state.lastErrorMessage ?? undefined} />
          </span>
          <span data-testid="tracer-session-runtime" data-runtime={runtime}>
            <RuntimePill observation={runtime} />
          </span>
        </div>
        <div className="layout-row">
          {cancelVisible ? (
            <Button
              variant="default"
              data-testid="tracer-session-cancel"
              disabled={!cancelEnabled}
              disabledReason={!cancelEnabled ? "Cancel unavailable" : undefined}
              onClick={() => {
                void journey.cancelSession(sessionId);
              }}
            >
              Cancel
            </Button>
          ) : null}
          <Button
            variant="danger"
            data-testid="tracer-session-stop"
            disabled={status === "stopped" || state.commandBusy}
            disabledReason={
              status === "stopped" ? "Already stopped" : state.commandBusy ? "Working…" : undefined
            }
            onClick={() => {
              void journey.stopSession(sessionId);
            }}
          >
            Stop
          </Button>
          <Button
            variant="ghost"
            data-testid="tracer-session-refresh"
            onClick={() => {
              void journey.refreshSnapshot().then(() => journey.loadEvents(sessionId));
            }}
          >
            Refresh snapshot
          </Button>
        </div>
      </header>

      <div
        className="layout-stack"
        aria-label="Session banners"
        data-testid="tracer-session-banners"
      >
        {auth === "unauthenticated" || runtime === "sign_in_required" ? (
          <div data-testid="tracer-banner-auth-required">
            <Banner severity="warning" title="Sign in required to use this agent runtime" live="assertive">
              <p>Choose an authentication method, then continue. Composer stays disabled until ready.</p>
            </Banner>
          </div>
        ) : null}
        {auth === "failed" ? (
          <div data-testid="tracer-banner-auth-failed">
            <Banner severity="error" title="Sign-in failed" live="assertive">
              <p>{state.lastErrorMessage ?? "Authentication failed."}</p>
            </Banner>
          </div>
        ) : null}
        {status === "disconnected" || runtime === "crashed" ? (
          <div data-testid="tracer-banner-runtime-disconnected">
            <Banner severity="error" title="Runtime disconnected" live="assertive">
              <p>
                The agent process exited while this session was active. Prompting is disabled. Never
                show Running after exit.
              </p>
            </Banner>
          </div>
        ) : null}
        {state.lastErrorMessage && (status === "failed" || state.lastFailureKind) ? (
          <div data-testid="tracer-banner-session-error">
            <Banner severity="error" title="Session command error" live="assertive">
              <p>{state.lastErrorMessage}</p>
            </Banner>
          </div>
        ) : null}
        {!state.heli.available ? (
          <div data-testid="tracer-banner-heli-unavailable">
            <Banner severity="info" title="Heli unavailable" live="polite">
              <p>{state.heli.summary} — non-fatal; session continues.</p>
            </Banner>
          </div>
        ) : null}
      </div>

      <div className="session-workspace__split">
        <section className="timeline-pane" aria-label="Timeline" data-testid="tracer-timeline">
          <h2 className="panel__title">Timeline</h2>
          {status === "ready" && state.events.length === 0 ? (
            <PresentationContainer
              kind="empty"
              title="Session ready"
              body="Send a prompt to begin. Events arrive as typed normalized envelopes."
            />
          ) : null}
          <div data-testid="tracer-session-presentation">{presentationForStatus(status)}</div>
          {pending ? (
            <div data-testid="tracer-approval-card" data-approval-id={pending.approvalId}>
              <PresentationContainer
                kind="approval"
                sessionStatus={status === "awaiting_approval" ? status : "awaiting_approval"}
                actions={
                  <>
                    <Button
                      variant="primary"
                      data-testid="tracer-approval-allow"
                      onClick={() => {
                        void journey.resolveApproval(sessionId, pending.approvalId, "allow");
                      }}
                    >
                      Allow
                    </Button>
                    <Button
                      variant="danger"
                      data-testid="tracer-approval-deny"
                      onClick={() => {
                        void journey.resolveApproval(sessionId, pending.approvalId, "deny");
                      }}
                    >
                      Deny
                    </Button>
                    <Button
                      variant="ghost"
                      data-testid="tracer-approval-cancel-request"
                      onClick={() => {
                        void journey.resolveApproval(sessionId, pending.approvalId, "cancel");
                      }}
                    >
                      Cancel request
                    </Button>
                  </>
                }
              >
                <p className="list__meta">
                  <strong>{pending.action}</strong> — {pending.description}
                </p>
                <p className="list__meta">
                  Risk: {pending.risk} — review carefully. Fail closed: never auto-allow.
                </p>
              </PresentationContainer>
            </div>
          ) : null}
          {state.events.length > 0 ? (
            <ul className="list" aria-label="Normalized events" data-testid="tracer-event-list">
              {state.events.map((e) => (
                <li
                  key={e.eventId}
                  className="list__item"
                  data-testid={`tracer-event-${e.sequence}`}
                  data-event-type={e.type}
                  data-event-sequence={String(e.sequence)}
                >
                  <div>
                    <div>
                      <code>{e.type}</code>
                      {typeof e.payload?.text === "string" ? (
                        <span className="list__meta"> — {String(e.payload.text)}</span>
                      ) : null}
                    </div>
                    <div className="list__meta">
                      seq {e.sequence} · {e.timestamp}
                    </div>
                  </div>
                </li>
              ))}
            </ul>
          ) : null}
        </section>

        <aside className="side-pane" aria-label="Side pane" data-testid="tracer-side-pane">
          <div className="side-pane__tabs" role="tablist" aria-label="Session side tabs">
            {(["plan", "approvals", "changes", "runtime"] as const).map((tab) => (
              <Button
                key={tab}
                variant="ghost"
                role="tab"
                data-testid={`tracer-side-tab-${tab}`}
                aria-selected={state.sideTab === tab}
                onClick={() => dispatch({ type: "setSideTab", tab })}
              >
                {tab === "plan"
                  ? "Plan"
                  : tab === "approvals"
                    ? "Approvals"
                    : tab === "changes"
                      ? "Changes"
                      : "Runtime"}
              </Button>
            ))}
          </div>
          <div role="tabpanel" className="list__meta" data-testid="tracer-side-tab-panel">
            {state.sideTab === "plan" && (
              <p>No plan yet. Plans appear when the agent shares one.</p>
            )}
            {state.sideTab === "approvals" && (
              <div data-testid="tracer-approvals-panel">
                {state.pendingApprovals.length === 0 ? (
                  <p>No pending approvals. Fail closed: never auto-allow.</p>
                ) : (
                  <ul className="list">
                    {state.pendingApprovals.map((a) => (
                      <li key={a.approvalId} className="list__item">
                        <div>
                          <div>{a.action}</div>
                          <div className="list__meta">{a.description}</div>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}
            {state.sideTab === "changes" && (
              <p>No file changes reported yet. Not a full VCS client.</p>
            )}
            {state.sideTab === "runtime" && (
              <p>
                Runtime diagnostics from snapshot. Process lifecycle owned by control plane.
                Observation: <RuntimePill observation={runtime} />
              </p>
            )}
          </div>
        </aside>
      </div>

      <div className="composer" data-testid="tracer-composer">
        <label htmlFor="prompt">
          Prompt
          <textarea
            id="prompt"
            data-testid="tracer-prompt-input"
            value={state.composerText}
            disabled={!composerOn}
            onChange={(e) => dispatch({ type: "setComposerText", text: e.target.value })}
            placeholder="Describe what the agent should do…"
          />
        </label>
        <div className="layout-row">
          <Button
            variant="primary"
            data-testid="tracer-prompt-send"
            disabled={!composerOn || !state.composerText.trim()}
            disabledReason={disabledReason ?? undefined}
            onClick={() => {
              void journey.submitPrompt(sessionId, state.composerText);
            }}
          >
            Send
          </Button>
          {disabledReason ? (
            <p className="composer__helper" data-testid="tracer-composer-helper">
              {disabledReason}
            </p>
          ) : null}
        </div>
      </div>

      <footer className="session-footer" data-testid="tracer-session-footer">
        <span data-testid="tracer-session-footer-backend">
          {backend} · session {sessionId.slice(0, 8)}…
        </span>
        <span data-testid="tracer-session-footer-seq">seq {state.snapshot.latestSequence}</span>
        <span data-testid="tracer-session-footer-auth">auth: {state.authState}</span>
        <span data-testid="tracer-session-footer-error">
          last error: {state.lastErrorMessage ?? "—"}
        </span>
      </footer>

      {state.demoRuntime ? (
        <section className="panel" aria-label="Mock scenario controls" data-testid="tracer-mock-controls">
          <h2 className="panel__title">Mock scenarios (browser / tests only)</h2>
          <div className="mock-controls">
            {(
              [
                ["default", "Ready"],
                ["runtime_unavailable", "Runtime unavailable"],
                ["authentication_required", "Auth required"],
                ["approval_request", "Approval"],
                ["completed_run", "Completed"],
                ["runtime_crash", "Crash"],
                ["session_history_restore", "History restore"],
                ["heli_unavailable", "Heli unavailable"],
              ] as const
            ).map(([scenario, label]) => (
              <Button
                key={scenario}
                data-testid={`tracer-mock-scenario-${scenario}`}
                onClick={() => void runScenario(scenario)}
              >
                {label}
              </Button>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}
