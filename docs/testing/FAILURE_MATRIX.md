# Failure Matrix — Vertical Slice

**Status:** Gate 0 reliability freeze candidate  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-test-strategy` (W0-D)  
**Normative contracts:** W0-A event/adapter/command docs  
**Wire evidence:** W0-B recon + Stage 0.1 integration notes

## 1. Purpose

For each failure class that can occur in the first vertical slice, define:

1. **Detection** — how Tracer learns something went wrong  
2. **User-visible outcome** — status, events, command errors  
3. **System actions** — cancel, kill, persist, refuse prompts  
4. **Recovery** — what the user/control plane may do next  
5. **Test coverage** — tier + scenario ids  
6. **Evidence label** — synthetic / fake / live  

If a failure has no row, Wave 1 must add one before treating the path as “done.”

## 2. Legend

| Column | Values |
|---|---|
| **Severity** | S0 blocker · S1 high · S2 medium · S3 low |
| **Session outcome** | W0-A status: `failed` / `disconnected` / `stopped` / `cancelling` → … |
| **CI** | `required` on standard CI · `platform` · `optional-live` |

## 3. Matrix

### 3.1 Process lifecycle failures

| ID | Failure | Severity | Detection | Events (W0-A) | Command / errorClass | Session outcome | System actions | Recovery | Tests | CI | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F-P01 | Runtime executable missing | S1 | Spawn fails | `runtime.process.failed` | `RuntimeExecutableNotFound` | `failed` | Do not mark ready | Fix config/PATH; recreate session | unit + T2 | required | fake |
| F-P02 | OS spawn failure | S1 | Spawn errno | `runtime.process.failed` | `RuntimeSpawnFailed` | `failed` | No process handle | Retry after env fix | T2 | required | fake |
| F-P03 | Initialize timeout / hang | S1 | Deadline `T_init` | `runtime.process.failed` or exit after kill | `ProtocolInitializeFailed` / `Timeout` | `failed` | Kill child | Recreate | T2 | required | fake |
| F-P04 | Initialize protocol error | S1 | Bad/missing init result | `adapter.protocol.error` + failed ready path | `ProtocolInitializeFailed` | `failed` | Shutdown process | Fix runtime version | T1/T2 | required | synthetic/fake |
| F-P05 | Unexpected exit while idle | S1 | Wait/exit | `runtime.process.exited` (`expected: false`) | later ops: `RuntimeDisconnected` | `disconnected` | Close pipes; free handle | Start new process | T2 | required | fake |
| F-P06 | Unexpected exit mid-prompt (**crash**) | S0 | Wait/exit | `runtime.process.exited` (`expected: false`) and/or `runtime.process.failed`; fail tools | `RuntimeCrashed` / `RuntimeDisconnected` | `failed` or `disconnected` | Fail active run; **no** success complete | New session/process | VS-06 `crash_nonzero_exit` | required | fake |
| F-P07 | EOF / broken pipe mid-prompt | S0 | Read EOF | same honesty as F-P06 | `RuntimeDisconnected` | `failed`/`disconnected` | End run | New process | VS-07 `eof_mid_prompt` | required | fake |
| F-P08 | Stderr flood then death | S2 | stderr chunks + exit | `runtime.process.stderr`* then exit/fail | as F-P06 | `failed`/`disconnected` | Truncate stderr with `truncated` | Inspect logs; restart | T2 | required | fake |
| F-P09 | Clean stop (user) | S3 (success path) | shutdown | `session.cancelled` optional; `runtime.process.exited` (`expected: true`) | stop ok | `stopped`/`completed` | Graceful then kill budget | N/A | VS-09 | required | fake |
| F-P10 | Force kill after cancel/stop timeout | S1 | `T_term` exceeded | exited expected if user-initiated stop | `CancellationFailed` then stop success | `stopped`/`cancelled` | **Kill tree / Job Object** | Ensure no orphans | `slow_cancel_ack` + T5 | required/platform | fake |
| F-P11 | Orphan child / PTY leak | S0 | Post-stop process scan | diagnostic logs | stop may return error if leak detected | `stopped` with warning if leak | Job Object kill-on-close (Windows); process group (Unix) | Manual cleanup; fix manager | T5 Windows + Unix | platform | fake |
| F-P12 | App crash while runtime runs | S1 | OS kills UI | (no further events) | N/A | durable status may be stale `running` | Prefer job kill-on-close so children die | On restart: reconcile status ≠ running without process | T5 + recovery | platform | fake |
| F-P13 | Exit before ready | S1 | exit during init | `runtime.process.failed` / exited | `ProtocolInitializeFailed` / `RuntimeCrashed` | `failed` | No ready | Recreate | T2 | required | fake |

### 3.2 Auth and session-create failures

| ID | Failure | Severity | Detection | Events (W0-A) | Command / errorClass | Session outcome | System actions | Recovery | Tests | CI | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F-A01 | `session/new` without authenticate (stock shape) | S0 | JSON-RPC error `-32000` Authentication required | no `session.ready`; optional protocol error mapping | prefer `AuthenticationRequired` (additive); until then non-success `InvalidState` / `PromptRejected` / mapped protocol error — **not** silent ready | not ready; may stay `starting_runtime`/`failed` per design | Do not accept prompts | Authenticate then create session | VS-02 + fixture `session-new-auth-required.json` | required | **live-scrubbed** fixture + fake |
| F-A02 | Authenticate failed (bad key / denied) | S1 | auth error response | failed session start | prefer `AuthenticationFailed` (additive) | `failed` | Keep process or recycle per policy | Re-auth | T2 fake scripted; T6 live optional | required (fake) / optional-live | fake / live-auth |
| F-A03 | Prompt before process ready | S1 | precondition | none or rejected | `RuntimeNotReady` / `InvalidState` | unchanged | Reject command | Wait for ready | T0/T2 | required | fake |
| F-A04 | Prompt before session ready | S1 | precondition | none | `InvalidState` | unchanged | Reject | Complete auth/session | T2 | required | fake |
| F-A05 | Confusing process-up with prompt-ready | S0 (product bug if occurs) | status audit | — | — | — | Tests must fail if UI shows ready prompts without `session.ready` | Fix control plane | VS-02, VS-13 | required | fake |

### 3.3 Protocol and framing failures

| ID | Failure | Severity | Detection | Events (W0-A) | errorClass | Session outcome | System actions | Recovery | Tests | CI | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F-R01 | Malformed JSON line | S1 | parse error | `adapter.protocol.error` | `ProtocolParseError` | continue if process alive; else disconnect | Continue or resync; never crash UI | If wedged → stop process | VS-08 `malformed_frame` | required | synthetic/fake |
| F-R02 | Valid JSON, invalid schema | S2 | validation | `adapter.protocol.error` | `ProtocolViolation` / parse | continue if possible | Ignore bad frame for side effects | Continue | T1/T2 | required | synthetic |
| F-R03 | Duplicate response id | S2 | correlation map | `adapter.protocol.error` | `ProtocolViolation` | continue | Ignore duplicate effects | Continue | `duplicate_response_id` | required | fake |
| F-R04 | Unknown standard-looking notification | S3 | unmapped | `adapter.protocol.unknown` | — | continue | Store metadata only | None | VS-08 | required | synthetic |
| F-R05 | Unknown **vendor** `x.ai/*` notification | S3 | unmapped vendor | `adapter.protocol.unknown` | — | continue | Do not require UI parse | Optional later map | VS-08 | required | synthetic |
| F-R06 | Oversized message | S2 | size guard | protocol error | `ProtocolParseError` / limits | continue or fail transport | Truncate/reject | Continue | T0/T2 | required | fake |
| F-R07 | Capability mismatch (hard requirement) | S1 | negotiation | `runtime.process.failed` | `CapabilityMismatch` | `failed` | Do not ready | Different runtime | T2 | required | fake |
| F-R08 | Capability unsupported op (soft) | S2 | API call | may emit status | `CapabilityUnsupported` | often still ready | Use documented fallback (e.g. process stop for cancel) | Fallback path | VS-11 | required | fake |

### 3.4 Cancellation and approval failures

| ID | Failure | Severity | Detection | Events (W0-A) | errorClass | Session outcome | System actions | Recovery | Tests | CI | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F-C01 | Cancel mid-stream (happy cancel) | S3 path | user cancel | `cancelling` → `session.cancelled` | cancel `accepted` | `stopped` or `ready` after cancel per design | Cooperative cancel | New prompt or stop | VS-04 | required | fake |
| F-C02 | Cancel not acknowledged in time | S1 | `T_cancel` | then process stop events | `CancellationFailed` possible | `stopped`/`cancelled` | Force kill | New process | `slow_cancel_ack` | required | fake |
| F-C03 | Cancel when capability missing | S1 | caps false | process stop path | `CapabilityUnsupported` on cooperative API | terminal stop | Process stop fallback | New process if killed | VS-11 | required | fake |
| F-C04 | **Cancel while permission pending** | S0 | user cancel during park | leave `awaiting_approval`; `approval.resolved` cancel **or** process death | cancel/stop accepted | terminal within bound | Cancel reverse-request **or** kill; **must not deadlock** | Recreate if process killed | VS-05 | required | fake |
| F-C05 | Permission request ignored (bug) | S0 if occurs | hung turn | stuck `awaiting_approval` | Timeout | must not stay forever | Watchdog → fail/stop | Stop session | VS-05 timeout branch | required | fake |
| F-C06 | Approval deny | S2 path | user/policy | `approval.resolved` deny; `tool.failed` or cancelled | — | return `ready` or fail tool only | Fail closed | Continue session | VS-03 deny | required | fake |
| F-C07 | Approval allow | S3 path | user | `approval.resolved` allow; tools complete | — | running → ready/completed | Forward decision | Continue | VS-03 allow | required | fake |
| F-C08 | Unknown approval id resolve | S2 | resolve call | — | `ApprovalUnknown` / `NotFound` | unchanged | No runtime side effect | Refresh pending list | T0/T2 | required | fake |
| F-C09 | Auto-approve unknown risk (forbidden) | S0 if occurs | audit | — | policy violation | — | **Must not happen** | Fix policy | contract + VS-03 | required | fake |
| F-C10 | Double resolve approval | S2 | state | — | `InvalidState` | unchanged | Ignore second | None | T0/T2 | required | fake |

### 3.5 Prompt / tool / agent failures

| ID | Failure | Severity | Detection | Events (W0-A) | errorClass | Session outcome | System actions | Recovery | Tests | CI | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F-T01 | Runtime rejects prompt | S2 | error response | `session.failed` or status + error | `PromptRejected` | `failed` or stay ready with error | Surface error | Edit prompt / re-auth | T2 | required | fake |
| F-T02 | Tool fails | S2 | tool update | `tool.failed` | — | often still ready | Show failure in timeline | Continue | happy+deny paths | required | fake |
| F-T03 | Partial stream then stop reason cancelled | S2 | prompt result | deltas + `session.cancelled` | — | cancelled | Persist partial | New prompt | VS-04 | required | fake |
| F-T04 | Refusal / safety stop | S2 | stop reason | message + completed/failed per map | may be `PromptRejected` | ready/failed | Show refusal | User revises | optional fake | required if implemented | synthetic |
| F-T05 | Missing streaming capability | S3 | caps | single `agent.message.completed` | — | normal | Synthesize completed | None | VS-12 | required | fake |

### 3.6 Storage and control-plane failures

| ID | Failure | Severity | Detection | Events (W0-A) | errorClass | Session outcome | System actions | Recovery | Tests | CI | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F-S01 | DB write failure mid-run | S1 | storage layer | `storage.error` | `StorageError` | may continue run but **must not** claim durable success falsely | Surface error; retry policy | Fix disk; restart | T3 | required | unit |
| F-S02 | Migration failure | S1 | startup | app fail open | `StorageError` | N/A | Refuse start if unsafe | Fix migrations | T3 | required | unit |
| F-S03 | Interrupted write | S1 | crash mid-tx | durable consistent or rolled back | — | — | SQLite transaction discipline | Restart + verify | T3 | required | unit |
| F-S04 | Stale “running” after restart | S1 | reconcile | status correction event optional | — | `disconnected`/`failed`/`stopped` | On boot: if no process, rewrite status | User resumes deliberately | VS-10 | required | fake+storage |
| F-S05 | Runtime writes DB (forbidden) | S0 if occurs | architecture test | — | — | — | **Must not** | Code review / seam test | review | required | design |

### 3.7 UI / command surface failures

| ID | Failure | Severity | Detection | Events | errorClass | Outcome | Actions | Recovery | Tests | CI | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F-U01 | Invalid command args | S3 | validate | — | `InvalidArgument` | no state change | Reject | Fix client | T0 | required | unit |
| F-U02 | Unknown session id | S3 | lookup | — | `NotFound` / `SessionNotFound` | — | Reject | Refresh list | T0 | required | unit |
| F-U03 | UI parses raw ACP (bug) | S0 if occurs | review/test | — | — | wrong coupling | Forbid imports | Refactor | VS-13 | required | unit |
| F-U04 | UI shows running after exit | S0 if occurs | state test | exit already emitted | — | lying UX | Bind UI to status+events | Fix store | VS-06 + T4 | required | fake |
| F-U05 | Project path missing | S2 | register/get | — | `NotFound`/`InvalidArgument` | project `missing` | Show missing state | Re-register | VS-14 | required | unit |

### 3.8 Platform-specific failures

| ID | Failure | Platform | Severity | Expected product behavior | Tests | CI |
|---|---|---|---|---|---|---|
| F-W01 | Orphan after TerminateProcess without job | Windows | S0 | Process manager **must** use Job Object (or documented equivalent) for session runtimes | T5 | platform |
| F-W02 | Stdin hang (historical stock issue) | Windows | S2 | Timeouts on init/prompt; prefer current stock binary; stderr separate | T6 optional + docs | optional-live |
| F-W03 | Named pipe leader surprises | Windows | S2 | MVP uses `--no-leader`; tests do not require leader | docs | N/A |
| F-W04 | No OS sandbox enforce | Windows | S2 | Permission UI still mandatory; never imply OS sandbox | docs + VS-03 | required (policy) |
| F-U01x | setsid/process group leak | macOS/Linux | S1 | Kill process group on force stop | T5 | platform |
| F-U02x | Sandbox deny tool | macOS/Linux | S2 | Surface tool failure; not Tracer crash | optional live | optional-live |

## 4. Timeout budget summary

| Budget | Suggested default | Used by |
|---|---|---|
| `T_init` | 20s (aligned with recon harness scale) | initialize readiness |
| `T_cancel` | 5–15s | cooperative cancel |
| `T_term` | 2–5s after cancel budget | force kill |
| `T_permission_watchdog` | ≤ `T_cancel + T_term` when cancelling | VS-05 / F-C04 |
| `T_prompt` (optional) | product policy | runaway streams |

Tests must use injectable fake delays rather than real multi-minute sleeps.

## 5. Recovery playbook (product)

| After… | User may… | System guarantees |
|---|---|---|
| `failed` start | Fix runtime path/auth; create new session | Old handle unusable |
| `disconnected` crash | Resume history view; start new runtime | Events preserved; no silent running |
| `stopped` cancel | Submit new prompt if session reusable; else new session | No orphans |
| Auth required | Complete auth method | Process may stay up (Gate P/I hold) |
| Storage error | Free disk; restart app | No false “saved” claims |

## 6. Mapping to Gate 1

Minimum **required** failure IDs for Gate 1 evidence:

```text
F-P01, F-P06, F-P07, F-P10, F-P11 (on at least one OS),
F-A01, F-A03, F-A05,
F-R01, F-R04, F-R05, F-R08,
F-C01, F-C02, F-C03, F-C04, F-C06, F-C07, F-C09,
F-S01, F-S04,
F-U03, F-U04
```

Optional live: F-A02 (live), F-W02.

## 7. Explicit non-claims

| Non-claim | Reason |
|---|---|
| Synthetic `session-prompt-stream.jsonl` proves live tool/permission parity | Synthetic only |
| Standard CI proves stock Grok auth login | No credentials in CI |
| Windows sandbox equals macOS/Linux enforce | W0-B: not applied the same |
| Vendor extension stability | Out of MVP contract |

---

**Document control:** W0-D deliverable. Update when Wave 1 discovers new failure classes; keep IDs stable.
