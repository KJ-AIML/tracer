import { useReducer, type ReactElement } from "react";
import { AppShell } from "./app/AppShell";
import { AboutPage } from "./app/AboutPage";
import { ErrorBoundary } from "./app/ErrorBoundary";
import { PresentationGallery } from "./app/PresentationGallery";
import { ProjectsHome } from "./features/projects/ProjectsHome";
import { ProjectWorkspace } from "./features/projects/ProjectWorkspace";
import { SessionWorkspacePlaceholder } from "./features/sessions";
import { createInitialMockState, mockReducer } from "./shared/store/mockStore";

function Router(): ReactElement {
  const [state, dispatch] = useReducer(mockReducer, undefined, createInitialMockState);

  let page: ReactElement;
  switch (state.route.name) {
    case "projects":
      page = <ProjectsHome state={state} dispatch={dispatch} />;
      break;
    case "project":
      page = (
        <ProjectWorkspace
          state={state}
          projectId={state.route.projectId}
          dispatch={dispatch}
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
      page = <ProjectsHome state={state} dispatch={dispatch} />;
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
