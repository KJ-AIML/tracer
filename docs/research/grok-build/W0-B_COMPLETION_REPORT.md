# W0-B Completion Report — Grok Build Runtime Reconnaissance

**Task ID:** `tracer-w0-grok-runtime-recon`  
**Work item:** W0-B  
**Heli session:** `heli-ses-a4cae745-a20b-44a0-aa74-9047ab23f8c9`  
**Lease:** `heli-lease-cecb71ab-c367-4de4-994a-32f3e62f07c7`  
**Worktree:** `repos/worktrees/tracer-w0-b`  
**Branch:** `agent/tracer-w0-grok-runtime-recon`  
**Write target:** `tracer`  
**Date:** 2026-07-17

## 1. Outcome

Completed. Stock Grok Build ACP stdio runtime is documented with capability matrix, process lifecycle, event mapping, fork risk analysis, and sanitized fixtures. Live probes confirmed initialize + unauthenticated session/new behavior on Windows with `grok 0.2.102`.

**Recommendation:** use `grok agent --no-leader stdio` as the stock sidecar for the vertical slice; **do not fork** until post-slice adoption gate.

## 2. Files changed

### Research docs

| Path | Purpose |
|---|---|
| `docs/research/grok-build/CAPABILITY_MATRIX.md` | Start command, methods, caps, vendor surface, Tracer mins |
| `docs/research/grok-build/PROCESS_LIFECYCLE.md` | Spawn → init → auth → session → prompt → cancel → shutdown/crash |
| `docs/research/grok-build/ACP_EVENT_MAPPING.md` | Wire → Tracer normalized event mapping |
| `docs/research/grok-build/FORK_RISK_REPORT.md` | Fork vs stock decision + risks |
| `docs/research/grok-build/W0-B_COMPLETION_REPORT.md` | This report |

### Fixtures

| Path | Purpose |
|---|---|
| `tests/fixtures/acp/README.md` | Sanitization policy |
| `tests/fixtures/acp/initialize-request.json` | Canonical initialize request |
| `tests/fixtures/acp/initialize-response.json` | Scrubbed live initialize result |
| `tests/fixtures/acp/session-new-auth-required.json` | Live auth gate error |
| `tests/fixtures/acp/session-prompt-stream.jsonl` | Synthetic stream sequence |
| `tests/fixtures/acp/permission-request.json` | Synthetic permission reverse-request |
| `tests/fixtures/acp/cancel-notification.json` | Synthetic cancel |

## 3. Commands run

```text
# Bootstrap
npx --yes github:KJ-AIML/heli-harness task claim tracer-w0-grok-runtime-recon --mode write --host grok-build
npx --yes github:KJ-AIML/heli-harness target set tracer
npx --yes github:KJ-AIML/heli-harness session status
npx --yes github:KJ-AIML/heli-harness task show tracer-w0-grok-runtime-recon
npx --yes github:KJ-AIML/heli-harness conflicts --task tracer-w0-grok-runtime-recon

# Source recon (read-only on repos/grok-build)
# - docs user-guide 15-agent-mode.md
# - pager-bin main agent dispatch
# - shell run_stdio_agent + mvp_agent acp_agent
# - xai-acp-lib stdin_reader
# - test-support acp_client
# - sandbox / tty-utils Windows notes

# Live wire probe (hermetic GROK_HOME)
grok agent --no-leader stdio
  → initialize (success)
  → session/new without authenticate → Authentication required

# Validation / commit
git -C <WORKSPACE_ROOT>/repos/grok-build status --porcelain   # empty
git status / git diff / git add owned paths only
git commit -m "docs(w0-b): grok-build runtime recon, ACP mapping, fixtures"
```

## 4. Validation

| Check | Result |
|---|---|
| Heli write lease on correct task | Yes |
| Target `tracer` + worktree `tracer-w0-b` | Yes |
| Path claims / conflicts | No overlaps |
| Writes only under `docs/research/grok-build/` and `tests/fixtures/acp/` | Yes |
| `repos/grok-build` clean (no edits) | Yes |
| Fixtures scrubbed (no tokens/private prompts/absolute machine paths) | Yes |
| Live initialize documented | Yes |
| Authenticated full prompt stream | **Not live** — blocked without credentials; synthetic fixture + source mapping provided |

## 5. Assumptions

1. Released `grok` on PATH (`0.2.102`) is representative of stock product ACP behavior for this recon; source tree pin is `SOURCE_REV=2ec0f0c…`.
2. Tracer MVP uses **stdio**, not `serve`/`headless`/`leader`.
3. W0-A contracts may refine Tracer event names; mapping tables use stable conceptual names.
4. Auth method ids are discovered from `initialize`, not hard-coded beyond examples.
5. Parent amendment overrides master-plan paths: deliverables live in Tracer worktree, not parent `resources/`.

## 6. Risks / residual gaps

| Item | Severity | Notes |
|---|---|---|
| No live authenticated prompt capture | Medium | Requires credentials/mock server; synthetic stream provided |
| Vendor `x.ai/*` surface is large & unstable | Medium | MVP should stick to standard ACP |
| Windows OS sandbox absent | Medium | Permission UI still required |
| Leader mode complexity | Low for MVP | Avoid via `--no-leader` |
| Binary vs source version skew | Low | Document both pins |

## 7. Commit SHA(s)

| Commit | Message |
|---|---|
| `ff2b2dd56d583511ccd3b0169e77d9fd99027f4a` | `docs(w0-b): grok-build runtime recon, ACP mapping, fixtures` |
| _(this report commit, if separate)_ | `docs(w0-b): completion report` |

Local commits only — **not pushed**.

## 8. Integration order

1. **After W0-A** architecture/contracts land (event protocol + runtime adapter contract).  
2. **This W0-B evidence** informs adapter capability negotiation and fake ACP design.  
3. **Before authorizing W0-C / W0-D** consumers that depend on runtime findings (UX/test strategy may read these docs).  
4. **W1 ACP client** implements adapter using fixtures + mapping.  
5. **Fork decision** deferred to post vertical-slice adoption gate (`FORK_RISK_REPORT.md`).

## 9. Key factual takeaways

- **Start command:** `grok agent stdio` (prefer `grok agent --no-leader stdio`).
- **Transport:** JSON-RPC 2.0 NDJSON on stdio; stderr for logs.
- **Readiness:** successful `initialize` (no ready banner).
- **Auth required** before `session/new` in real runs.
- **Streaming:** `session/update` with standard update types; vendor extras via `x.ai/session_notification`.
- **Permissions:** blocking `session/request_permission` reverse-requests.
- **Cancel:** `session/cancel` notification with optional subagent/rewind meta.
- **Shutdown:** close stdin / kill child; agent closes PTYs on stdio exit path.
- **Fork:** not recommended for Wave 0.

## 10. Lease release

Performed at end of worker run:

```text
npx --yes github:KJ-AIML/heli-harness task release tracer-w0-grok-runtime-recon --session heli-ses-a4cae745-a20b-44a0-aa74-9047ab23f8c9
npx --yes github:KJ-AIML/heli-harness session close --session heli-ses-a4cae745-a20b-44a0-aa74-9047ab23f8c9
```
