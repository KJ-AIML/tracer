/**
 * Session workspace — VS1-H2 snapshot/command driven.
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
  const composerOn = isComposerEnabled(status, runtime, auth) && !state.commandBusy;
  const disabledReason =
    state.commandBusy
      ? "Working…"
      : composerDisabledReason(status, runtime, auth);
  const cancelVisible = isCancelVisible(status);
  const cancelEnabled = isCancelEnabled(status) && !state.commandBusy;
  const pending = state.pendingApprovals[0];

  const leave = (): void => {
    if (status === "running" || status === "awaiting_approval") {
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
    <div className="session-workspace">
      <header className="session-workspace__header">
        <div className="layout-row">
          <Button variant="ghost" onClick={leave}>
            ← Sessions
          </Button>
          <strong>Session</strong>
          <StatusChip status={status} sublabel={state.lastErrorMessage ?? undefined} />
          <RuntimePill observation={runtime} />
        </div>
        <div className="layout-row">
          {cancelVisible ? (
            <Button
              variant="default"
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
            onClick={() => {
              void journey.refreshSnapshot().then(() => journey.loadEvents(sessionId));
            }}
          >
            Refresh snapshot
          </Button>
        </div>
      </header>

      <div className="layout-stack" aria-label="Session banners">
        {auth === "unauthenticated" || runtime === "sign_in_required" ? (
          <Banner severity="warning" title="Sign in required to use this agent runtime" live="assertive">
            <p>Choose an authentication method, then continue. Composer stays disabled until ready.</p>
          </Banner>
        ) : null}
        {auth === "failed" ? (
          <Banner severity="error" title="Sign-in failed" live="assertive">
            <p>{state.lastErrorMessage ?? "Authentication failed."}</p>
          </Banner>
        ) : null}
        {status === "disconnected" ? (
          <Banner severity="error" title="Runtime disconnected" live="assertive">
            <p>
              The agent process exited while this session was active. Prompting is disabled. Never show
              Running after exit.
            </p>
          </Banner>
        ) : null}
        {state.lastErrorMessage && status === "failed" ? (
          <Banner severity="error" title="Session failed" live="assertive">
            <p>{state.lastErrorMessage}</p>
          </Banner>
        ) : null}
        {!state.heli.available ? (
          <Banner severity="info" title="Heli unavailable" live="polite">
            <p>{state.heli.summary} — non-fatal; session continues.</p>
          </Banner>
        ) : null}
      </div>

      <div className="session-workspace__split">
        <section className="timeline-pane" aria-label="Timeline">
          <h2 className="panel__title">Timeline</h2>
          {status === "ready" && state.events.length === 0 ? (
            <PresentationContainer
              kind="empty"
              title="Session ready"
              body="Send a prompt to begin. Events arrive as typed normalized envelopes."
            />
          ) : null}
          {presentationForStatus(status)}
          {status === "awaiting_approval" && pending ? (
            <PresentationContainer
              kind="approval"
              sessionStatus={status}
              actions={
                <>
                  <Button
                    variant="primary"
                    disabled={state.commandBusy}
                    onClick={() => {
                      void journey.resolveApproval(sessionId, pending.approvalId, "allow");
                    }}
                  >
                    Allow
                  </Button>
                  <Button
                    variant="danger"
                    disabled={state.commandBusy}
                    onClick={() => {
                      void journey.resolveApproval(sessionId, pending.approvalId, "deny");
                    }}
                  >
                    Deny
                  </Button>
                  <Button
                    variant="ghost"
                    disabled={state.commandBusy}
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
          ) : null}
          {state.events.length > 0 ? (
            <ul className="list" aria-label="Normalized events">
              {state.events.map((e) => (
                <li key={e.eventId} className="list__item">
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

        <aside className="side-pane" aria-label="Side pane">
          <div className="side-pane__tabs" role="tablist" aria-label="Session side tabs">
            {(["plan", "approvals", "changes", "runtime"] as const).map((tab) => (
              <Button
                key={tab}
                variant="ghost"
                role="tab"
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
          <div role="tabpanel" className="list__meta">
            {state.sideTab === "plan" && (
              <p>No plan yet. Plans appear when the agent shares one.</p>
            )}
            {state.sideTab === "approvals" && (
              <div>
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

      <div className="composer">
        <label htmlFor="prompt">
          Prompt
          <textarea
            id="prompt"
            value={state.composerText}
            disabled={!composerOn}
            onChange={(e) => dispatch({ type: "setComposerText", text: e.target.value })}
            placeholder="Describe what the agent should do…"
          />
        </label>
        <div className="layout-row">
          <Button
            variant="primary"
            disabled={!composerOn || !state.composerText.trim()}
            disabledReason={disabledReason ?? undefined}
            onClick={() => {
              void journey.submitPrompt(sessionId, state.composerText);
            }}
          >
            Send
          </Button>
          {disabledReason ? <p className="composer__helper">{disabledReason}</p> : null}
        </div>
      </div>

      <footer className="session-footer">
        <span>
          {state.demoRuntime ? "mock" : "tauri"} · session {sessionId.slice(0, 8)}…
        </span>
        <span>seq {state.snapshot.latestSequence}</span>
        <span>auth: {state.authState}</span>
        <span>last error: {state.lastErrorMessage ?? "—"}</span>
      </footer>

      {state.demoRuntime ? (
        <section className="panel" aria-label="Mock scenario controls">
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
              <Button key={scenario} onClick={() => void runScenario(scenario)}>
                {label}
              </Button>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}