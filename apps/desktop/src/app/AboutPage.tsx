import type { ReactElement } from "react";
import { getInvokeMode, TRACER_EVENTS_CHANNEL } from "../shared/commands/invoke";

export function AboutPage(): ReactElement {
  return (
    <div className="panel layout-stack">
      <h1 className="panel__title">About Tracer</h1>
      <p>
        Tracer is a desktop control plane for AI coding agents — session-centric, not a full IDE.
      </p>
      <ul className="list__meta">
        <li>Module: W1-A Desktop Shell</li>
        <li>Invoke mode: {getInvokeMode()}</li>
        <li>Event channel (contract): {TRACER_EVENTS_CHANNEL}</li>
        <li>App info command: tracer_app_info (W1-F)</li>
      </ul>
      <p className="list__meta">
        Mock store only. No raw ACP parsing. Control plane handoff documented in docs/modules/w1-a/.
      </p>
    </div>
  );
}
