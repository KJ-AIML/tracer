import { useEffect, type Dispatch, type ReactElement } from "react";
import { Button, EmptyState, StatusChip } from "@tracer/ui";
import type { AppAction, AppViewState, SnapshotJourney } from "../../shared/store/snapshotStore";

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

  useEffect(() => {
    void journey.loadSessions(projectId);
  }, [journey, projectId]);

  if (!project) {
    return (
      <EmptyState
        title="Project not found"
        body="This project is not in the current snapshot/project list."
        action={
          <Button onClick={() => dispatch({ type: "navigate", route: { name: "projects" } })}>
            Back to projects
          </Button>
        }
      />
    );
  }

  return (
    <div className="layout-stack">
      <div className="layout-row">
        <Button
          variant="ghost"
          onClick={() => dispatch({ type: "navigate", route: { name: "projects" } })}
        >
          ← Projects
        </Button>
      </div>

      <header className="panel">
        <h1 className="panel__title">{project.name}</h1>
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

      <section className="panel" aria-label="Sessions">
        <div className="layout-row" style={{ justifyContent: "space-between" }}>
          <h2 className="panel__title" style={{ margin: 0 }}>
            Sessions
          </h2>
          <Button
            variant="primary"
            disabled={state.commandBusy}
            disabledReason={state.commandBusy ? "Working…" : undefined}
            onClick={() => {
              void journey.createSession(projectId, "New session");
            }}
          >
            Create session
          </Button>
        </div>

        {sessions.length === 0 ? (
          <EmptyState
            title="No sessions yet"
            body="Create a session to run an agent against this project."
          />
        ) : (
          <ul className="list" aria-label="Session list">
            {sessions.map((s) => (
              <li key={s.sessionId} className="list__item">
                <div>
                  <div>{s.title}</div>
                  <div className="list__meta">
                    <StatusChip status={s.status} />
                  </div>
                </div>
                <Button
                  variant="primary"
                  onClick={() => {
                    void journey.openSession(s.projectId, s.sessionId);
                  }}
                >
                  Open
                </Button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}