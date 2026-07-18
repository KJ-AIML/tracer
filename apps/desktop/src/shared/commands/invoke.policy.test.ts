/**
 * W2-B invoke policy — Tauri detection, browser mock fallback, no silent downgrade.
 *
 * CI class: standard — network no, credentials no, live Grok no.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  getInvokeMode,
  invokeTracer,
  isTauriAvailable,
  resolveInvokeBackend,
  setInvokeMode,
  setMockBackend,
  TracerInvokeError,
} from "./invoke";
import { createMockBackend } from "./mockBackend";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const g = globalThis as any;

beforeEach(() => {
  delete g.__TAURI__;
  setInvokeMode("auto");
  setMockBackend(null);
});

afterEach(() => {
  delete g.__TAURI__;
  setInvokeMode("auto");
  setMockBackend(null);
  vi.restoreAllMocks();
});

describe("W2-B invoke policy", () => {
  it("A2: detects Tauri when __TAURI__.core.invoke present", () => {
    g.__TAURI__ = { core: { invoke: async () => ({}) } };
    expect(isTauriAvailable()).toBe(true);
    setInvokeMode("auto");
    expect(resolveInvokeBackend()).toBe("tauri");
  });

  it("A2: without Tauri, auto resolves to mock (browser fallback)", () => {
    expect(isTauriAvailable()).toBe(false);
    setInvokeMode("auto");
    expect(resolveInvokeBackend()).toBe("mock");
  });

  it("A14: browser fallback is deterministic with mock backend", async () => {
    setInvokeMode("auto");
    setMockBackend(createMockBackend("default"));
    expect(resolveInvokeBackend()).toBe("mock");
    const snap = await invokeTracer("tracer_presentation_snapshot");
    expect(snap).toMatchObject({ version: 1 });
  });

  it("A14: browser without mock still fails closed (no fake success)", async () => {
    setInvokeMode("auto");
    setMockBackend(null);
    await expect(invokeTracer("tracer_presentation_snapshot")).rejects.toBeInstanceOf(
      TracerInvokeError,
    );
  });

  it("A15: forced tauri mode without IPC → error, never mock", async () => {
    setInvokeMode("tauri");
    setMockBackend(createMockBackend("default"));
    expect(resolveInvokeBackend()).toBe("tauri");
    expect(isTauriAvailable()).toBe(false);

    await expect(invokeTracer("tracer_presentation_snapshot")).rejects.toMatchObject({
      errorClass: "InternalError",
      message: expect.stringMatching(/no silent mock downgrade/i),
    });
    // Mode remains tauri; mock was not used.
    expect(getInvokeMode()).toBe("tauri");
    expect(resolveInvokeBackend()).toBe("tauri");
  });

  it("A15: real Tauri invoke failure → error; no silent mock downgrade", async () => {
    const invoke = vi.fn(async () => {
      throw JSON.stringify({
        errorClass: "RuntimeSpawnFailed",
        message: "spawn failed",
        retryable: false,
      });
    });
    g.__TAURI__ = { core: { invoke } };
    setInvokeMode("auto");
    // Install mock — must NOT be used after Tauri failure.
    setMockBackend(createMockBackend("default"));
    expect(resolveInvokeBackend()).toBe("tauri");

    await expect(invokeTracer("tracer_session_create", { projectId: "x" })).rejects.toMatchObject({
      errorClass: "RuntimeSpawnFailed",
      message: "spawn failed",
    });
    expect(invoke).toHaveBeenCalledTimes(1);
    // Still tauri backend after failure.
    expect(resolveInvokeBackend()).toBe("tauri");
  });

  it("A15: real Tauri non-JSON throw still fails closed", async () => {
    g.__TAURI__ = {
      core: {
        invoke: async () => {
          throw new Error("IPC channel closed");
        },
      },
    };
    setInvokeMode("tauri");
    setMockBackend(createMockBackend("default"));

    await expect(invokeTracer("tracer_app_info")).rejects.toMatchObject({
      errorClass: "InternalError",
      message: expect.stringMatching(/IPC channel closed/),
      details: expect.objectContaining({ silentMockDowngrade: false }),
    });
  });

  it("forced mock ignores Tauri even if present", async () => {
    const invoke = vi.fn(async () => ({ version: 99 }));
    g.__TAURI__ = { core: { invoke } };
    setInvokeMode("mock");
    setMockBackend(createMockBackend("default"));
    expect(resolveInvokeBackend()).toBe("mock");
    const snap = (await invokeTracer("tracer_presentation_snapshot")) as { version: number };
    expect(snap.version).toBe(1);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("A3: auto + Tauri routes to invoke with normalized structured args", async () => {
    const invoke = vi.fn(async (_cmd: string, args: unknown) => ({ ok: true, args }));
    g.__TAURI__ = { core: { invoke } };
    setInvokeMode("auto");
    await invokeTracer("tracer_session_create", {
      projectId: "p1",
      title: "t",
      runtime: { scenarioId: "happy_prompt_stream" },
    });
    expect(invoke).toHaveBeenCalledWith(
      "tracer_session_create",
      expect.objectContaining({
        args: expect.objectContaining({ projectId: "p1" }),
      }),
    );
  });
});
