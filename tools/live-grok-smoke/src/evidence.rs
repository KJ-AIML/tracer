//! Structured, sanitized evidence report for live Grok smoke.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;

use crate::discovery::DiscoveryResult;
use crate::stages::StageId;

/// Overall / scenario classification (LVS + LVA task contracts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiveClassification {
    /// Dry-run / plan only; live stages not executed.
    NotRun,
    /// Auth gate blocked session / prompt stages.
    BlockedByAuth,
    /// Some stages passed; authenticated path incomplete.
    Partial,
    /// All requested live scenarios passed.
    Pass,
    /// Unexpected failure (not auth-block).
    Fail,
    /// Live ran but approval reverse-request was not observed (LVA honesty).
    NotObserved,
    /// Provider completed without permission reverse-request for the inducing prompt (LVA).
    UnsupportedByPrompt,
}

/// Per-stage status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Pass,
    Fail,
    Skipped,
    Blocked,
    NotRun,
}

/// One lifecycle stage result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageEvidence {
    pub stage: String,
    pub stage_id: String,
    pub status: StageStatus,
    pub duration_ms: u64,
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Named LVS scenario result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioResult {
    pub id: String,
    pub status: LiveClassification,
    pub detail: String,
}

/// Which scenario suite this report covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuiteKind {
    /// VS1-H1 LVS-01…LVS-08 smoke.
    Lvs,
    /// W2-D LVA-01…LVA-07 live approval validation.
    Lva,
}

/// Full harness report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceReport {
    pub schema_version: u32,
    pub harness: String,
    pub work_item: String,
    /// Always: manual_local_live_authenticated_smoke
    pub classification_tier: String,
    pub classification: LiveClassification,
    pub generated_at: String,
    pub platform: String,
    pub dry_run: bool,
    pub live_opt_in: bool,
    /// `lvs` (default smoke) or `lva` (approval reverse-request suite).
    pub suite: SuiteKind,
    pub discovery: Option<DiscoveryResult>,
    pub spawn_plan: Value,
    pub stages: Vec<StageEvidence>,
    pub scenarios: Vec<ScenarioResult>,
    pub assumptions_checked: Vec<AssumptionCheck>,
    pub notes: Vec<String>,
    /// Sanitized event type sequence observed (not full payloads with secrets).
    pub observed_event_types: Vec<String>,
}

/// Compare observed behavior vs W0-B / W1-D assumptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssumptionCheck {
    pub id: String,
    pub source: String,
    pub statement: String,
    pub observed: String,
    pub match_status: String,
}

impl EvidenceReport {
    pub fn new(dry_run: bool, live_opt_in: bool, platform: String) -> Self {
        Self::new_suite(dry_run, live_opt_in, platform, SuiteKind::Lvs)
    }

    pub fn new_suite(dry_run: bool, live_opt_in: bool, platform: String, suite: SuiteKind) -> Self {
        let now = OffsetDateTime::now_utc();
        let generated_at = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let (work_item, notes) = match suite {
            SuiteKind::Lvs => (
                "VS1-H1".into(),
                vec![
                    "Credentials are never stored or printed by this harness.".into(),
                    "Standard CI must not invoke `run` without opt-in env (and CI matrix forbids live).".into(),
                ],
            ),
            SuiteKind::Lva => (
                "W2-D".into(),
                vec![
                    "Credentials are never stored or printed by this harness.".into(),
                    "Standard CI must not invoke approval-run without TRACER_LIVE_GROK=1.".into(),
                    "Never auto-approve: allow/deny only via explicit LVA scenario actions.".into(),
                    "Do not claim LVA PASS without an observed approval.requested reverse-request.".into(),
                ],
            ),
        };
        Self {
            schema_version: 1,
            harness: "live-grok-smoke".into(),
            work_item,
            classification_tier: "manual_local_live_authenticated_smoke".into(),
            classification: if dry_run {
                LiveClassification::NotRun
            } else {
                LiveClassification::Partial
            },
            generated_at,
            platform,
            dry_run,
            live_opt_in,
            suite,
            discovery: None,
            spawn_plan: json!({}),
            stages: Vec::new(),
            scenarios: Vec::new(),
            assumptions_checked: default_assumptions(),
            notes,
            observed_event_types: Vec::new(),
        }
    }

    pub fn push_stage(&mut self, stage: StageEvidence) {
        self.stages.push(stage);
    }

