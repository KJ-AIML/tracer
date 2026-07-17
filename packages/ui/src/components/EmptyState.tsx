import type { ReactElement, ReactNode } from "react";
import { Icon } from "../icons";

export interface EmptyStateProps {
  title: string;
  body?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ title, body, action, className }: EmptyStateProps): ReactElement {
  const classes = ["tracer-empty-state", "tracer-presentation", className]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={classes} data-presentation="empty" role="status">
      <Icon name="empty" title="Empty" />
      <h2 className="tracer-presentation__title">{title}</h2>
      {body ? <p className="tracer-presentation__body">{body}</p> : null}
      {action}
    </div>
  );
}
