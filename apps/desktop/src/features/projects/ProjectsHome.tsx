import { useState, type Dispatch, type ReactElement } from "react";
import { Button, EmptyState, LoadingState, PresentationContainer } from "@tracer/ui";
import type { AppAction, AppViewState, SnapshotJourney } from "../../shared/store/snapshotStore";

interface Props {
  state: AppViewState;
  dispatch: Dispatch<AppAction>;
  journey: SnapshotJourney;
}

export function ProjectsHome({ state, dispatch, journey }: Props): ReactElement {
  const [rootPath, setRootPath] = useState("");
  const [projectName, setProjectName] = useState("");
  const [registerError, setRegisterError] = useState<string | null>(null);

  if (state.loadPhase === "loading") {
    return (
      <div data-testid="tracer-projects-loading">
        <LoadingState label="Loading projects…" />
      </div>
    );
  }

  const onRegister = async (): Promise<void> => {
    setRegisterError(null);
    const path = rootPath.trim();
    if (!path) {
      setRegisterError("Project root path is required");
      return;
    }
    try {
      await journey.registerProject(path, projectName.trim() || undefined);
      setRootPath("");
      setProjectName("");
    } catch (e) {
      setRegisterError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="layout-stack" data-testid="tracer-projects-home">
      <div className="layout-row" style={{ justifyContent: "space-between" }}>
        <h1 className="panel__title" style={{ margin: 0 }} data-testid="tracer-projects-title">
          Projects
        </h1>
        <div className="layout-row">
          <Button
            variant="ghost"
            data-testid="tracer-projects-refresh"
            onClick={() => {
              void journey.refreshSnapshot();
              void journey.loadProjects();
            }}
          >
            Refresh snapshot
          </Button>
        </div>
      </div>

      <section className="panel" aria-label="Register project" data-testid="tracer-project-register">
        <h2 className="panel__title" style={{ marginTop: 0 }}>
          Register project
        </h2>
        <p className="list__meta">
          Register a local repository path via <code>tracer_project_register</code>. Native folder
          picker remains optional; path entry enables automated GUI journeys.
        </p>
        <div className="layout-stack">
          <label htmlFor="project-root-path">
            Project root path
            <input
              id="project-root-path"
              data-testid="tracer-project-root-path"
              type="text"
              value={rootPath}
              onChange={(e) => setRootPath(e.target.value)}
              placeholder="Absolute path to a local folder"
              autoComplete="off"
            />
          </label>
          <label htmlFor="project-name">
            Display name (optional)
            <input
              id="project-name"
              data-testid="tracer-project-name"
              type="text"
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="tracer"
              autoComplete="off"
            />
          </label>
          <div className="layout-row">
            <Button
              variant="primary"
              data-testid="tracer-project-register-submit"
              disabled={state.commandBusy}
              disabledReason={state.commandBusy ? "Working…" : undefined}
              onClick={() => {
                void onRegister();
              }}
            >
              Register project
            </Button>
            {registerError ? (
              <p
                className="composer__helper"
                role="alert"
                data-testid="tracer-project-register-error"
              >
                {registerError}
              </p>
            ) : null}
          </div>
        </div>
      </section>

      {state.projects.length === 0 ? (
        <EmptyState
          title="No projects yet"
          body="Open a local repository to manage agent sessions with Tracer."
          action={
            <Button
              variant="primary"
              data-testid="tracer-projects-refresh-empty"
              onClick={() => {
                void journey.loadProjects();
              }}
            >
              Refresh projects
            </Button>
          }
        />
      ) : (
        <ul className="list" aria-label="Project list" data-testid="tracer-project-list">
          {state.projects.map((p) => (
            <li
              key={p.projectId}
              className="list__item"
              data-testid={`tracer-project-item-${p.projectId}`}
              data-project-id={p.projectId}
            >
              <div>
                <div data-testid="tracer-project-item-name">{p.name}</div>
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
                data-testid={`tracer-project-open-${p.projectId}`}
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
      )}

      <PresentationContainer
        kind="empty"
        title="Typed snapshot source of truth"
        body="Projects come from tracer_project_list. Presentation state refreshes via tracer_presentation_snapshot."
      />
    </div>
  );
}
