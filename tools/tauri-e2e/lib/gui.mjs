/**
 * L3-J GUI interaction helpers - selector priority (product + harness):
 * 1. role + accessible name (preferred when a11y locators are reliable)
 * 2. form label / htmlFor association (product markup exposes labelled fields)
 * 3. data-testid="tracer-*" (primary automation hook; stable product contract)
 * 4. state marker attributes (data-tracer-ready, data-session-status, ...)
 * 5. CSS last resort (avoid in journeys)
 *
 * Journeys use testids for WebView2 determinism; product controls retain
 * visible labels, roles, and aria-* for accessibility.
 */

import { setTimeout as delay } from "node:timers/promises";
import { byTestId, WebDriverClient } from "./webdriver.mjs";
import { ResultClass } from "./classify.mjs";

/**
 * @param {WebDriverClient} client
 * @param {string} testId
 * @param {{ timeoutMs?: number, intervalMs?: number }} [opts]
 */
export async function waitForTestId(client, testId, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 30_000;
  const intervalMs = opts.intervalMs ?? 300;
  const sel = byTestId(testId);
  const deadline = Date.now() + timeoutMs;
  let lastErr = null;
  while (Date.now() < deadline) {
    try {
      const res = await client.findElement(sel.using, sel.value, { timeoutMs: 5_000 });
      if (res.elementId) return res.elementId;
      lastErr = new Error(`no element for ${testId}: HTTP ${res.statusCode}`);
    } catch (e) {
      lastErr = e;
    }
    await delay(intervalMs);
  }
  throw new Error(
    `waitForTestId(${testId}) timeout: ${lastErr instanceof Error ? lastErr.message : lastErr}`,
  );
}

/**
 * @param {WebDriverClient} client
 * @param {string} testId
 */
export async function clickTestId(client, testId, opts = {}) {
  const id = await waitForTestId(client, testId, opts);
  await client.click(id);
  return id;
}

/**
 * Clear + type into a testid field.
 * @param {WebDriverClient} client
 * @param {string} testId
 * @param {string} text
 */
export async function typeTestId(client, testId, text, opts = {}) {
  const id = await waitForTestId(client, testId, opts);
  try {
    await client.clear(id);
  } catch {
    // clear unsupported — select-all via script fallback
    await client
      .execute(
        `var el = document.querySelector('[data-testid="${testId}"]');
         if (el) { el.focus(); el.value = ''; el.dispatchEvent(new Event('input', { bubbles: true })); }`,
      )
      .catch(() => {});
  }
  // Prefer setting value via DOM for React controlled inputs, then dispatch input/change.
  const setRes = await client.execute(
    `var el = document.querySelector('[data-testid="${testId}"]');
     if (!el) return { ok: false, reason: 'missing' };
     el.focus();
     var proto = el.tagName === 'TEXTAREA'
       ? window.HTMLTextAreaElement.prototype
       : window.HTMLInputElement.prototype;
     var desc = Object.getOwnPropertyDescriptor(proto, 'value');
     if (desc && desc.set) desc.set.call(el, arguments[0]);
     else el.value = arguments[0];
     el.dispatchEvent(new Event('input', { bubbles: true }));
     el.dispatchEvent(new Event('change', { bubbles: true }));
     return { ok: true, value: el.value };`,
    [String(text)],
  );
  const ok = setRes.body?.value?.ok === true;
  if (!ok) {
    await client.sendKeys(id, text);
  }
  return id;
}

/**
 * Select option by value on a <select data-testid>.
 * @param {WebDriverClient} client
 * @param {string} testId
 * @param {string} value
 */
export async function selectTestId(client, testId, value, opts = {}) {
  await waitForTestId(client, testId, opts);
  const res = await client.execute(
    `var el = document.querySelector('[data-testid="${testId}"]');
     if (!el) return { ok: false };
     el.value = arguments[0];
     el.dispatchEvent(new Event('input', { bubbles: true }));
     el.dispatchEvent(new Event('change', { bubbles: true }));
     return { ok: true, value: el.value };`,
    [String(value)],
  );
  if (res.body?.value?.ok !== true) {
    throw new Error(`selectTestId(${testId}) failed for value=${value}`);
  }
}

/**
 * Surface WebDriver protocol errors instead of treating error objects as values.
 * Prevents false "backend switched" when session died mid-suite.
 * @param {{ statusCode?: number, body?: any }} res
 * @param {string} label
 */
export function assertWebDriverOk(res, label = "command") {
  const v = res?.body?.value;
  if (v && typeof v === "object" && typeof v.error === "string") {
    const msg = v.message || v.error;
    const err = new Error(`WebDriver error during ${label}: ${v.error}: ${msg}`);
    err.code =
      /invalid session/i.test(v.error) || /invalid session/i.test(String(msg))
        ? "INVALID_SESSION"
        : "WEBDRIVER_ERROR";
    throw err;
  }
  if (res?.statusCode >= 400) {
    const err = new Error(
      `WebDriver HTTP ${res.statusCode} during ${label}: ${JSON.stringify(res.body).slice(0, 300)}`,
    );
    err.code =
      res.statusCode === 404 ? "INVALID_SESSION" : "WEBDRIVER_HTTP_ERROR";
    throw err;
  }
  return res;
}

