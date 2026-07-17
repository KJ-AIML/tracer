//! Structured, sanitized evidence report for live Grok smoke.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;

use crate::discovery::DiscoveryResult;
use crate::stages::StageId;

/// Overall live result classification (task contract).
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
        Self {
            schema_version: 1,
            harness: "live-grok-smoke".into(),
            work_item: "VS1-H1".into(),
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
            discovery: None,
            spawn_plan: json!({}),
            stages: Vec::new(),
            scenarios: Vec::new(),
            assumptions_checked: default_assumptions(),
            notes: vec![
                "Credentials are never stored or printed by this harness.".into(),
                "Standard CI must not invoke `run` without opt-in env (and CI matrix forbids live).".into(),
            ],
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

        let required = ["LVS-01", "LVS-02", "LVS-03", "LVS-04", "LVS-05", "LVS-06", "LVS-07", "LVS-08"];
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
}
