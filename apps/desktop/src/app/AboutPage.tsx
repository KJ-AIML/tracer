import type { ReactElement } from "react";
import {
  getInvokeMode,
  resolveInvokeBackend,
  TRACER_EVENTS_CHANNEL,
} from "../shared/commands/invoke";

export function AboutPage(): ReactElement {
  return (
    <div className="panel layout-stack">
      <h1 className="panel__title">About Tracer</h1>
      <p>
        Tracer is a desktop control plane for AI coding agents — session-centric, not a full IDE.
      </p>
      <ul className="list__meta">
        <li>Module: VS1-H2 Desktop Snapshot Wiring</li>
        <li>Invoke mode: {getInvokeMode()} → backend {resolveInvokeBackend()}</li>
        <li>Event channel (contract): {TRACER_EVENTS_CHANNEL}</li>
        <li>Presentation: typed snapshots via tracer_presentation_snapshot</li>
        <li>App info command: tracer_app_info</li>
      </ul>
      <p className="list__meta">
        React receives typed snapshots only. No raw ACP parsing. No SQLite from UI. Process lifecycle
        owned by control plane.
      </p>
    </div>
  );
}