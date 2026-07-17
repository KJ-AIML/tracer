import type { ReactElement } from "react";

export interface LoadingStateProps {
  /** Required text equivalent for spinner (reduced-motion / a11y). */
  label: string;
  className?: string;
}

export function LoadingState({ label, className }: LoadingStateProps): ReactElement {
  const classes = ["tracer-loading-state", "tracer-presentation", className]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={classes} data-presentation="loading" role="status" aria-live="polite">
      <span className="tracer-spinner" aria-hidden="true" />
      <p className="tracer-presentation__title">{label}</p>
    </div>
  );
}
