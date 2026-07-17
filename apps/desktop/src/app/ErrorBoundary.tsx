import { Component, type ErrorInfo, type ReactNode } from "react";
import { Banner, Button } from "@tracer/ui";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Shell-level only; no telemetry backend in W1-A.
    console.error("Tracer shell error boundary", error, info.componentStack);
  }

  private reset = (): void => {
    this.setState({ error: null });
  };

  render(): ReactNode {
    if (this.state.error) {
      return (
        <div className="error-boundary tracer-ui">
          <Banner
            severity="error"
            title="Something went wrong in the Tracer shell"
            live="assertive"
            actions={
              <Button variant="primary" onClick={this.reset}>
                Try again
              </Button>
            }
          >
            <p>A UI error was caught by the top-level error boundary. No session data was modified.</p>
            <pre>{this.state.error.message}</pre>
          </Banner>
        </div>
      );
    }
    return this.props.children;
  }
}
