import type { Dispatch, ReactElement } from "react";
import { Button, EmptyState, StatusChip } from "@tracer/ui";
import { MOCK_IDS, type MockAction, type MockState } from "../../shared/store/mockStore";

interface Props {
  state: MockState;
  projectId: string;
  dispatch: Dispatch<MockAction>;
}

export function ProjectWorkspace({ state, projectId, dispatch }: Props): ReactElement {
  const project = state.projects.find((p) => p.projectId === projectId);
  const sessions = state.sessionsByProject[projectId] ?? [];

  if (!project) {
    return (
      <EmptyState
        title="Project not found"
        body="This project is not in the mock store."
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
            onClick={() =>
              dispatch({
                type: "navigate",
                route: {
                  name: "session",
                  projectId: MOCK_IDS.projectId,
                  sessionId: MOCK_IDS.sessionId,
                },
              })
            }
          >
            Create session (mock)
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
                  onClick={() =>
                    dispatch({
                      type: "navigate",
                      route: {
                        name: "session",
                        projectId: s.projectId,
                        sessionId: s.sessionId,
                      },
                    })
                  }
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
