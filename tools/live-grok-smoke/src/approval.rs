//! W2-D live approval reverse-request validation (LVA-01…LVA-07).
//!
//! Opt-in only. Never auto-approves without an explicit scenario action.
//! Never fabricates PASS when `approval.requested` was not observed.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tracer_domain::{EventType, ProjectId, SessionId};
use tracer_runtime_adapter::{
    grok_stdio_spawn_config, AdapterEvent, ApprovalDecisionRequest, PromptRequest, RuntimeAdapter,
    SessionCreateParams, ShutdownOptions,
};

use crate::discovery::{self, DiscoveryResult};
use crate::evidence::{
    stage_evidence, EvidenceReport, LiveClassification, ScenarioResult, StageStatus, SuiteKind,
};
use crate::sanitize;
use crate::stages::{RunConfig, StageId};

/// Public-safe default prompt intended to induce a tool permission reverse-request.
/// Operators may override via `--prompt`. Never private content.
pub const DEFAULT_APPROVAL_INDUCING_PROMPT: &str = "\
Create a new text file named tracer-lva-probe.txt in the current working directory \
containing the single word probe, then stop. Use a file tool if available.";

/// Budget for reverse-request observation per attempt (live).
const APPROVAL_WAIT: Duration = Duration::from_secs(45);
/// After cancel/resolve is initiated, control must return within this window
/// (no-deadlock). Measured from action initiation when possible; overall
/// attempt budget is APPROVAL_WAIT + this slack.
const NO_DEADLOCK_BUDGET: Duration = Duration::from_secs(30);
/// Gap between LVA attempts so the agent can settle.
const ATTEMPT_SETTLE: Duration = Duration::from_millis(400);

