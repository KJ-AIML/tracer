# Fork Risk Report — Grok Build as Tracer Runtime

**Task:** W0-B  
**Question:** Should Tracer adopt stock Grok Build as a sidecar via ACP, or fork/rebrand a downstream runtime early?  
**Recommendation (Wave 0):** **Do not fork yet.** Integrate stock ACP stdio behind a runtime adapter. Revisit after the vertical slice and adoption gate.

## 1. Executive summary

Grok Build already implements a full ACP agent server (`grok agent stdio`) with session lifecycle, streaming, permissions, cancellation, persistence, MCP, and a large vendor extension surface. Tracer’s first vertical slice only needs the **standard ACP core** plus thin auth/session glue.

Forking now would:

- Create immediate dual-maintenance against a large Rust monorepo  
- Delay the vertical slice  
- Risk baking Tracer into xAI-specific extensions instead of portable ACP  

**Adopt-as-process first. Fork only with evidence.**

## 2. What “stock runtime” means

| Asset | Location | Mutability for W0-B |
|---|---|---|
| Upstream research checkout | `repos/grok-build` | **Read-only** |
| Released CLI | `grok` on PATH (user install) | External dependency |
| Downstream product runtime (future) | `repos/tracer-agent-runtime` (planned) | Not started |

Stock usage model:

```text
Tracer Desktop → Control Plane → Runtime Adapter → child process: `grok agent --no-leader stdio`
```

## 3. Value of not forking (near term)

| Benefit | Detail |
|---|---|
| Speed | Vertical slice can start without recompiling agent runtime |
| Protocol leverage | Official ACP + SDKs exist; tests already use `agent-client-protocol` |
| Continuous upstream fixes | Auth, models, tools, sandbox, MCP evolve without Tracer merges |
| Clear product boundary | Tracer owns UX/control plane; Grok owns model/tool execution |
| Parallelism | W0/W1 agents work on Tracer contracts while runtime stays black-box |

## 4. Risk register

### 4.1 Protocol / API drift

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| `x.ai/*` extensions change without notice | High for vendor features; Low for MVP if unused | High | Depend only on standard ACP methods for MVP; feature-detect `_meta` |
| ACP crate version skew (`0.10.x`) | Medium | Medium | Pin adapter to observed wire shapes; integration fixtures |
| Capability flags rename | Medium | Medium | Capability matrix snapshot + negotiation tests |
| Underscored methods (`_x.ai/...`) | Low | Observed | Treat as vendor; tolerant parser |

### 4.2 Auth and licensing

| Risk | Severity | Mitigation |
|---|---|---|
| Requires xAI/Grok authentication | High product dependency | Support api key + login flows; abstract auth in adapter |
| Enterprise OIDC / team policies | Medium | Document; don’t hardcode method ids beyond discovery |
| ToS / redistribution of modified CLI | High if forked/rebranded | Prefer stock binary distribution; legal review before fork |
| Telemetry in stock binary | Medium | Document; later runtime may need policy knobs |

### 4.3 Process and platform

| Risk | Severity | Mitigation |
|---|---|---|
| Windows stdin/stdio edge cases | Medium (historical hangs fixed upstream) | Use current `grok`; keep stderr separate; timeouts |
| Leader mode surprises | Medium | Tracer spawns with `--no-leader` |
| Child process leaks (PTY/shell) | Medium | Graceful cancel + job object / kill tree |
| OS sandbox weak/absent on Windows | Medium | Product warnings; permission UI still required |
| Source builds not CI-tested on Windows | Medium | Prefer released binaries for Windows dev |

### 4.4 Product coupling

| Risk | Severity | Mitigation |
|---|---|---|
| UI assumes Grok slash commands / goals / subagents | High if over-integrated | Normalize to Tracer event protocol; optional panels |
| Session storage lives under Grok home | Medium | Adapter maps session ids; Tracer DB is source of UX truth |
| Model catalog empty until auth | Low | Expected; gate prompts on auth+models |
| Always-approve misuse | High security | Default ask; yolo only explicit user setting |

### 4.5 Maintenance if forked early

| Cost | Detail |
|---|---|
| Monorepo size | Dozens of crates (pager, shell, tools, sampler, workspace, …) |
| Sync burden | Upstream `SOURCE_REV` moves; cherry-picks painful |
| Security patches | Must track independently |
| Branding | Auth endpoints, telemetry, paths (`~/.grok`) intertwined |
| Build complexity | protoc/DotSlash, platform sandbox features |

