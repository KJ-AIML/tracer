import type { Dispatch, ReactElement } from "react";
import { Button, EmptyState, PresentationContainer } from "@tracer/ui";
import { MOCK_IDS, type MockAction, type MockState } from "../../shared/store/mockStore";

interface Props {
  state: MockState;
  dispatch: Dispatch<MockAction>;
}

export function ProjectsHome({ state, dispatch }: Props): ReactElement {
  if (state.projects.length === 0) {
    return (
      <div className="layout-stack">
        <EmptyState
          title="No projects yet"
          body="Open a local repository to manage agent sessions with Tracer."
          action={
            <Button
              variant="primary"
              onClick={() => dispatch({ type: "restoreProjects" })}
            >
              Open a local repository (mock)
            </Button>
          }
        />
      </div>
    );
  }

  return (
    <div className="layout-stack">
      <div className="layout-row" style={{ justifyContent: "space-between" }}>
        <h1 className="panel__title" style={{ margin: 0 }}>
          Projects
        </h1>
        <div className="layout-row">
          <Button
            variant="ghost"
            onClick={() => dispatch({ type: "clearProjects" })}
          >
            Simulate empty
          </Button>
          <Button variant="primary" disabled disabledReason="W1-F wires tracer_project_register">
            Register project
          </Button>
        </div>
      </div>

      <ul className="list" aria-label="Project list">
        {state.projects.map((p) => (
          <li key={p.projectId} className="list__item">
            <div>
              <div>{p.name}</div>
              <div className="list__meta">
                {p.status === "ready" ? "Ready" : p.status === "missing" ? "Folder missing" : "Invalid project"}
                {" · "}
                <span>{p.rootPath}</span>
              </div>
            </div>
            <Button
              variant="primary"
              onClick={() =>
                dispatch({
                  type: "navigate",
                  route: { name: "project", projectId: p.projectId },
                })
              }
            >
              Open
            </Button>
          </li>
        ))}
      </ul>

      <PresentationContainer
        kind="empty"
        title="Register more repositories later"
        body="Path dialogs and tracer_project_* commands land with W1-F control plane."
      />

      <p className="list__meta">
        Demo project id: <code>{MOCK_IDS.projectId}</code>
      </p>
    </div>
  );
}
