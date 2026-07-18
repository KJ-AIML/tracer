import { useEffect, useMemo, useReducer, useRef, type ReactElement } from "react";
import { AppShell } from "./app/AppShell";
import { AboutPage } from "./app/AboutPage";
import { ErrorBoundary } from "./app/ErrorBoundary";
import { PresentationGallery } from "./app/PresentationGallery";
import { ProjectsHome } from "./features/projects/ProjectsHome";
import { ProjectWorkspace } from "./features/projects/ProjectWorkspace";
import { SessionWorkspacePlaceholder } from "./features/sessions";
import {
  appReducer,
  createInitialAppState,
  SnapshotJourney,
} from "./shared/store/snapshotStore";
import { LoadingState } from "@tracer/ui";

function Router(): ReactElement {
  const [state, dispatch] = useReducer(appReducer, undefined, createInitialAppState);
  const stateRef = useRef(state);
  stateRef.current = state;

  const journey = useMemo(
    () => new SnapshotJourney(() => stateRef.current, dispatch),
    [],
  );

  useEffect(() => {
    SnapshotJourney.bootstrap();
    void journey.bootstrapLoad();
  }, [journey]);

  const ready =
    state.loadPhase === "ready" || state.loadPhase === "failed";
  const backend = state.demoRuntime ? "mock" : "tauri";

  let page: ReactElement;
  if (state.loadPhase === "loading" || state.loadPhase === "idle") {
    page = (
      <div data-testid="tracer-loading" aria-busy="true">
        <LoadingState label="Loading presentation snapshot…" />
      </div>
    );
  } else {
    switch (state.route.name) {
      case "projects":
        page = <ProjectsHome state={state} dispatch={dispatch} journey={journey} />;
        break;
      case "project":
        page = (
          <ProjectWorkspace
            state={state}
            projectId={state.route.projectId}
            dispatch={dispatch}
            journey={journey}
          />
        );
        break;
      case "session":
        page = (
          <SessionWorkspacePlaceholder
            state={state}
            projectId={state.route.projectId}
            sessionId={state.route.sessionId}
            dispatch={dispatch}
            journey={journey}
          />
        );
        break;
      case "about":
        page = <AboutPage />;
        break;
      case "presentation-gallery":
        page = <PresentationGallery />;
        break;
      default:
        page = <ProjectsHome state={state} dispatch={dispatch} journey={journey} />;
    }
  }

  return (
    <div
      id="tracer-app-root"
      data-testid="tracer-app-root"
      data-tracer-ready={ready ? "true" : "false"}
      data-tracer-backend={backend}
      data-tracer-load-phase={state.loadPhase}
      data-tracer-route={state.route.name}
    >
      {/* Stable readiness marker for L3-J WebDriver waits (DOM + a11y). */}
      {ready ? (
        <div
          data-testid="tracer-app-ready"
          role="status"
          aria-live="polite"
          aria-label="Tracer application ready"
          style={{
            position: "absolute",
            width: 1,
            height: 1,
            overflow: "hidden",
            clip: "rect(0 0 0 0)",
          }}
        >
          ready:{backend}
        </div>
      ) : null}
      <AppShell state={state} dispatch={dispatch}>
        {page}
      </AppShell>
    </div>
  );
}

export function App(): ReactElement {
  return (
    <ErrorBoundary>
      <Router />
    </ErrorBoundary>
  );
}