    pub fn finalize_classification(&mut self) {
        if self.dry_run {
            // Dry-run validates construction; treat as Pass if all construction stages pass.
            let any_fail = self.stages.iter().any(|s| s.status == StageStatus::Fail);
            self.classification = if any_fail {
                LiveClassification::Fail
            } else {
                LiveClassification::NotRun
            };
            return;
        }

        if self.suite == SuiteKind::Lva {
            self.finalize_lva_classification();
            return;
        }

        let any_fail = self.stages.iter().any(|s| s.status == StageStatus::Fail);
        let any_blocked = self.stages.iter().any(|s| s.status == StageStatus::Blocked);
        let live_stages: Vec<_> = self
            .stages
            .iter()
            .filter(|s| s.status == StageStatus::Pass || s.status == StageStatus::Fail)
            .collect();

        if any_fail {
            // Auth-only failures may still be classified as BLOCKED if only blocked stages.
            self.classification = LiveClassification::Fail;
            // Prefer BLOCKED_BY_AUTH when failures are solely auth-gated session/prompt
            // and operator marked allow_unauth via stage Blocked status.
            return;
        }

        if any_blocked {
            let authenticated_pass = self.scenarios.iter().any(|s| {
                matches!(s.id.as_str(), "LVS-04" | "LVS-05" | "LVS-06")
                    && s.status == LiveClassification::Pass
            });
            self.classification = if authenticated_pass {
                LiveClassification::Partial
            } else {
                LiveClassification::BlockedByAuth
            };
            return;
        }

        let required = [
            "LVS-01", "LVS-02", "LVS-03", "LVS-04", "LVS-05", "LVS-06", "LVS-07", "LVS-08",
        ];
        let all_required_pass = required.iter().all(|id| {
            self.scenarios
                .iter()
                .any(|s| s.id == *id && s.status == LiveClassification::Pass)
        });

        if all_required_pass {
            self.classification = LiveClassification::Pass;
        } else if live_stages.is_empty() {
            self.classification = LiveClassification::NotRun;
        } else {
            self.classification = LiveClassification::Partial;
        }
    }

    /// LVA overall classification: honest about non-observation (never fabricates PASS).
    fn finalize_lva_classification(&mut self) {
        let status_of = |id: &str| self.scenarios.iter().find(|s| s.id == id).map(|s| s.status);

        if self
            .scenarios
            .iter()
            .any(|s| s.status == LiveClassification::Fail)
            || self.stages.iter().any(|s| s.status == StageStatus::Fail)
        {
            // Prefer auth block when session never authenticated.
            if self
                .scenarios
                .iter()
                .any(|s| s.status == LiveClassification::BlockedByAuth)
                || self.stages.iter().any(|s| s.status == StageStatus::Blocked)
            {
                // Fail still wins if there is a true product fail; only pure auth → blocked.
                let only_auth_or_blocked = !self.scenarios.iter().any(|s| {
                    s.status == LiveClassification::Fail
                        && !s.detail.to_ascii_lowercase().contains("auth")
                }) && !self.stages.iter().any(|s| {
                    s.status == StageStatus::Fail
                        && s.stage_id != "discovery"
                        && s.stage_id != "startup"
                        && s.stage_id != "initialize"
                });
                if only_auth_or_blocked
                    && self
                        .scenarios
                        .iter()
                        .any(|s| s.status == LiveClassification::BlockedByAuth)
                {
                    self.classification = LiveClassification::BlockedByAuth;
                    return;
                }
            }
            self.classification = LiveClassification::Fail;
            return;
        }

        if self
            .scenarios
            .iter()
            .any(|s| s.status == LiveClassification::BlockedByAuth)
            || self.stages.iter().any(|s| s.status == StageStatus::Blocked)
        {
            self.classification = LiveClassification::BlockedByAuth;
            return;
        }

        let required = [
            "LVA-01", "LVA-02", "LVA-03", "LVA-04", "LVA-05", "LVA-06", "LVA-07",
        ];
        let all_pass = required
            .iter()
            .all(|id| status_of(id) == Some(LiveClassification::Pass));
        if all_pass {
            self.classification = LiveClassification::Pass;
            return;
        }

        // Honest non-observation aggregates.
        let any_not_observed = self
            .scenarios
            .iter()
            .any(|s| s.status == LiveClassification::NotObserved);
        let any_unsupported = self
            .scenarios
            .iter()
            .any(|s| s.status == LiveClassification::UnsupportedByPrompt);
        let any_pass = self
            .scenarios
            .iter()
            .any(|s| s.status == LiveClassification::Pass);

        if any_unsupported && !any_pass {
            self.classification = LiveClassification::UnsupportedByPrompt;
        } else if any_not_observed && !any_pass {
            self.classification = LiveClassification::NotObserved;
        } else if any_pass || any_not_observed || any_unsupported {
            self.classification = LiveClassification::Partial;
        } else {
            self.classification = LiveClassification::NotRun;
        }
    }
}