/**
 * Read attribute from testid element (via DOM script for reliability).
 */
export async function attrTestId(client, testId, name) {
  const res = await client.execute(
    `var el = document.querySelector('[data-testid="${testId}"]');
     if (!el) return null;
     return el.getAttribute(arguments[0]);`,
    [name],
  );
  assertWebDriverOk(res, `attrTestId(${testId},${name})`);
  return res.body?.value ?? null;
}

export async function textTestId(client, testId) {
  const res = await client.execute(
    `var el = document.querySelector('[data-testid="${testId}"]');
     return el ? (el.innerText || el.textContent || '') : null;`,
  );
  assertWebDriverOk(res, `textTestId(${testId})`);
  return res.body?.value ?? null;
}

export async function existsTestId(client, testId) {
  const res = await client.execute(
    `return !!document.querySelector('[data-testid="${testId}"]');`,
  );
  assertWebDriverOk(res, `existsTestId(${testId})`);
  return res.body?.value === true;
}

/**
 * Wait until predicate script returns truthy.
 * @param {WebDriverClient} client
 * @param {string} script  body returning boolean
 */
export async function waitForScript(client, script, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 30_000;
  const intervalMs = opts.intervalMs ?? 300;
  const deadline = Date.now() + timeoutMs;
  let last = null;
  while (Date.now() < deadline) {
    try {
      const res = await client.execute(script);
      last = res.body?.value;
      if (last) return last;
    } catch (e) {
      last = e instanceof Error ? e.message : String(e);
    }
    await delay(intervalMs);
  }
  throw new Error(`waitForScript timeout; last=${JSON.stringify(last)}`);
}


/**
 * Type into an input associated with a visible label (htmlFor / wrapping label).
 * Falls back to data-testid.
 */
export async function typeByLabel(client, labelText, text, fallbackTestId, opts = {}) {
  const res = await client.execute(
    `var labelText = arguments[0], value = arguments[1];
     var labels = document.querySelectorAll('label');
     for (var i = 0; i < labels.length; i++) {
       var lab = labels[i];
       var t = (lab.innerText || lab.textContent || '').replace(/\s+/g, ' ').trim();
       if (t.indexOf(labelText) !== 0 && t !== labelText && t.indexOf(labelText) === -1) continue;
       var input = lab.querySelector('input,textarea,select');
       if (!input && lab.htmlFor) input = document.getElementById(lab.htmlFor);
       if (!input) continue;
       input.focus();
       var proto = input.tagName === 'TEXTAREA'
         ? window.HTMLTextAreaElement.prototype
         : window.HTMLInputElement.prototype;
       var desc = Object.getOwnPropertyDescriptor(proto, 'value');
       if (desc && desc.set) desc.set.call(input, value);
       else input.value = value;
       input.dispatchEvent(new Event('input', { bubbles: true }));
       input.dispatchEvent(new Event('change', { bubbles: true }));
       return { ok: true, via: 'label' };
     }
     return { ok: false };`,
    [labelText, String(text)],
  );
  if (res.body?.value?.ok) return res.body.value;
  if (fallbackTestId) {
    await typeTestId(client, fallbackTestId, text, opts);
    return { ok: true, via: "testid", testId: fallbackTestId };
  }
  throw new Error(`typeByLabel(${labelText}) failed`);
}

/**
 * Click by role/name or button text; falls back to data-testid.
 */
export async function clickByRoleName(client, role, name, fallbackTestId, opts = {}) {
  const res = await client.execute(
    `var name = arguments[0];
     var btns = document.querySelectorAll('button,[role="button"]');
     for (var i = 0; i < btns.length; i++) {
       var t = (btns[i].innerText || btns[i].getAttribute('aria-label') || '').replace(/\s+/g, ' ').trim();
       if (t === name || t.indexOf(name) === 0) { btns[i].click(); return { ok: true, via: 'role-name' }; }
     }
     return { ok: false };`,
    [name],
  );
  if (res.body?.value?.ok) return res.body.value;
  if (fallbackTestId) {
    await clickTestId(client, fallbackTestId, opts);
    return { ok: true, via: "testid", testId: fallbackTestId };
  }
  throw new Error(`clickByRoleName(${role},${name}) failed`);
}

/**
 * Wait for app ready marker (DOM).
 * @param {WebDriverClient} client
 */
export async function waitAppReady(client, opts = {}) {
  await waitForTestId(client, "tracer-app-ready", {
    timeoutMs: opts.timeoutMs ?? 45_000,
  });
  const backend = await attrTestId(client, "tracer-app-root", "data-tracer-backend");
  return { backend };
}

/**
 * Register project + open workspace via GUI.
 * @param {WebDriverClient} client
 * @param {{ rootPath: string, name?: string }} args
 */
