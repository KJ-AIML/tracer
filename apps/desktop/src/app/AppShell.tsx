import type { Dispatch, ReactElement, ReactNode } from "react";
import { Button } from "@tracer/ui";
import type { MockAction, MockState } from "../shared/store/mockStore";
import { GlobalStatusRegion } from "./GlobalStatusRegion";

interface Props {
  state: MockState;
  dispatch: Dispatch<MockAction>;
  children: ReactNode;
}

export function AppShell({ state, dispatch, children }: Props): ReactElement {
  return (
    <div className="app-shell tracer-ui">
      <header className="app-shell__header">
        <div className="layout-row">
          <span className="app-shell__brand">Tracer</span>
          {state.demoRuntime ? (
            <span className="demo-badge" title="Synthetic mock store — not live model output">
              Demo runtime · not live model output
            </span>
          ) : null}
        </div>
        <nav className="app-shell__nav" aria-label="Primary">
          <Button
            variant="ghost"
            aria-current={state.route.name === "projects" ? "page" : undefined}
            onClick={() => dispatch({ type: "navigate", route: { name: "projects" } })}
          >
            Projects
          </Button>
          <Button
            variant="ghost"
            aria-current={state.route.name === "about" ? "page" : undefined}
            onClick={() => dispatch({ type: "navigate", route: { name: "about" } })}
          >
            About
          </Button>
          <Button
            variant="ghost"
            aria-current={state.route.name === "presentation-gallery" ? "page" : undefined}
            onClick={() => dispatch({ type: "navigate", route: { name: "presentation-gallery" } })}
          >
            Presentation states
          </Button>
        </nav>
      </header>

      <div className="app-shell__global" aria-label="Global status">
        <GlobalStatusRegion
          banner={state.globalBanner}
          onDismiss={() => dispatch({ type: "setGlobalBanner", banner: "none" })}
        />
      </div>

      <main className="app-shell__main">{children}</main>
    </div>
  );
}
