import { useEffect, useState, type Dispatch, type ReactElement } from "react";
import { Button, EmptyState, StatusChip } from "@tracer/ui";
import type { AppAction, AppViewState, SnapshotJourney } from "../../shared/store/snapshotStore";

/** Fake ACP catalog scenarios used by GUI journeys (not live provider). */
const FAKE_SCENARIOS = [
  { id: "happy_prompt_stream", label: "Happy stream" },
  { id: "permission_allow", label: "Permission (allow path)" },
  { id: "permission_deny", label: "Permission (deny path)" },
  { id: "cancel_while_permission_pending", label: "Cancel while approval pending" },
  { id: "cancel_mid_stream", label: "Cancel mid-stream" },
  { id: "crash_nonzero_exit", label: "Crash (nonzero exit)" },
  { id: "eof_mid_prompt", label: "EOF mid-prompt" },
  { id: "auth_required_session_new", label: "Auth required" },
] as const;

interface Props {
  state: AppViewState;
  projectId: string;
  dispatch: Dispatch<AppAction>;
  journey: SnapshotJourney;
}

export function ProjectWorkspace({
  state,
  projectId,
  dispatch,
  journey,
}: Props): ReactElement {
  const project = state.projects.find((p) => p.projectId === projectId);
  const sessions = state.sessionsByProject[projectId] ?? [];
  const [sessionTitle, setSessionTitle] = useState("New session");
  const [scenarioId, setScenarioId] = useState<string>("happy_prompt_stream");
  const [createError, setCreateError] = useState<string | null>(null);

  useEffect(() => {
    void journey.loadSessions(projectId);
  }, [journey, projectId]);

  if (!project) {
    return (
      <div data-testid="tracer-project-not-found">
        <EmptyState
          title="Project not found"
          body="This project is not in the current snapshot/project list."
          action={
            <Button
              data-testid="tracer-back-projects"
              onClick={() => dispatch({ type: "navigate", route: { name: "projects" } })}
            >
              Back to projects
            </Button>
          }
        />
      </div>
    );
  }

  const onCreate = async (): Promise<void> => {
    setCreateError(null);
    try {
      await journey.createSession(projectId, sessionTitle.trim() || "New session", {
        runtimeKind: "acp-stdio",
        scenarioId,
      });
    } catch (e) {
      setCreateError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="layout-stack" data-testid="tracer-project-workspace" data-project-id={projectId}>
      <div className="layout-row">
        <Button
          variant="ghost"
          data-testid="tracer-back-projects"
          onClick={() => dispatch({ type: "navigate", route: { name: "projects" } })}
        >
          ← Projects
        </Button>
      </div>

      <header className="panel" data-testid="tracer-project-header">
        <h1 className="panel__title" data-testid="tracer-project-title">
          {project.name}
        </h1>
        <p className="list__meta">
          Path status:{" "}
          <strong>
            {project.status === "ready"
              ? "Ready"
              : project.status === "missing"
                ? "Folder missing"
                : "Invalid project"}
          </strong>
          {" · "}
          {project.rootPath}
        </p>
      </header>

      <section className="panel" aria-label="Sessions" data-testid="tracer-sessions-panel">
        <div className="layout-row" style={{ justifyContent: "space-between" }}>
          <h2 className="panel__title" style={{ margin: 0 }}>
            Sessions
          </h2>
        </div>

        <div className="layout-stack" data-testid="tracer-session-create">
          <label htmlFor="session-title">
            Session title
            <input
              id="session-title"
              data-testid="tracer-session-title"
              type="text"
              value={sessionTitle}
              onChange={(e) => setSessionTitle(e.target.value)}
            />
          </label>
          <label htmlFor="session-scenario">
            Fake ACP scenario
            <select
              id="session-scenario"
              data-testid="tracer-session-scenario"
              value={scenarioId}
              onChange={(e) => setScenarioId(e.target.value)}
            >
              {FAKE_SCENARIOS.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.label}
                </option>
              ))}
            </select>
          </label>
          <div className="layout-row">
            <Button
              variant="primary"
              data-testid="tracer-session-create-submit"
              disabled={state.commandBusy}
              disabledReason={state.commandBusy ? "Working…" : undefined}
              onClick={() => {
                void onCreate();
              }}
            >
              Create session
            </Button>
            {createError ? (
              <p className="composer__helper" role="alert" data-testid="tracer-session-create-error">
                {createError}
              </p>
            ) : null}
          </div>
        </div>

        {sessions.length === 0 ? (
          <EmptyState
            title="No sessions yet"
            body="Create a session to run an agent against this project."
          />
        ) : (
          <ul className="list" aria-label="Session list" data-testid="tracer-session-list">
            {sessions.map((s) => {
              const focused =
                state.snapshot.activeSessionId === s.sessionId ||
                (state.route.name === "session" &&
                  "sessionId" in state.route &&
                  state.route.sessionId === s.sessionId);
              return (
                <li
                  key={s.sessionId}
                  className="list__item"
                  data-testid={`tracer-session-item-${s.sessionId}`}
                  data-session-id={s.sessionId}
                  data-session-focused={focused ? "true" : "false"}
                >
                  <div>
                    <div data-testid="tracer-session-item-title">{s.title}</div>
                    <div className="list__meta">
                      <StatusChip status={s.status} />
                      {focused ? (
                        <span data-testid="tracer-session-focus-marker"> · focused</span>
                      ) : null}
                    </div>
                  </div>
                  <Button
                    variant="primary"
                    data-testid={`tracer-session-open-${s.sessionId}`}
                    onClick={() => {
                      void journey.openSession(s.projectId, s.sessionId);
                    }}
                  >
                    Open
                  </Button>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}
