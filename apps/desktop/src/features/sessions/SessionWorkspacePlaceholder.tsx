/**
 * Session workspace SHELL placeholder (W1-A).
 * Feature bodies for timeline/approvals/changes/terminal belong to later modules.
 * Layout regions follow SESSION_SCREEN_SPEC; data is mock-store only.
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
  composerDisabledReason,
  isCancelVisible,
  isComposerEnabled,
  type MockAction,
  type MockState,
} from "../../shared/store/mockStore";

interface Props {
  state: MockState;
  projectId: string;
  sessionId: string;
  dispatch: Dispatch<MockAction>;
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
      return null; // dedicated interrupt below
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
}: Props): ReactElement {
  const status = state.activeSessionStatus;
  const runtime = state.runtimeObservation;
  const composerOn = isComposerEnabled(status, runtime);
  const disabledReason = composerDisabledReason(status, runtime);
  const cancelVisible = isCancelVisible(status);

  const leave = (): void => {
    if (status === "running" || status === "awaiting_approval") {
      const ok = window.confirm(
        "Session is still active (running or awaiting approval). Leave anyway? (Mock shell does not stop the runtime.)",
      );
      if (!ok) return;
    }
    dispatch({ type: "navigate", route: { name: "project", projectId } });
  };

  return (
    <div className="session-workspace">
      <header className="session-workspace__header">
        <div className="layout-row">
          <Button variant="ghost" onClick={leave}>
            ← Sessions
          </Button>
          <strong>Demo session</strong>
          <StatusChip status={status} sublabel={state.lastError ?? undefined} />
          <RuntimePill observation={runtime} />
        </div>
        <div className="layout-row">
          {cancelVisible ? (
            <Button
              variant="default"
              onClick={() => dispatch({ type: "setSessionStatus", status: "cancelling" })}
            >
              Cancel
            </Button>
          ) : null}
          <Button
            variant="danger"
            disabled={status === "stopped"}
            disabledReason={status === "stopped" ? "Already stopped" : undefined}
            onClick={() => dispatch({ type: "simulateCancel" })}
          >
            Stop
          </Button>
        </div>
      </header>

      <div className="layout-stack" aria-label="Session banners">
        {runtime === "sign_in_required" ? (
          <Banner severity="warning" title="Sign in required to use this agent runtime" live="assertive">
            <p>Choose an authentication method, then continue. Composer stays disabled until ready.</p>
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
        {state.lastError && status === "failed" ? (
          <Banner severity="error" title="Session failed" live="assertive">
            <p>{state.lastError}</p>
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
              body="Send a prompt to begin. (Mock store — no real agent.)"
            />
          ) : null}
          {presentationForStatus(status)}
          {status === "awaiting_approval" ? (
            <PresentationContainer
              kind="approval"
              sessionStatus={status}
              actions={
                <>
                  <Button
                    variant="primary"
                    onClick={() => dispatch({ type: "setSessionStatus", status: "running" })}
                  >
                    Allow (mock)
                  </Button>
                  <Button
                    variant="danger"
                    onClick={() => dispatch({ type: "setSessionStatus", status: "running" })}
                  >
                    Deny (mock)
                  </Button>
                  <Button variant="ghost" onClick={() => dispatch({ type: "simulateCancel" })}>
                    Cancel request
                  </Button>
                </>
              }
            >
              <p className="list__meta">
                Risk: Unknown — review carefully. Fail closed: never auto-allow.
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
              <p>No plan yet. Plans appear when the agent shares one. (Feature body: later module.)</p>
            )}
            {state.sideTab === "approvals" && (
              <p>
                Pending approvals appear here. Resolve via tracer_approval_resolve (W1-F). Fail closed.
              </p>
            )}
            {state.sideTab === "changes" && (
              <p>No file changes reported yet. Not a full VCS client.</p>
            )}
            {state.sideTab === "runtime" && (
              <p>
                Runtime diagnostics placeholder. Process lifecycle owned by W1-C/W1-F. Observation:{" "}
                <RuntimePill observation={runtime} />
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
            disabled={!composerOn}
            disabledReason={disabledReason ?? undefined}
            onClick={() => dispatch({ type: "simulatePromptSubmit" })}
          >
            Send
          </Button>
          {disabledReason ? <p className="composer__helper">{disabledReason}</p> : null}
        </div>
      </div>

      <footer className="session-footer">
        <span>mock · session {sessionId.slice(0, 8)}…</span>
        <span>seq {state.events.length}</span>
        <span>last error: {state.lastError ?? "—"}</span>
      </footer>

      <section className="panel" aria-label="Mock session controls">
        <h2 className="panel__title">Mock controls (W1-A smoke)</h2>
        <div className="mock-controls">
          <label>
            Status{" "}
            <select
              value={status}
              onChange={(e) =>
                dispatch({
                  type: "setSessionStatus",
                  status: e.target.value as SessionStatus,
                })
              }
            >
              {(
                [
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
                ] as SessionStatus[]
              ).map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </label>
          <Button onClick={() => dispatch({ type: "simulateApproval" })}>Approval</Button>
          <Button onClick={() => dispatch({ type: "simulateDisconnect" })}>Disconnect</Button>
          <Button onClick={() => dispatch({ type: "simulateComplete" })}>Complete</Button>
          <Button onClick={() => dispatch({ type: "simulateCancel" })}>Cancel/Stop</Button>
          <Button
            onClick={() =>
              dispatch({ type: "simulateFail", message: "Mock failure: capability mismatch" })
            }
          >
            Fail
          </Button>
          <Button
            onClick={() =>
              dispatch({ type: "setRuntimeObservation", observation: "sign_in_required" })
            }
          >
            Auth gate
          </Button>
          <Button onClick={() => dispatch({ type: "resetDemo" })}>Reset demo</Button>
        </div>
      </section>
    </div>
  );
}
