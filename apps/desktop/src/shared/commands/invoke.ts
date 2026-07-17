/**
 * Tauri command invocation wrapper (contract names only).
 *
 * VS1-H2: typed command surface + mock backend for browser/tests.
 * Tauri environments prefer real control-plane commands.
 * React never parses raw ACP; commands return typed JSON only.
 *
 * Command names from docs/contracts/TAURI_COMMAND_CONTRACT_V1.md + W1-F snapshot/heli.
 */

import type { CommandError } from "../types/tracer";
import type { MockBackend } from "./mockBackend";

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
  | "tracer_app_info"
  | "tracer_presentation_snapshot"
  | "tracer_heli_status";

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

export type InvokeMode = "mock" | "tauri" | "auto";

let mode: InvokeMode = "auto";
let mockBackend: MockBackend | null = null;

export function setInvokeMode(next: InvokeMode): void {
  mode = next;
}

export function getInvokeMode(): InvokeMode {
  return mode;
}

/** Install deterministic mock backend (browser dev + unit tests). */
export function setMockBackend(backend: MockBackend | null): void {
  mockBackend = backend;
}

export function getMockBackend(): MockBackend | null {
  return mockBackend;
}

function isTauriAvailable(): boolean {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const tauri = (globalThis as any).__TAURI__;
  return Boolean(tauri?.core?.invoke);
}

/**
 * Effective backend: Tauri when available (unless forced mock), else mock backend.
 * Production desktop must prefer real commands.
 */
export function resolveInvokeBackend(): "tauri" | "mock" {
  if (mode === "mock") return "mock";
  if (mode === "tauri") return "tauri";
  // auto
  return isTauriAvailable() ? "tauri" : "mock";
}

/**
 * invokeTracer — request/response only. High-frequency traffic uses tracer://events.
 * Mock mode keeps shell usable offline; Tauri mode uses W1-F control plane commands.
 */
export async function invokeTracer<TResult = unknown>(
  command: TracerCommandName,
  args?: Record<string, unknown>,
): Promise<TResult> {
  const backend = resolveInvokeBackend();

  if (backend === "tauri") {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const tauri = (globalThis as any).__TAURI__;
    if (!tauri?.core?.invoke) {
      throw new TracerInvokeError({
        errorClass: "InternalError",
        message: "Tauri invoke API not available in this environment.",
        retryable: false,
      });
    }
    try {
      // Tauri command args: W1-F handlers use flat args or { args: {...} } depending on command.
      // Contract front-end uses Record; desktop glue accepts camelCase fields.
      return (await tauri.core.invoke(command, normalizeTauriArgs(command, args))) as TResult;
    } catch (e: unknown) {
      if (typeof e === "string") {
        try {
          const parsed = JSON.parse(e) as {
            errorClass?: string;
            message?: string;
            retryable?: boolean;
            details?: Record<string, unknown>;
          };
          throw new TracerInvokeError({
            errorClass: parsed.errorClass ?? "InternalError",
            message: parsed.message ?? e,
            retryable: parsed.retryable ?? false,
            details: parsed.details,
          });
        } catch (inner) {
          if (inner instanceof TracerInvokeError) throw inner;
        }
      }
      if (e instanceof TracerInvokeError) throw e;
      throw new TracerInvokeError({
        errorClass: "InternalError",
        message: e instanceof Error ? e.message : String(e),
        retryable: false,
      });
    }
  }

  // mock backend
  if (!mockBackend) {
    throw new TracerInvokeError({
      errorClass: "Unsupported",
      message: `Mock shell: ${command} not available. Install mock backend or run desktop app.`,
      retryable: false,
      details: { command, mode },
    });
  }

  return mockBackend.handle<TResult>(command, args);
}

/**
 * Normalize args for Tauri handlers.
 * Some commands take a single structured `args` param; others take flat named params.
 */
function normalizeTauriArgs(
  command: TracerCommandName,
  args?: Record<string, unknown>,
): Record<string, unknown> {
  if (!args) return {};
  // Commands with flat sessionId / projectId top-level in Rust signatures:
  const flatTopLevel: TracerCommandName[] = [
    "tracer_project_get",
    "tracer_session_get",
    "tracer_approval_list_pending",
    "tracer_runtime_status",
  ];
  if (flatTopLevel.includes(command)) {
    return args;
  }
  // Structured-arg commands: pass as `args` object matching Rust Deserialize params.
  const structured: TracerCommandName[] = [
    "tracer_project_register",
    "tracer_session_list",
    "tracer_session_create",
    "tracer_session_submit_prompt",
    "tracer_session_cancel",
    "tracer_session_stop",
    "tracer_events_list",
    "tracer_approval_resolve",
  ];
  if (structured.includes(command)) {
    return { args };
  }
  return args;
}