fn default_assumptions() -> Vec<AssumptionCheck> {
    vec![
        AssumptionCheck {
            id: "A-W0B-01".into(),
            source: "docs/research/grok-build/PROCESS_LIFECYCLE.md".into(),
            statement: "Stock start command is `grok agent --no-leader stdio`".into(),
            observed: "pending".into(),
            match_status: "pending".into(),
        },
        AssumptionCheck {
            id: "A-W1D-01".into(),
            source: "crates/tracer-runtime-adapter/src/config.rs::grok_stdio_spawn_config".into(),
            statement: "W1-D spawn helper emits agent --no-leader stdio".into(),
            observed: "pending".into(),
            match_status: "pending".into(),
        },
        AssumptionCheck {
            id: "A-W0B-02".into(),
            source: "docs/research/grok-build/W0-B_COMPLETION_REPORT.md".into(),
            statement: "initialize may succeed without credentials".into(),
            observed: "pending".into(),
            match_status: "pending".into(),
        },
        AssumptionCheck {
            id: "A-W0B-03".into(),
            source: "docs/research/grok-build/PROCESS_LIFECYCLE.md".into(),
            statement: "session/new without auth returns Authentication required".into(),
            observed: "pending".into(),
            match_status: "pending".into(),
        },
        AssumptionCheck {
            id: "A-W1D-02".into(),
            source: "docs/modules/w1-d/W1_D_PUBLIC_INTERFACE.md".into(),
            statement: "process alive ≠ protocol ready ≠ session ready".into(),
            observed: "pending".into(),
            match_status: "pending".into(),
        },
    ]
}

pub fn stage_evidence(
    id: StageId,
    status: StageStatus,
    duration_ms: u64,
    notes: Vec<String>,
    error_class: Option<String>,
    details: Option<Value>,
) -> StageEvidence {
    StageEvidence {
        stage: id.display_name().into(),
        stage_id: id.as_str().into(),
        status,
        duration_ms,
        notes,
        error_class,
        details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_finalize_not_run() {
        let mut r = EvidenceReport::new(true, false, "test".into());
        r.push_stage(stage_evidence(
            StageId::Discovery,
            StageStatus::Pass,
            1,
            vec!["dry-run".into()],
            None,
            None,
        ));
        r.finalize_classification();
        assert_eq!(r.classification, LiveClassification::NotRun);
    }

    #[test]
    fn lva_finalize_never_pass_without_all_pass() {
        let mut r = EvidenceReport::new_suite(false, true, "test".into(), SuiteKind::Lva);
        for id in [
            "LVA-01", "LVA-02", "LVA-03", "LVA-04", "LVA-05", "LVA-06", "LVA-07",
        ] {
            r.scenarios.push(ScenarioResult {
                id: id.into(),
                status: if id == "LVA-01" {
                    LiveClassification::NotObserved
                } else {
                    LiveClassification::Pass
                },
                detail: "unit".into(),
            });
        }
        r.finalize_classification();
        assert_ne!(r.classification, LiveClassification::Pass);
        assert_eq!(r.classification, LiveClassification::Partial);
    }

    #[test]
    fn lva_finalize_unsupported_when_all_unsupported() {
        let mut r = EvidenceReport::new_suite(false, true, "test".into(), SuiteKind::Lva);
        r.scenarios.push(ScenarioResult {
            id: "LVA-01".into(),
            status: LiveClassification::UnsupportedByPrompt,
            detail: "unit".into(),
        });
        r.scenarios.push(ScenarioResult {
            id: "LVA-02".into(),
            status: LiveClassification::UnsupportedByPrompt,
            detail: "unit".into(),
        });
        r.finalize_classification();
        assert_eq!(r.classification, LiveClassification::UnsupportedByPrompt);
    }

    #[test]
    fn lva_finalize_blocked_by_auth() {
        let mut r = EvidenceReport::new_suite(false, true, "test".into(), SuiteKind::Lva);
        r.push_stage(stage_evidence(
            StageId::Session,
            StageStatus::Blocked,
            1,
            vec!["BLOCKED_BY_AUTH".into()],
            None,
            None,
        ));
        for id in [
            "LVA-01", "LVA-02", "LVA-03", "LVA-04", "LVA-05", "LVA-06", "LVA-07",
        ] {
            r.scenarios.push(ScenarioResult {
                id: id.into(),
                status: LiveClassification::BlockedByAuth,
                detail: "requires authenticated session".into(),
            });
        }
        r.finalize_classification();
        assert_eq!(r.classification, LiveClassification::BlockedByAuth);
    }
}
