import type { Dispatch, ReactElement } from "react";
import { Button, EmptyState, LoadingState, PresentationContainer } from "@tracer/ui";
import type { AppAction, AppViewState, SnapshotJourney } from "../../shared/store/snapshotStore";

interface Props {
  state: AppViewState;
  dispatch: Dispatch<AppAction>;
  journey: SnapshotJourney;
}

export function ProjectsHome({ state, dispatch, journey }: Props): ReactElement {
  if (state.loadPhase === "loading") {
    return <LoadingState label="Loading projects…" />;
  }

  if (state.projects.length === 0) {
    return (
      <div className="layout-stack">
        <EmptyState
          title="No projects yet"
          body="Open a local repository to manage agent sessions with Tracer."
          action={
            <Button
              variant="primary"
              onClick={() => {
                void journey.loadProjects();
              }}
            >
              Refresh projects
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
            onClick={() => {
              void journey.refreshSnapshot();
              void journey.loadProjects();
            }}
          >
            Refresh snapshot
          </Button>
          <Button
            variant="primary"
            disabled
            disabledReason="Native folder picker wires with host dialogs; register via tracer_project_register"
          >
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
                {p.status === "ready"
                  ? "Ready"
                  : p.status === "missing"
                    ? "Folder missing"
                    : "Invalid project"}
                {" · "}
                <span>{p.rootPath}</span>
              </div>
            </div>
            <Button
              variant="primary"
              onClick={() => {
                dispatch({
                  type: "navigate",
                  route: { name: "project", projectId: p.projectId },
                });
                void journey.loadSessions(p.projectId);
              }}
            >
              Open
            </Button>
          </li>
        ))}
      </ul>

      <PresentationContainer
        kind="empty"
        title="Typed snapshot source of truth"
        body="Projects come from tracer_project_list. Presentation state refreshes via tracer_presentation_snapshot."
      />
    </div>
  );
}