/// LVA scenario ids in contract order.
pub const LVA_SCENARIOS: &[&str] = &[
    "LVA-01", "LVA-02", "LVA-03", "LVA-04", "LVA-05", "LVA-06", "LVA-07",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttemptAction {
    /// Wait for reverse-request only (observe), then cancel to free the turn.
    Observe,
    /// Resolve allow-once when reverse-request appears.
    AcceptOnce,
    /// Resolve reject-once when reverse-request appears.
    RejectOnce,
    /// Cancel prompt while reverse-request is pending.
    CancelWhilePending,
}

/// Observed outcome of one inducing attempt.
#[derive(Debug, Clone)]
struct AttemptOutcome {
    action: AttemptAction,
    reverse_request_observed: bool,
    resolved_ok: bool,
    cancel_ok: bool,
    prompt_returned: bool,
    terminal_observed: bool,
    /// Agent finished the turn without ever emitting a reverse-request
    /// (natural completion — not a timeout cancel).
    prompt_completed_without_approval: bool,
    /// Waiter hit APPROVAL_WAIT without seeing `approval.requested`.
    timed_out_without_rr: bool,
    within_deadlock_budget: bool,
    notes: Vec<String>,
}

fn scenario_enabled(cfg: &RunConfig, id: &str) -> bool {
    match &cfg.scenarios {
        None => true,
        Some(list) => list.iter().any(|s| s.eq_ignore_ascii_case(id)),
    }
}

fn resolve_exe(disc: &DiscoveryResult) -> String {
    if let Some(p) = &disc.spawn_path {
        return p.display().to_string();
    }
    "grok".into()
}

fn drain_event_types(adapter: &RuntimeAdapter, out: &mut Vec<String>, ms: u64) {
    let deadline = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < deadline {
        let batch = adapter.drain_events();
        if batch.is_empty() {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
        for ev in batch {
            if let AdapterEvent::Event(env) = ev {
                let t = env.event_type.as_str().to_string();
                if !out.contains(&t) {
                    out.push(t);
                }
            }
        }
    }
    for ev in adapter.drain_events() {
        if let AdapterEvent::Event(env) = ev {
            let t = env.event_type.as_str().to_string();
            if !out.contains(&t) {
                out.push(t);
            }
        }
    }
}

fn push_all_scenarios(report: &mut EvidenceReport, cfg: &RunConfig, status: LiveClassification, detail: &str) {
    for id in LVA_SCENARIOS {
        if scenario_enabled(cfg, id) {
            report.scenarios.push(ScenarioResult {
                id: (*id).into(),
                status,
                detail: detail.into(),
            });
        }
    }
}

/// Dry-run for LVA suite: construction + scenario plan, no agent stdio.
pub fn run_approval_dry_run(cfg: &RunConfig) -> Result<EvidenceReport, String> {
    let platform = discovery::platform_label();
    let mut report = EvidenceReport::new_suite(true, false, platform, SuiteKind::Lva);

    let t0 = Instant::now();
    let disc = discovery::discover_grok(cfg.grok_override.as_deref());
    let exe = disc.executable.clone().unwrap_or_else(|| "grok".into());
    report.discovery = Some(disc.clone());
    report.push_stage(stage_evidence(
        StageId::Discovery,
        StageStatus::Pass,
        t0.elapsed().as_millis() as u64,
        {
            let mut n = disc.notes.clone();
            n.push("approval dry-run: version probe only; agent stdio not launched".into());
            if !disc.found {
                n.push("binary not found on this host — spawn plan still validated".into());
            }
            n
        },
        None,
        Some(json!({
            "found": disc.found,
            "version": disc.version,
            "source": disc.source,
        })),
    ));

    let t1 = Instant::now();
    let spec = grok_stdio_spawn_config(&exe, &cfg.cwd);
    let args = tracer_runtime_adapter::grok_stdio_args();
    let expected = vec![
        "agent".to_string(),
        "--no-leader".to_string(),
        "stdio".to_string(),
    ];
    let args_ok = args == expected && spec.args == expected;
    report.spawn_plan = json!({
        "displayName": spec.display_name,
        "kind": spec.kind.as_str(),
        "executable": sanitize::display_path(std::path::Path::new(&exe), Some(&cfg.cwd)),
        "args": spec.args,
        "cwd": sanitize::display_path(&cfg.cwd, None),
        "isolateProcessTree": spec.isolate_process_tree,
        "killOnDrop": spec.kill_on_drop,
        "matchesW0bW1d": args_ok,
        "productHelper": "tracer_runtime_adapter::grok_stdio_spawn_config",
        "suite": "lva",
    });
    report.push_stage(stage_evidence(
        StageId::Startup,
        if args_ok {
            StageStatus::Pass
        } else {
            StageStatus::Fail
        },
        t1.elapsed().as_millis() as u64,
        vec![
            "approval dry-run: spawn config via grok_stdio_spawn_config".into(),
            "process not launched".into(),
        ],
        None,
        Some(report.spawn_plan.clone()),
    ));

    for stage in [
        StageId::Initialize,
        StageId::AuthRequirement,
        StageId::Session,
        StageId::Prompt,
        StageId::Stream,
        StageId::Approval,
        StageId::Cancel,
        StageId::Shutdown,
    ] {
        report.push_stage(stage_evidence(
            stage,
            StageStatus::NotRun,
            0,
            vec![
                "approval dry-run: stage plan validated; not launched".into(),
                format!(
                    "live requires: TRACER_LIVE_GROK=1 + `approval-run` + stage {}",
                    stage.as_str()
                ),
            ],
            None,
            None,
        ));
    }

    for id in LVA_SCENARIOS {
        if !scenario_enabled(cfg, id) {
            continue;
        }
        report.scenarios.push(ScenarioResult {
            id: (*id).into(),
            status: LiveClassification::NotRun,
            detail: "approval dry-run only; live LVA scenario not executed".into(),
        });
    }

    report.notes.push(
        "Approval dry-run does not prove live reverse-request behavior. \
         Use TRACER_LIVE_GROK=1 approval-run."
            .into(),
    );
    report.notes.push(format!(
        "Default inducing prompt (public-safe): {DEFAULT_APPROVAL_INDUCING_PROMPT}"
    ));
    report.finalize_classification();
    Ok(report)
}

/// Live LVA suite execution.
pub fn run_approval_live(cfg: &RunConfig) -> Result<EvidenceReport, String> {
    let platform = discovery::platform_label();
    let mut report = EvidenceReport::new_suite(false, true, platform, SuiteKind::Lva);
    let mut event_types: Vec<String> = Vec::new();

    // --- Discovery ---
    let t0 = Instant::now();
    let disc = discovery::discover_grok(cfg.grok_override.as_deref());
    report.discovery = Some(disc.clone());
    let found = disc.found;
    report.push_stage(stage_evidence(
        StageId::Discovery,
        if found {
            StageStatus::Pass
        } else {
            StageStatus::Fail
        },
        t0.elapsed().as_millis() as u64,
        disc.notes.clone(),
        if found {
            None
        } else {
            Some("RuntimeExecutableNotFound".into())
        },
        Some(json!({
            "version": disc.version,
            "source": disc.source,
        })),
    ));
    if !found {
        push_all_scenarios(
            &mut report,
            cfg,
            LiveClassification::Fail,
            "grok binary not found",
        );
        report.finalize_classification();
        return Ok(report);
    }

    let exe = resolve_exe(&disc);
    let spec = grok_stdio_spawn_config(&exe, &cfg.cwd);
    report.spawn_plan = json!({
        "displayName": spec.display_name,
        "kind": spec.kind.as_str(),
        "executable": sanitize::display_path(std::path::Path::new(&exe), Some(&cfg.cwd)),
        "args": spec.args.clone(),
        "cwd": sanitize::display_path(&cfg.cwd, None),
        "isolateProcessTree": spec.isolate_process_tree,
        "killOnDrop": spec.kill_on_drop,
        "productHelper": "tracer_runtime_adapter::grok_stdio_spawn_config",
        "suite": "lva",
    });

    // --- Startup ---
    let t1 = Instant::now();
    let adapter = match RuntimeAdapter::start(spec, ProjectId::new(), SessionId::new()) {
        Ok(a) => Arc::new(a),
        Err(e) => {
            report.push_stage(stage_evidence(
                StageId::Startup,
                StageStatus::Fail,
                t1.elapsed().as_millis() as u64,
                vec![sanitize::sanitize_text(&e.message)],
                Some(format!("{:?}", e.error_class)),
                None,
            ));
            push_all_scenarios(
                &mut report,
                cfg,
                LiveClassification::Fail,
                &sanitize::sanitize_text(&e.message),
            );
            report.finalize_classification();
            return Ok(report);
        }
    };

    let alive = adapter.is_process_alive();
    drain_event_types(adapter.as_ref(), &mut event_types, 200);
    report.push_stage(stage_evidence(
        StageId::Startup,
        if alive {
            StageStatus::Pass
        } else {
            StageStatus::Fail
        },
        t1.elapsed().as_millis() as u64,
        vec![
            format!("process_alive={alive}"),
            "spawned via RuntimeAdapter::start + grok_stdio_spawn_config".into(),
        ],
        None,
        None,
    ));
    if !alive {
        let _ = adapter.force_kill();
        push_all_scenarios(
            &mut report,
            cfg,
            LiveClassification::Fail,
            "process not alive",
        );
        report.observed_event_types = event_types;
        report.finalize_classification();
        return Ok(report);
    }

    let setup_result = (|| -> Result<(), String> {
        // --- Initialize ---
        let t = Instant::now();
        match adapter.initialize() {
            Ok(caps) => {
                let ready = adapter.is_protocol_ready();
                drain_event_types(adapter.as_ref(), &mut event_types, 200);
                report.push_stage(stage_evidence(
                    StageId::Initialize,
                    if ready {
                        StageStatus::Pass
                    } else {
                        StageStatus::Fail
                    },
                    t.elapsed().as_millis() as u64,
                    vec![
                        format!("protocol_ready={ready}"),
                        format!("session_ready={}", adapter.is_session_ready()),
                        format!("caps.approvals={}", caps.approvals),
                        format!("caps.cancellation={}", caps.cancellation),
                    ],
                    None,
                    Some(json!({
                        "cancellation": caps.cancellation,
                        "approvals": caps.approvals,
                        "promptStreaming": caps.prompt_streaming,
                    })),
                ));
                if !ready {
                    return Err("protocol not ready after initialize".into());
                }
            }
            Err(e) => {
                report.push_stage(stage_evidence(
                    StageId::Initialize,
                    StageStatus::Fail,
                    t.elapsed().as_millis() as u64,
                    vec![sanitize::sanitize_text(&e.message)],
                    Some(format!("{:?}", e.error_class)),
                    None,
                ));
                return Err(sanitize::sanitize_text(&e.message));
            }
        }

        // --- Auth requirement (inspect only) ---
        let t = Instant::now();
        let st = adapter.inspect();
        let auth_methods: Vec<Value> = st
            .auth_methods
            .iter()
            .map(|m| {
                if let Some(obj) = m.as_object() {
                    let mut slim = serde_json::Map::new();
                    for key in ["id", "name", "description"] {
                        if let Some(v) = obj.get(key) {
                            slim.insert(key.to_string(), sanitize::sanitize_json(v));
                        }
                    }
                    Value::Object(slim)
                } else {
                    sanitize::sanitize_json(m)
                }
            })
            .collect();
        let auth_state = adapter.auth_state();
        report.push_stage(stage_evidence(
            StageId::AuthRequirement,
            StageStatus::Pass,
            t.elapsed().as_millis() as u64,
            vec![
                format!("auth_state={}", auth_state.as_str()),
                format!("auth_methods_count={}", auth_methods.len()),
                "tokens never printed".into(),
            ],
            None,
            Some(json!({ "authMethods": auth_methods, "authState": auth_state.as_str() })),
        ));

        // --- Session ---
        let t = Instant::now();
        let cwd_str = cfg.cwd.display().to_string();
        match adapter.create_session(SessionCreateParams {
            cwd: cwd_str,
            model_hints: None,
        }) {
            Ok(sid) => {
                drain_event_types(adapter.as_ref(), &mut event_types, 300);
                let session_ready = adapter.is_session_ready();
                report.push_stage(stage_evidence(
                    StageId::Session,
                    if session_ready {
                        StageStatus::Pass
                    } else {
                        StageStatus::Fail
                    },
                    t.elapsed().as_millis() as u64,
                    vec![
                        format!("session_ready={session_ready}"),
                        format!("runtime_session_id_len={}", sid.len()),
                    ],
                    None,
                    None,
                ));
                if !session_ready {
                    return Err("session not ready after create_session".into());
                }
            }
            Err(e) => {
                let auth_req = format!("{:?}", e.error_class).contains("AuthenticationRequired")
                    || e.message
                        .to_ascii_lowercase()
                        .contains("authentication required");
                let status = if auth_req {
                    StageStatus::Blocked
                } else {
                    StageStatus::Fail
                };
                report.push_stage(stage_evidence(
                    StageId::Session,
                    status,
                    t.elapsed().as_millis() as u64,
                    vec![
                        sanitize::sanitize_text(&e.message),
                        if auth_req {
                            "BLOCKED_BY_AUTH: session/new requires authentication".into()
                        } else {
                            "session/new failed".into()
                        },
                    ],
                    Some(format!("{:?}", e.error_class)),
                    None,
                ));
                let class = if auth_req {
                    LiveClassification::BlockedByAuth
                } else {
                    LiveClassification::Fail
                };
                push_all_scenarios(
                    &mut report,
                    cfg,
                    class,
                    if auth_req {
                        "requires authenticated session"
                    } else {
                        "session/new failed"
                    },
                );
                for stage in [
                    StageId::Prompt,
                    StageId::Stream,
                    StageId::Approval,
                    StageId::Cancel,
                ] {
                    report.push_stage(stage_evidence(
                        stage,
                        if auth_req {
                            StageStatus::Blocked
                        } else {
                            StageStatus::Fail
                        },
                        0,
                        vec![if auth_req {
                            "skipped: BLOCKED_BY_AUTH".into()
                        } else {
                            "skipped: session failed".into()
                        }],
                        None,
                        None,
                    ));
                }
                return Ok(());
            }
        }

        // Prefer approval-inducing default when operator left the smoke default prompt.
        let prompt_text = if cfg.prompt == crate::stages::DEFAULT_PUBLIC_PROMPT {
            DEFAULT_APPROVAL_INDUCING_PROMPT.to_string()
        } else {
            cfg.prompt.clone()
        };

        let mut plan: Vec<AttemptAction> = Vec::new();
        if scenario_enabled(cfg, "LVA-02") {
            plan.push(AttemptAction::AcceptOnce);
        }
        if scenario_enabled(cfg, "LVA-03") {
            plan.push(AttemptAction::RejectOnce);
        }
        if scenario_enabled(cfg, "LVA-04") || scenario_enabled(cfg, "LVA-05") {
            plan.push(AttemptAction::CancelWhilePending);
        }
        if plan.is_empty() {
            plan.push(AttemptAction::Observe);
        }

        let mut outcomes: Vec<AttemptOutcome> = Vec::new();
        for (i, action) in plan.iter().enumerate() {
            thread::sleep(ATTEMPT_SETTLE);
            let out = run_one_attempt(
                Arc::clone(&adapter),
                &prompt_text,
                *action,
                i,
                &mut event_types,
            );
            outcomes.push(out);
        }

        classify_lva_scenarios(&mut report, cfg, &outcomes, &event_types);

        let any_rr = outcomes.iter().any(|o| o.reverse_request_observed);
        let any_terminal = outcomes.iter().any(|o| o.terminal_observed)
            || event_types.iter().any(|t| {
                t == "session.completed" || t == "session.cancelled" || t == "session.failed"
            });
        let cancel_outcomes: Vec<_> = outcomes
            .iter()
            .filter(|o| matches!(o.action, AttemptAction::CancelWhilePending))
            .collect();
        let any_deadlock_ok = if cancel_outcomes.is_empty() {
            outcomes
                .iter()
                .all(|o| o.within_deadlock_budget && o.prompt_returned)
        } else {
            cancel_outcomes
                .iter()
                .all(|o| o.within_deadlock_budget && o.prompt_returned)
        };

        report.push_stage(stage_evidence(
            StageId::Prompt,
            StageStatus::Pass,
            0,
            vec![
                format!("approval-inducing attempts={}", outcomes.len()),
                format!("prompt_len={}", prompt_text.chars().count()),
                "prompt text is public-safe; not private operator content".into(),
            ],
            None,
            None,
        ));
        report.push_stage(stage_evidence(
            StageId::Stream,
            if !event_types.is_empty() {
                StageStatus::Pass
            } else {
                StageStatus::Fail
            },
            0,
            vec![format!("observed_event_types_count={}", event_types.len())],
            None,
            Some(json!({ "eventTypes": event_types.clone() })),
        ));
        report.push_stage(stage_evidence(
            StageId::Approval,
            StageStatus::Pass,
            0,
            vec![if any_rr {
                "approval.requested observed on at least one attempt — never auto-approved without scenario action".into()
            } else {
                "no approval.requested observed — LVA reverse-request scenarios classified NOT_OBSERVED / UNSUPPORTED_BY_PROMPT (not fabricated PASS)".into()
            }],
            None,
            Some(json!({
                "attempts": outcomes.len(),
                "reverseRequestObserved": any_rr,
            })),
        ));
        let cancel_attempted = !cancel_outcomes.is_empty();
        report.push_stage(stage_evidence(
            StageId::Cancel,
            if !cancel_attempted {
                StageStatus::Skipped
            } else if any_deadlock_ok {
                StageStatus::Pass
            } else {
                StageStatus::Fail
            },
            0,
            vec![
                format!("cancel_while_pending_attempted={cancel_attempted}"),
                format!("within_deadlock_budget={any_deadlock_ok}"),
                format!("terminal_observed={any_terminal}"),
            ],
            None,
            None,
        ));

        // Attach per-attempt notes (sanitized) to report notes.
        for (i, o) in outcomes.iter().enumerate() {
            report.notes.push(format!(
                "attempt[{i}] action={:?} rr={} resolved_ok={} cancel_ok={} terminal={} notes={}",
                o.action,
                o.reverse_request_observed,
                o.resolved_ok,
                o.cancel_ok,
                o.terminal_observed,
                o.notes.join("; ")
            ));
        }

        Ok(())
    })();

    if let Err(e) = setup_result {
        report
            .notes
            .push(format!("lva stage error: {}", sanitize::sanitize_text(&e)));
        if report.scenarios.is_empty() {
            push_all_scenarios(&mut report, cfg, LiveClassification::Fail, &e);
        }
    }

    // --- Shutdown (always if started) ---
    let t = Instant::now();
    let shut = adapter.shutdown(ShutdownOptions::default());
    thread::sleep(Duration::from_millis(100));
    let still_alive = adapter.is_process_alive();
    if still_alive {
        let _ = adapter.force_kill();
        thread::sleep(Duration::from_millis(100));
    }
    let orphan = adapter.is_process_alive();
    drain_event_types(adapter.as_ref(), &mut event_types, 100);
    report.push_stage(stage_evidence(
        StageId::Shutdown,
        if !orphan {
            StageStatus::Pass
        } else {
            StageStatus::Fail
        },
        t.elapsed().as_millis() as u64,
        vec![
            match &shut {
                Ok(()) => "shutdown Ok".into(),
                Err(e) => sanitize::sanitize_text(&e.message),
            },
            format!("alive_after_shutdown={still_alive}"),
            format!("orphan_after_force={orphan}"),
        ],
        None,
        None,
    ));

    if scenario_enabled(cfg, "LVA-07") {
        report.scenarios.retain(|s| s.id != "LVA-07");
        report.scenarios.push(ScenarioResult {
            id: "LVA-07".into(),
            status: if !orphan {
                LiveClassification::Pass
            } else {
                LiveClassification::Fail
            },
            detail: "shutdown leaves no orphan process".into(),
        });
    }

    if scenario_enabled(cfg, "LVA-06") && !report.scenarios.iter().any(|s| s.id == "LVA-06") {
        let terminal = event_types.iter().any(|t| {
            t == EventType::SessionCompleted.as_str()
                || t == EventType::SessionCancelled.as_str()
                || t == EventType::SessionFailed.as_str()
                || t == "session.completed"
                || t == "session.cancelled"
                || t == "session.failed"
        });
        report.scenarios.push(ScenarioResult {
            id: "LVA-06".into(),
            status: if terminal {
                LiveClassification::Pass
            } else {
                LiveClassification::NotObserved
            },
            detail: if terminal {
                "terminal session state observed".into()
            } else {
                "no terminal session event observed".into()
            },
        });
    }

    report.observed_event_types = event_types;
    report.finalize_classification();
    Ok(report)
}

fn run_one_attempt(
    adapter: Arc<RuntimeAdapter>,
    prompt_text: &str,
    action: AttemptAction,
    index: usize,
    global_events: &mut Vec<String>,
) -> AttemptOutcome {
    let mut local_events: Vec<String> = Vec::new();
    let approval_id_slot: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let saw_rr: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let action_ok: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    // Elapsed from cancel/resolve initiation to waiter exit (for LVA-05).
    let action_elapsed_ms: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
    let timed_out_slot: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let waiter_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let start = Instant::now();

    let a_wait = Arc::clone(&adapter);
    let aid_slot = Arc::clone(&approval_id_slot);
    let saw = Arc::clone(&saw_rr);
    let a_ok = Arc::clone(&action_ok);
    let a_elapsed = Arc::clone(&action_elapsed_ms);
    let timed_out = Arc::clone(&timed_out_slot);
    let w_events = Arc::clone(&waiter_events);
    let waiter = thread::spawn(move || {
        let deadline = Instant::now() + APPROVAL_WAIT;
        while Instant::now() < deadline {
            for ev in a_wait.drain_events() {
                if let AdapterEvent::Event(env) = ev {
                    let t = env.event_type.as_str().to_string();
                    {
                        let mut ge = w_events.lock().unwrap();
                        if !ge.contains(&t) {
                            ge.push(t.clone());
                        }
                    }
                    if env.event_type == EventType::ApprovalRequested
                        || env.event_type.as_str() == "approval.requested"
                    {
                        let aid = env
                            .payload
                            .get("approvalId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !aid.is_empty() {
                            *aid_slot.lock().unwrap() = Some(aid.clone());
                        }
                        *saw.lock().unwrap() = true;

                        let action_start = Instant::now();
                        match action {
                            AttemptAction::Observe => {
                                let _ = a_wait.cancel_prompt();
                                *a_ok.lock().unwrap() = true;
                            }
                            AttemptAction::AcceptOnce => {
                                if !aid.is_empty() {
                                    let r = a_wait.resolve_approval(ApprovalDecisionRequest {
                                        approval_id: aid,
                                        decision: "allow".into(),
                                        option_id: Some("allow-once".into()),
                                        reason: Some("lva-02".into()),
                                    });
                                    *a_ok.lock().unwrap() = r.is_ok();
                                }
                            }
                            AttemptAction::RejectOnce => {
                                if !aid.is_empty() {
                                    let r = a_wait.resolve_approval(ApprovalDecisionRequest {
                                        approval_id: aid,
                                        decision: "deny".into(),
                                        option_id: Some("reject-once".into()),
                                        reason: Some("lva-03".into()),
                                    });
                                    *a_ok.lock().unwrap() = r.is_ok();
                                }
                            }
                            AttemptAction::CancelWhilePending => {
                                let r = a_wait.cancel_prompt();
                                *a_ok.lock().unwrap() = r.is_ok()
                                    || r.as_ref()
                                        .err()
                                        .map(|e| {
                                            format!("{:?}", e.error_class)
                                                .contains("CapabilityUnsupported")
                                        })
                                        .unwrap_or(false);
                            }
                        }
                        *a_elapsed.lock().unwrap() =
                            Some(action_start.elapsed().as_millis() as u64);
                        return;
                    }
                    // If the turn completed without RR, stop waiting early.
                    if matches!(
                        env.event_type.as_str(),
                        "session.completed" | "session.failed"
                    ) {
                        *a_elapsed.lock().unwrap() = Some(0);
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_millis(25));
        }
        // Timeout: always free the turn so submit_prompt cannot hang forever.
        *timed_out.lock().unwrap() = true;
        let action_start = Instant::now();
        let _ = a_wait.cancel_prompt();
        *a_elapsed.lock().unwrap() = Some(action_start.elapsed().as_millis() as u64);
    });

    let prompt_res = adapter.submit_prompt(PromptRequest {
        prompt_id: Some(format!("lva-attempt-{index}")),
        text: prompt_text.to_string(),
    });
    let _ = waiter.join();
    let elapsed = start.elapsed();

    for t in waiter_events.lock().unwrap().iter() {
        if !local_events.contains(t) {
            local_events.push(t.clone());
        }
        if !global_events.contains(t) {
            global_events.push(t.clone());
        }
    }
    drain_event_types(adapter.as_ref(), &mut local_events, 400);
    for t in &local_events {
        if !global_events.contains(t) {
            global_events.push(t.clone());
        }
    }

    let reverse_request_observed = *saw_rr.lock().unwrap();
    let approval_id = approval_id_slot.lock().unwrap().clone();
    let action_succeeded = *action_ok.lock().unwrap();
    let action_ms = action_elapsed_ms.lock().unwrap().clone();
    let timed_out_without_rr = *timed_out_slot.lock().unwrap() && !reverse_request_observed;
    let terminal_observed = local_events.iter().any(|t| {
        t == "session.completed" || t == "session.cancelled" || t == "session.failed"
    }) || global_events.iter().any(|t| {
        t == "session.completed" || t == "session.cancelled" || t == "session.failed"
    });

    // Natural completion without RR (not a timeout cancel). Prefer completed
    // over cancelled so timeout→cancel is NOT_OBSERVED, not UNSUPPORTED.
    let saw_completed = local_events.iter().any(|t| t == "session.completed")
        || global_events.iter().any(|t| t == "session.completed");
    let prompt_completed_without_approval = !reverse_request_observed
        && !timed_out_without_rr
        && (saw_completed
            || (matches!(prompt_res, Ok(()))
                && local_events
                    .iter()
                    .any(|t| t.contains("message") || t == "session.completed")));

    // LVA-05: control returned (prompt RPC completed) and the cancel/resolve
    // path itself did not hang. Waiting for RR up to APPROVAL_WAIT is not a
    // deadlock — only post-action hang counts.
    let within_deadlock_budget = match action_ms {
        Some(ms) => {
            Duration::from_millis(ms) <= NO_DEADLOCK_BUDGET
                && elapsed <= APPROVAL_WAIT + NO_DEADLOCK_BUDGET
        }
        None => elapsed <= APPROVAL_WAIT + NO_DEADLOCK_BUDGET,
    };

    let mut notes = vec![
        format!("action={action:?}"),
        format!("elapsed_ms={}", elapsed.as_millis()),
        format!("action_elapsed_ms={action_ms:?}"),
        format!("reverse_request_observed={reverse_request_observed}"),
        format!("timed_out_without_rr={timed_out_without_rr}"),
        format!("action_succeeded={action_succeeded}"),
        match &prompt_res {
            Ok(()) => "prompt RPC returned Ok".into(),
            Err(e) => format!(
                "prompt RPC: {} ({:?})",
                sanitize::sanitize_text(&e.message),
                e.error_class
            ),
        },
    ];
    if let Some(id) = &approval_id {
        notes.push(format!("approval_id_len={}", id.len()));
    }

    AttemptOutcome {
        action,
        reverse_request_observed,
        resolved_ok: action_succeeded
            && matches!(
                action,
                AttemptAction::AcceptOnce | AttemptAction::RejectOnce
            ),
        cancel_ok: action_succeeded && matches!(action, AttemptAction::CancelWhilePending),
        prompt_returned: true,
        terminal_observed,
        prompt_completed_without_approval,
        timed_out_without_rr,
        within_deadlock_budget,
        notes,
    }
}

/// Map attempt outcomes to LVA-01…LVA-06 classifications (LVA-07 set at shutdown).
fn classify_lva_scenarios(
    report: &mut EvidenceReport,
    cfg: &RunConfig,
    outcomes: &[AttemptOutcome],
    event_types: &[String],
) {
    let any_rr = outcomes.iter().any(|o| o.reverse_request_observed);
    let any_completed_no_rr = outcomes
        .iter()
        .any(|o| o.prompt_completed_without_approval);
    let any_timeout_no_rr = outcomes.iter().any(|o| o.timed_out_without_rr);
    let accept = outcomes
        .iter()
        .find(|o| matches!(o.action, AttemptAction::AcceptOnce));
    let reject = outcomes
        .iter()
        .find(|o| matches!(o.action, AttemptAction::RejectOnce));
    let cancel = outcomes
        .iter()
        .find(|o| matches!(o.action, AttemptAction::CancelWhilePending));

    // LVA-01: reverse-request observed
    if scenario_enabled(cfg, "LVA-01") {
        let (status, detail) = if any_rr {
            (
                LiveClassification::Pass,
                "approval reverse-request observed".to_string(),
            )
        } else if any_completed_no_rr && !any_timeout_no_rr {
            (
                LiveClassification::UnsupportedByPrompt,
                "provider completed without session/request_permission for inducing prompt"
                    .to_string(),
            )
        } else {
            (
                LiveClassification::NotObserved,
                "approval.requested not observed within wait budget".to_string(),
            )
        };
        report.scenarios.push(ScenarioResult {
            id: "LVA-01".into(),
            status,
            detail,
        });
    }

    // Helper for scenarios that require a reverse-request
    let dep = |attempt: Option<&AttemptOutcome>,
               success: bool,
               pass_detail: &str,
               fail_detail: &str|
     -> (LiveClassification, String) {
        match attempt {
            Some(o) if o.reverse_request_observed && success => {
                (LiveClassification::Pass, pass_detail.into())
            }
            Some(o) if o.reverse_request_observed && !success => {
                (LiveClassification::Fail, fail_detail.into())
            }
            Some(o) if o.prompt_completed_without_approval && !o.timed_out_without_rr => (
                LiveClassification::UnsupportedByPrompt,
                "no reverse-request; cannot exercise decision path".into(),
            ),
            Some(_) | None => (
                LiveClassification::NotObserved,
                "reverse-request not observed; decision path not exercised".into(),
            ),
        }
    };

    if scenario_enabled(cfg, "LVA-02") {
        let (status, detail) = dep(
            accept,
            accept.map(|o| o.resolved_ok).unwrap_or(false),
            "approval accepted once (allow-once)",
            "reverse-request seen but allow-once resolve failed",
        );
        report.scenarios.push(ScenarioResult {
            id: "LVA-02".into(),
            status,
            detail,
        });
    }

    if scenario_enabled(cfg, "LVA-03") {
        let (status, detail) = dep(
            reject,
            reject.map(|o| o.resolved_ok).unwrap_or(false),
            "approval rejected once (reject-once)",
            "reverse-request seen but reject-once resolve failed",
        );
        report.scenarios.push(ScenarioResult {
            id: "LVA-03".into(),
            status,
            detail,
        });
    }

    if scenario_enabled(cfg, "LVA-04") {
        let (status, detail) = dep(
            cancel,
            cancel.map(|o| o.cancel_ok).unwrap_or(false),
            "cancel while approval pending returned",
            "reverse-request seen but cancel-while-pending failed",
        );
        report.scenarios.push(ScenarioResult {
            id: "LVA-04".into(),
            status,
            detail,
        });
    }

    // LVA-05: no deadlock — control returns within budget for exercised paths.
    if scenario_enabled(cfg, "LVA-05") {
        let relevant: Vec<_> = if let Some(c) = cancel {
            vec![c]
        } else {
            outcomes.iter().collect()
        };
        let ok = !relevant.is_empty()
            && relevant
                .iter()
                .all(|o| o.prompt_returned && o.within_deadlock_budget);
        // Deadlock check is meaningful even without RR (prompt must still return).
        let (status, detail) = if ok {
            (
                LiveClassification::Pass,
                "no deadlock: prompt/control returned within budget".to_string(),
            )
        } else if relevant.is_empty() {
            (
                LiveClassification::NotObserved,
                "no attempts to evaluate deadlock budget".to_string(),
            )
        } else {
            (
                LiveClassification::Fail,
                "prompt/control exceeded deadlock budget or did not return".to_string(),
            )
        };
        report.scenarios.push(ScenarioResult {
            id: "LVA-05".into(),
            status,
            detail,
        });
    }

    // LVA-06: terminal state observed
    if scenario_enabled(cfg, "LVA-06") {
        let terminal = outcomes.iter().any(|o| o.terminal_observed)
            || event_types.iter().any(|t| {
                t == "session.completed" || t == "session.cancelled" || t == "session.failed"
            });
        report.scenarios.push(ScenarioResult {
            id: "LVA-06".into(),
            status: if terminal {
                LiveClassification::Pass
            } else {
                LiveClassification::NotObserved
            },
            detail: if terminal {
                "terminal session state observed".into()
            } else {
                "no terminal session event observed".into()
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn approval_dry_run_lists_lva_not_run() {
        let cfg = RunConfig {
            live: false,
            through: None,
            scenarios: None,
            cwd: PathBuf::from("."),
            grok_override: Some(PathBuf::from("grok")),
            prompt: DEFAULT_APPROVAL_INDUCING_PROMPT.into(),
            allow_unauth: true,
        };
        let report = run_approval_dry_run(&cfg).expect("dry-run");
        assert_eq!(report.suite, SuiteKind::Lva);
        assert_eq!(report.work_item, "W2-D");
        assert!(report.dry_run);
        assert_eq!(report.classification, LiveClassification::NotRun);
        for id in LVA_SCENARIOS {
            assert!(
                report
                    .scenarios
                    .iter()
                    .any(|s| s.id == *id && s.status == LiveClassification::NotRun),
                "missing {id}"
            );
        }
        assert_eq!(report.spawn_plan["matchesW0bW1d"], true);
        assert_eq!(
            report.spawn_plan["productHelper"],
            "tracer_runtime_adapter::grok_stdio_spawn_config"
        );
    }

    #[test]
    fn classify_does_not_pass_without_reverse_request() {
        let cfg = RunConfig {
            live: true,
            through: None,
            scenarios: None,
            cwd: PathBuf::from("."),
            grok_override: None,
            prompt: DEFAULT_APPROVAL_INDUCING_PROMPT.into(),
            allow_unauth: true,
        };
        let mut report = EvidenceReport::new_suite(false, true, "test".into(), SuiteKind::Lva);
        let outcomes = vec![AttemptOutcome {
            action: AttemptAction::AcceptOnce,
            reverse_request_observed: false,
            resolved_ok: false,
            cancel_ok: false,
            prompt_returned: true,
            terminal_observed: true,
            prompt_completed_without_approval: true,
            timed_out_without_rr: false,
            within_deadlock_budget: true,
            notes: vec![],
        }];
        classify_lva_scenarios(
            &mut report,
            &cfg,
            &outcomes,
            &["session.completed".into()],
        );
        let s01 = report.scenarios.iter().find(|s| s.id == "LVA-01").unwrap();
        assert_eq!(s01.status, LiveClassification::UnsupportedByPrompt);
        let s02 = report.scenarios.iter().find(|s| s.id == "LVA-02").unwrap();
        assert_ne!(s02.status, LiveClassification::Pass);
        assert_eq!(s02.status, LiveClassification::UnsupportedByPrompt);
    }

    #[test]
    fn classify_timeout_without_rr_is_not_observed() {
        let cfg = RunConfig {
            live: true,
            through: None,
            scenarios: Some(vec!["LVA-01".into(), "LVA-02".into()]),
            cwd: PathBuf::from("."),
            grok_override: None,
            prompt: DEFAULT_APPROVAL_INDUCING_PROMPT.into(),
            allow_unauth: true,
        };
        let mut report = EvidenceReport::new_suite(false, true, "test".into(), SuiteKind::Lva);
        let outcomes = vec![AttemptOutcome {
            action: AttemptAction::AcceptOnce,
            reverse_request_observed: false,
            resolved_ok: false,
            cancel_ok: false,
            prompt_returned: true,
            terminal_observed: true,
            prompt_completed_without_approval: false,
            timed_out_without_rr: true,
            within_deadlock_budget: true,
            notes: vec![],
        }];
        classify_lva_scenarios(
            &mut report,
            &cfg,
            &outcomes,
            &["session.cancelled".into()],
        );
        assert_eq!(
            report
                .scenarios
                .iter()
                .find(|s| s.id == "LVA-01")
                .unwrap()
                .status,
            LiveClassification::NotObserved
        );
        assert_eq!(
            report
                .scenarios
                .iter()
                .find(|s| s.id == "LVA-02")
                .unwrap()
                .status,
            LiveClassification::NotObserved
        );
    }

    #[test]
    fn classify_pass_when_reverse_request_and_resolve() {
        let cfg = RunConfig {
            live: true,
            through: None,
            scenarios: Some(vec!["LVA-01".into(), "LVA-02".into()]),
            cwd: PathBuf::from("."),
            grok_override: None,
            prompt: DEFAULT_APPROVAL_INDUCING_PROMPT.into(),
            allow_unauth: true,
        };
        let mut report = EvidenceReport::new_suite(false, true, "test".into(), SuiteKind::Lva);
        let outcomes = vec![AttemptOutcome {
            action: AttemptAction::AcceptOnce,
            reverse_request_observed: true,
            resolved_ok: true,
            cancel_ok: false,
            prompt_returned: true,
            terminal_observed: true,
            prompt_completed_without_approval: false,
            timed_out_without_rr: false,
            within_deadlock_budget: true,
            notes: vec![],
        }];
        classify_lva_scenarios(&mut report, &cfg, &outcomes, &["approval.requested".into()]);
        assert_eq!(
            report
                .scenarios
                .iter()
                .find(|s| s.id == "LVA-01")
                .unwrap()
                .status,
            LiveClassification::Pass
        );
        assert_eq!(
            report
                .scenarios
                .iter()
                .find(|s| s.id == "LVA-02")
                .unwrap()
                .status,
            LiveClassification::Pass
        );
    }
}