## 5. When a fork / downstream runtime becomes justified

Open a **runtime adoption gate** only if multiple conditions hold:

1. Stock ACP cannot express a Tracer-critical capability **and** vendor extensions are insufficient or unstable.  
2. Auth/branding/path requirements legally or product-wise require a Tracer-owned binary.  
3. Tracer needs offline/mock/fake runtimes + stock runtimes with identical adapter contracts (fake can exist **without** forking Grok).  
4. Upstream cadence blocks security fixes Tracer must ship.  
5. Multi-agent orchestration (ALMS/Heli) needs runtime hooks that cannot live in the control plane.

Until then, implement:

- `repos/tracer` adapter + fake ACP server for tests  
- Optional later `repos/tracer-agent-runtime` as **downstream** with clear cherry-pick policy  

## 6. Minimal change surface if fork is forced later

If forced, prefer **thin downstream**, not a full rebrand on day one:

| Layer | Fork? | Notes |
|---|---|---|
| `xai-grok-shell` ACP agent | Possibly | Core protocol server |
| Tools / sampler / models | Avoid initially | High churn |
| Pager TUI | No | Tracer has its own UI |
| Paths / auth branding | Yes if productized | `GROK_HOME` → Tracer paths |
| Telemetry defaults | Yes | Privacy policy |
| Vendor extensions | Trim | Keep ACP standard + Tracer-owned extensions |

**Do not** copy the entire pager UI stack into Tracer.

## 7. Adapter isolation strategy (anti-fork insurance)

```text
┌──────────────────────────────────────────┐
│ Tracer Control Plane                     │
│  - process supervisor                    │
│  - permission policy                     │
│  - persistence of Tracer sessions        │
└─────────────────┬────────────────────────┘
                  │ RuntimeAdapter trait
      ┌───────────┴────────────┐
      ▼                        ▼
 GrokAcpAdapter            FakeAcpAdapter
 (stock grok stdio)        (tests)
      │
      ▼ optional later
 TracerRuntimeAdapter (downstream binary)
```

Hard rules:

- UI never imports ACP types  
- Capability negotiation at initialize  
- Vendor features behind `if caps.vendor.x_ai`  
- Contract tests use sanitized fixtures from W0-B  

## 8. Windows vs Unix risk differential

| Area | Windows | macOS/Linux |
|---|---|---|
| Released binary ACP stdio | Works (probed) | Works |
| Building from `repos/grok-build` | Best-effort | Supported |
| Leader IPC | Named pipes | Unix sockets |
| Stdin implementation | Special-case dup to NUL | Direct stdin |
| Kernel sandbox | Not enforced like Unix | Landlock / Seatbelt |
| Process tree kill | Job Objects | process groups / setsid |
| Shell/PTY | ConPTY / shell cascade | PTY |

Tracer’s process supervisor must be **platform-aware** even if the ACP schema is not.

## 9. Security posture using stock binary

- Treat agent as **semi-trusted local peer**, not a pure library.  
- Permission prompts are mandatory by default.  
- Isolate workspace cwd; do not start agent with secrets on argv.  
- Never log `authenticate` secrets, API keys, or raw private prompts.  
- Consider env scrubbing when spawning (pass only required vars).  
- Future: sandbox policy UI even when OS sandbox inactive (Windows).

## 10. Decision record (Wave 0)

| Option | Decision |
|---|---|
| A. Stock `grok agent stdio` via adapter | **Accepted for vertical slice** |
| B. Immediate full fork of grok-build into tracer-agent-runtime | **Rejected** |
| C. Fake ACP only (no stock) | Rejected as sole path; keep as **test double** |
| D. WebSocket `agent serve` as primary | Rejected for MVP (stdio simpler, local-first) |

### Revisit triggers

- Post vertical slice demo  
- Auth/branding blockers from product  
- ACP standard gaps proven in adapter work (W1-D)  
- Legal guidance on redistribution  

## 11. Integration order note

Per Wave 0 amendment:

1. Complete **W0-A** contracts (event protocol, adapter contract).  
2. Consume this W0-B evidence.  
3. Only then authorize **W0-C/D** consumers and **W1** ACP client implementation.  
4. Runtime fork decision is **post-slice**, not Wave 0.

## 12. Bottom line

Stock Grok Build is a **viable ACP runtime sidecar** today. The largest risks are product coupling and vendor drift—not missing core protocol support. Tracer should invest in a **strict adapter boundary** and fixtures, not a premature fork.