export async function guiRegisterProject(client, args) {
  // Ensure projects home
  if (!(await existsTestId(client, "tracer-projects-home"))) {
    await clickTestId(client, "tracer-nav-projects");
    await waitForTestId(client, "tracer-projects-home");
  }
  // Label-first, testid fallback (priority 2 → 3).
  await typeByLabel(client, "Project root path", args.rootPath, "tracer-project-root-path");
  if (args.name) {
    await typeByLabel(client, "Display name", args.name, "tracer-project-name");
  }
  await clickTestId(client, "tracer-project-register-submit");
  await waitForTestId(client, "tracer-project-workspace", { timeoutMs: 45_000 });
}

/**
 * Create session with optional fake scenario and land on session workspace.
 */
export async function guiCreateSession(client, args = {}) {
  await waitForTestId(client, "tracer-project-workspace", { timeoutMs: 20_000 });
  if (args.title) {
    await typeByLabel(client, "Session title", args.title, "tracer-session-title");
  }
  if (args.scenarioId) {
    await selectTestId(client, "tracer-session-scenario", args.scenarioId);
  }
  await clickTestId(client, "tracer-session-create-submit");
  await waitForTestId(client, "tracer-session-workspace", { timeoutMs: 90_000 });
  const sessionId = await attrTestId(client, "tracer-session-workspace", "data-session-id");
  const status = await attrTestId(client, "tracer-session-workspace", "data-session-status");
  return { sessionId, status };
}

export async function guiSubmitPrompt(client, text) {
  await typeByLabel(client, "Prompt", text, "tracer-prompt-input");
  await clickByRoleName(client, "button", "Send", "tracer-prompt-send");
}

export async function guiRefreshSession(client) {
  if (await existsTestId(client, "tracer-session-refresh")) {
    await clickTestId(client, "tracer-session-refresh");
  }
}

/**
 * Poll session status attribute until match or timeout.
 * @param {WebDriverClient} client
 * @param {(status: string|null) => boolean} pred
 */
export async function waitSessionStatus(client, pred, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 60_000;
  const deadline = Date.now() + timeoutMs;
  let last = null;
  while (Date.now() < deadline) {
    if (await existsTestId(client, "tracer-session-refresh")) {
      await clickTestId(client, "tracer-session-refresh").catch(() => {});
    }
    last = await attrTestId(client, "tracer-session-workspace", "data-session-status");
    if (pred(last)) return last;
    await delay(opts.intervalMs ?? 400);
  }
  throw new Error(`waitSessionStatus timeout; last=${last}`);
}

export async function waitEventType(client, eventType, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 60_000;
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await existsTestId(client, "tracer-session-refresh")) {
      await clickTestId(client, "tracer-session-refresh").catch(() => {});
    }
    const res = await client.execute(
      `var nodes = document.querySelectorAll('[data-event-type]');
       for (var i = 0; i < nodes.length; i++) {
         if (nodes[i].getAttribute('data-event-type') === arguments[0]) return true;
       }
       return false;`,
      [eventType],
    );
    if (res.body?.value === true) return true;
    await delay(opts.intervalMs ?? 400);
  }
  throw new Error(`waitEventType(${eventType}) timeout`);
}

export async function anyEventType(client, types) {
  const res = await client.execute(
    `var want = arguments[0];
     var nodes = document.querySelectorAll('[data-event-type]');
     var found = [];
     for (var i = 0; i < nodes.length; i++) {
       var t = nodes[i].getAttribute('data-event-type');
       if (want.indexOf(t) !== -1) found.push(t);
     }
     return found;`,
    [types],
  );
  return res.body?.value || [];
}

export async function waitAnyEventType(client, types, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 60_000;
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await existsTestId(client, "tracer-session-refresh")) {
      await clickTestId(client, "tracer-session-refresh").catch(() => {});
    }
    const found = await anyEventType(client, types);
    if (found.length) return found;
    await delay(opts.intervalMs ?? 400);
  }
  throw new Error(`waitAnyEventType(${types.join(",")}) timeout`);
}

/**
 * Normalize journey outcome object.
 */
export function journeyResult(id, result, message, detail = {}) {
  return {
    id,
    result,
    message,
    detail,
    claimsProductJourney: true,
  };
}

export function pass(id, message, detail) {
  return journeyResult(id, ResultClass.PASS, message, detail);
}

export function fail(id, message, detail) {
  return journeyResult(id, ResultClass.FAIL, message, detail);
}

export function partial(id, message, detail) {
  return journeyResult(id, ResultClass.PARTIAL, message, detail);
}

export function blockedProductGap(id, message, detail) {
  return journeyResult(id, ResultClass.BLOCKED_BY_PRODUCT_GAP, message, detail);
}

export function blockedFixture(id, message, detail) {
  return journeyResult(id, ResultClass.BLOCKED_BY_FIXTURE, message, detail);
}

export function blockedTooling(id, message, detail) {
  return journeyResult(id, ResultClass.BLOCKED_BY_TOOLING, message, detail);
}
