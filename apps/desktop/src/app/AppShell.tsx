import type { Dispatch, ReactElement, ReactNode } from "react";
import { Button } from "@tracer/ui";
import type { AppAction, AppViewState } from "../shared/store/snapshotStore";
import { GlobalStatusRegion } from "./GlobalStatusRegion";

interface Props {
  state: AppViewState;
  dispatch: Dispatch<AppAction>;
  children: ReactNode;
}

export function AppShell({ state, dispatch, children }: Props): ReactElement {
  const backend = state.demoRuntime ? "mock" : "tauri";
  return (
    <div
      className="app-shell tracer-ui"
      data-testid="tracer-app-shell"
      data-tracer-backend={backend}
    >
      <header className="app-shell__header" data-testid="tracer-app-header">
        <div className="layout-row">
          <span className="app-shell__brand" data-testid="tracer-brand">
            Tracer
          </span>
          <span
            className="demo-badge"
            data-testid="tracer-backend-badge"
            data-tracer-backend={backend}
            title={
              state.demoRuntime
                ? "Mock command backend — not live model output"
                : "Tauri control-plane commands (fail-closed; no silent mock)"
            }
          >
            {state.demoRuntime
              ? "Demo runtime · not live model output"
              : "Tauri mode · control plane"}
          </span>
        </div>
        <nav className="app-shell__nav" aria-label="Primary" data-testid="tracer-primary-nav">
          <Button
            variant="ghost"
            data-testid="tracer-nav-projects"
            aria-current={state.route.name === "projects" ? "page" : undefined}
            onClick={() => dispatch({ type: "navigate", route: { name: "projects" } })}
          >
            Projects
          </Button>
          <Button
            variant="ghost"
            data-testid="tracer-nav-about"
            aria-current={state.route.name === "about" ? "page" : undefined}
            onClick={() => dispatch({ type: "navigate", route: { name: "about" } })}
          >
            About
          </Button>
          <Button
            variant="ghost"
            data-testid="tracer-nav-presentation"
            aria-current={state.route.name === "presentation-gallery" ? "page" : undefined}
            onClick={() => dispatch({ type: "navigate", route: { name: "presentation-gallery" } })}
          >
            Presentation states
          </Button>
        </nav>
      </header>

      <div
        className="app-shell__global"
        aria-label="Global status"
        data-testid="tracer-global-status"
      >
        {state.lastErrorMessage && state.loadPhase === "failed" ? (
          <div
            data-testid="tracer-invoke-error"
            role="alert"
            className="layout-stack"
            style={{ marginBottom: "0.5rem" }}
          >
            <strong>Command failed (fail-closed)</strong>
            <p>{state.lastErrorMessage}</p>
            <p className="list__meta">
              Backend remains {backend}; Tracer never silently falls back to mock after a Tauri
              invoke failure.
            </p>
          </div>
        ) : null}
        <GlobalStatusRegion
          banner={state.globalBanner}
          heliSummary={state.heli.summary}
          onDismiss={() => dispatch({ type: "setGlobalBanner", banner: "none" })}
        />
      </div>

      <main className="app-shell__main" data-testid="tracer-main">
        {children}
      </main>
    </div>
  );
}