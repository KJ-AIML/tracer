import type { ReactElement } from "react";

export type IconName =
  | "spinner"
  | "check"
  | "check-circle"
  | "activity"
  | "shield"
  | "error"
  | "unlink"
  | "stop"
  | "plug"
  | "plus"
  | "info"
  | "warning"
  | "empty";

interface IconProps {
  name: IconName;
  /** Accessible name when icon is not adjacent to visible text (A5). */
  title?: string;
  className?: string;
}

/** Lightweight SVG icons; decorative when title omitted and adjacent text present. */
export function Icon({ name, title, className }: IconProps): ReactElement {
  const common = {
    className: className ?? "tracer-status-chip__icon",
    width: 14,
    height: 14,
    viewBox: "0 0 16 16",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.6,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": title ? undefined : true,
    role: title ? ("img" as const) : undefined,
  };

  const body = (() => {
    switch (name) {
      case "spinner":
        return <circle cx="8" cy="8" r="5" strokeDasharray="20 10" />;
      case "check":
        return <path d="M3.5 8.5 6.5 11.5 12.5 4.5" />;
      case "check-circle":
        return (
          <>
            <circle cx="8" cy="8" r="6" />
            <path d="M5 8.2 7.1 10.2 11 5.8" />
          </>
        );
      case "activity":
        return <path d="M2 8h3l1.5-4 2.5 8 1.5-4H14" />;
      case "shield":
        return <path d="M8 2 13 4v4c0 3.2-2.1 5.2-5 6.5C5.1 13.2 3 11.2 3 8V4l5-2z" />;
      case "error":
        return (
          <>
            <circle cx="8" cy="8" r="6" />
            <path d="M8 5v3.5M8 11h.01" />
          </>
        );
      case "unlink":
        return (
          <>
            <path d="M6 10 3.8 12.2a2.2 2.2 0 1 1-3.1-3.1L3 6.8" />
            <path d="M10 6 12.2 3.8a2.2 2.2 0 1 1 3.1 3.1L13 9.2" />
            <path d="M6.5 9.5 9.5 6.5" />
          </>
        );
      case "stop":
        return <rect x="4" y="4" width="8" height="8" rx="1" />;
      case "plug":
        return <path d="M6 2v4M10 2v4M4 6h8v2a4 4 0 0 1-8 0V6zM8 12v2" />;
      case "plus":
        return <path d="M8 3v10M3 8h10" />;
      case "info":
        return (
          <>
            <circle cx="8" cy="8" r="6" />
            <path d="M8 7v4M8 5h.01" />
          </>
        );
      case "warning":
        return (
          <>
            <path d="M8 2 14.5 13H1.5L8 2z" />
            <path d="M8 6.5v3M8 11.5h.01" />
          </>
        );
      case "empty":
        return <rect x="3" y="4" width="10" height="8" rx="1.5" />;
      default:
        return <circle cx="8" cy="8" r="5" />;
    }
  })();

  return (
    <svg {...common}>
      {title ? <title>{title}</title> : null}
      {body}
    </svg>
  );
}
