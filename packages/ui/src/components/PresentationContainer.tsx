import type { ReactElement, ReactNode } from "react";
import type { PresentationKind, SessionStatus } from "../types";
import { Banner } from "./Banner";
import { EmptyState } from "./EmptyState";
import { LoadingState } from "./LoadingState";
import { StatusChip } from "./StatusChip";

export interface PresentationContainerProps {
  kind: PresentationKind;
  /** Optional override title. */
  title?: string;
  body?: string;
  /** Session status chip when relevant. */
  sessionStatus?: SessionStatus;
  actions?: ReactNode;
  children?: ReactNode;
  className?: string;
}

const DEFAULTS: Record<PresentationKind, { title: string; body: string; status?: SessionStatus }> =
  {
    empty: {
      title: "Nothing here yet",
      body: "Register a local repository or create a session to begin.",
    },
    loading: {
      title: "Loading…",
      body: "Please wait.",
      status: "creating",
    },
    running: {
      title: "Agent is working",
      body: "Live timeline updates appear here. Cancel is available while the run is active.",
      status: "running",
    },
    failed: {
      title: "Session failed",
      body: "Review the error details in diagnostics. Prompting is disabled.",
      status: "failed",
    },
    disconnected: {
      title: "Runtime disconnected",
      body: "The agent process exited while this session was active. Prompting is disabled.",
      status: "disconnected",
    },
    completed: {
      title: "Session completed",
      body: "Review plan and changes, or start a new session.",
      status: "completed",
    },
    cancelled: {
      title: "Session cancelled",
      body: "The run was cancelled or stopped. History remains available for review.",
      status: "stopped",
    },
    approval: {
      title: "Approval needed",
      body: "Review the request carefully. Tracer never auto-allows unknown risk.",
      status: "awaiting_approval",
    },
  };

/**
 * Product presentation containers for STATE_MATRIX §12 shorthand states.
 * Always includes text labels (and StatusChip when applicable) — not color-only.
 */
export function PresentationContainer({
  kind,
  title,
  body,
  sessionStatus,
  actions,
  children,
  className,
}: PresentationContainerProps): ReactElement {
  const defaults = DEFAULTS[kind];
  const resolvedTitle = title ?? defaults.title;
  const resolvedBody = body ?? defaults.body;
  const status = sessionStatus ?? defaults.status;
  const classes = ["tracer-presentation", `tracer-presentation--${kind}`, className]
    .filter(Boolean)
    .join(" ");

  if (kind === "empty") {
    return (
      <EmptyState
        title={resolvedTitle}
        body={resolvedBody}
        action={actions}
        className={className}
      />
    );
  }

  if (kind === "loading") {
    return <LoadingState label={resolvedTitle} className={className} />;
  }

  if (kind === "failed" || kind === "disconnected") {
    return (
      <div className={classes} data-presentation={kind}>
        <Banner
          severity="error"
          title={resolvedTitle}
          live="assertive"
          actions={actions}
        >
          <p>{resolvedBody}</p>
          {status ? (
            <div className="tracer-presentation__meta">
              <StatusChip status={status} />
            </div>
          ) : null}
          {children}
        </Banner>
      </div>
    );
  }

  if (kind === "approval") {
    return (
      <div
        className={classes}
        data-presentation={kind}
        role="alertdialog"
        aria-modal="false"
        aria-label={resolvedTitle}
        tabIndex={-1}
      >
        <Banner severity="warning" title={resolvedTitle} live="assertive" actions={actions}>
          <p>{resolvedBody}</p>
          {status ? (
            <div className="tracer-presentation__meta">
              <StatusChip status={status} />
            </div>
          ) : null}
          {children}
        </Banner>
      </div>
    );
  }

  // running | completed | cancelled
  return (
    <div className={classes} data-presentation={kind} role="status" aria-live="polite">
      {status ? (
        <div className="tracer-presentation__meta">
          <StatusChip status={status} />
        </div>
      ) : null}
      <h2 className="tracer-presentation__title">{resolvedTitle}</h2>
      <p className="tracer-presentation__body">{resolvedBody}</p>
      {children}
      {actions}
    </div>
  );
}
