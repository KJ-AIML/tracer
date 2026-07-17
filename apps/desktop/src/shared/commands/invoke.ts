/**
 * Tauri command invocation wrapper (contract names only).
 *
 * W1-A: mock mode only — no real backend.
 * REPLACE_WHEN_W1F_CONTROL_PLANE_AVAILABLE — bind to @tauri-apps/api invoke.
 *
 * Command names from docs/contracts/TAURI_COMMAND_CONTRACT_V1.md.
 */

import type { CommandError } from "../types/tracer";

export type TracerCommandName =
  | "tracer_project_list"
  | "tracer_project_register"
  | "tracer_project_get"
  | "tracer_project_remove"
  | "tracer_session_list"
  | "tracer_session_create"
  | "tracer_session_get"
  | "tracer_session_submit_prompt"
  | "tracer_session_cancel"
  | "tracer_session_stop"
  | "tracer_approval_list_pending"
  | "tracer_approval_resolve"
  | "tracer_events_list"
  | "tracer_runtime_status"
  | "tracer_app_info";

/** Live event channel name (contract §2.2). */
export const TRACER_EVENTS_CHANNEL = "tracer://events";

export class TracerInvokeError extends Error {
  readonly errorClass: string;
  readonly retryable: boolean;
  readonly details?: Record<string, unknown>;

  constructor(err: CommandError) {
    super(err.message);
    this.name = "TracerInvokeError";
    this.errorClass = err.errorClass;
    this.retryable = err.retryable;
    this.details = err.details;
  }
}

export type InvokeMode = "mock" | "tauri";

let mode: InvokeMode = "mock";

export function setInvokeMode(next: InvokeMode): void {
  mode = next;
}

export function getInvokeMode(): InvokeMode {
  return mode;
}

/**
 * invokeTracer — request/response only. High-frequency traffic uses tracer://events.
 * Mock mode rejects with Unsupported until W1-F wires control plane.
 */
export async function invokeTracer<TResult = unknown>(
  command: TracerCommandName,
  _args?: Record<string, unknown>,
): Promise<TResult> {
  if (mode === "mock") {
    throw new TracerInvokeError({
      errorClass: "Unsupported",
      message: `Mock shell: ${command} is not wired. Use mock store; W1-F owns real control plane.`,
      retryable: false,
      details: { command, mode },
    });
  }

  // Tauri path reserved for W1-F composition.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const tauri = (globalThis as any).__TAURI__;
  if (!tauri?.core?.invoke) {
    throw new TracerInvokeError({
      errorClass: "InternalError",
      message: "Tauri invoke API not available in this environment.",
      retryable: false,
    });
  }
  return tauri.core.invoke(command, _args) as Promise<TResult>;
}
