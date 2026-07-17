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

  let page: ReactElement;
  if (state.loadPhase === "loading" || state.loadPhase === "idle") {
    page = <LoadingState label="Loading presentation snapshot…" />;
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
    <AppShell state={state} dispatch={dispatch}>
      {page}
    </AppShell>
  );
}

export function App(): ReactElement {
  return (
    <ErrorBoundary>
      <Router />
    </ErrorBoundary>
  );
}