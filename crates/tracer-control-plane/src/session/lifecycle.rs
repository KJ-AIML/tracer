//! Drain lifecycle model + event-after-terminal policy (W2.2-C).
//!
//! # Authoritative completion
//!
//! Adapter prompt return is **not** authoritative for ingestion completion.
//! Authoritative phases for a prompt cycle:
//!
//! 1. `AdapterTerminalObserved` — first terminal event seen on the drain path
//! 2. `TerminalPersisted` — terminal event successfully appended to SQLite
//! 3. `TerminalStateCommitted` — session state + presentation updated post-persist
//! 4. Late-event grace / source closed / bounded drain complete
//! 5. Drain + pump joined → safe for runtime shutdown
//!
//! Presentation of terminal status must only happen at/after step 3
//! (post-persist publish). Prompt RPC return may race ahead of 2–4 and must
//! not end ingestion early.

use std::time::Duration;
use tracer_domain::SessionStatus;

/// Bounded grace after a terminal event for late metadata / trailing frames.
pub const LATE_EVENT_GRACE: Duration = Duration::from_millis(500);

/// Max wait for the async persist pump to exit after the bridge closes.
pub const LATE_DRAIN_JOIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Observed lifecycle phase for a live session's ingest path.
///
/// Monotonic for a single session lifetime (may re-enter `PromptActive` on a
/// subsequent prompt while drain remains active).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrainLifecyclePhase {
    /// LiveSession constructed; ingestor not yet started.
    #[default]
    RuntimeStarted,
    /// OS drain + async pump running.
    EventDrainActive,
    /// Prompt RPC in flight (control-plane marked Running).
    PromptActive,
    /// Terminal event type observed on drain (not yet necessarily persisted).
    AdapterTerminalObserved,
    /// Terminal event row committed to SQLite.
    TerminalPersisted,
    /// Session state + presentation projection updated after terminal persist.
    TerminalStateCommitted,
    /// Adapter `submit_prompt` (or equivalent) returned to the control plane.
    AdapterOperationReturned,
    /// Post-return / post-terminal late-event grace window.
    LateEventGrace,
    /// Adapter event source closed or stop requested and bridge drained.
    SourceClosedOrBoundedDrainComplete,
    /// OS drain thread joined.
    DrainTaskJoined,
    /// Ingest fully stopped; safe for process shutdown / Drop.
    RuntimeShutdown,
}

impl DrainLifecyclePhase {
    fn rank(self) -> u8 {
        match self {
            Self::RuntimeStarted => 0,
            Self::EventDrainActive => 1,
            Self::PromptActive => 2,
            Self::AdapterTerminalObserved => 3,
            Self::TerminalPersisted => 4,
            Self::TerminalStateCommitted => 5,
            Self::AdapterOperationReturned => 6,
            Self::LateEventGrace => 7,
            Self::SourceClosedOrBoundedDrainComplete => 8,
            Self::DrainTaskJoined => 9,
            Self::RuntimeShutdown => 10,
        }
    }

    /// Advance phase only forward (except re-entry into `PromptActive` for a new run).
    pub fn advance_to(self, next: Self) -> Self {
        if next == Self::PromptActive
            && matches!(
                self,
                Self::EventDrainActive
                    | Self::TerminalStateCommitted
                    | Self::AdapterOperationReturned
                    | Self::LateEventGrace
                    | Self::TerminalPersisted
                    | Self::AdapterTerminalObserved
            )
        {
            return Self::PromptActive;
        }
        if next.rank() >= self.rank() {
            next
        } else {
            self
        }
    }
}

/// How to treat an event that arrives after a prompt-cycle terminal is committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateEventDisposition {
    /// Apply state transition normally (may upgrade severity, e.g. crash after cancel).
    ApplyFully,
    /// Persist and apply non-status fields; never reopen a completed run.
    PersistNoStatusRegression,
    /// Duplicate terminal of the same class — persist/count, ignore status churn.
    DuplicateTerminal,
    /// Expected channel lifecycle signal (not an event payload).
    ExpectedChannelClose,
}

/// Prompt-cycle terminal event types (end of agent run, not process lifecycle).
pub fn is_prompt_terminal_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "session.completed" | "session.failed" | "session.cancelled"
    )
}

/// Session statuses that are terminal for a live run.
pub fn is_run_terminal_status(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Failed | SessionStatus::Disconnected | SessionStatus::Stopped
            | SessionStatus::Completed
    )
}

/// Policy for events observed after a prompt terminal has been persisted.
///
/// | Case | Disposition |
/// |---|---|
/// | Duplicate terminal | `DuplicateTerminal` |
/// | Late non-terminal (deltas, tools, prompt.submitted) | `PersistNoStatusRegression` |
/// | Late metadata / protocol unknown | `PersistNoStatusRegression` |
/// | Process exit / crash after terminal | `ApplyFully` (may upgrade to Failed) |
/// | Channel close without terminal | handled outside (not an event) |
pub fn late_event_disposition(
    already_terminal: bool,
    prior_terminal_event: Option<&str>,
    event_type: &str,
) -> LateEventDisposition {
    if !already_terminal {
        return LateEventDisposition::ApplyFully;
    }
    if is_prompt_terminal_event(event_type) {
        if prior_terminal_event == Some(event_type) {
            return LateEventDisposition::DuplicateTerminal;
        }
        // Different terminal class (e.g. completed then failed) — allow upgrade path.
        return LateEventDisposition::ApplyFully;
    }
    match event_type {
        "runtime.process.exited" | "runtime.process.failed" => LateEventDisposition::ApplyFully,
        _ => LateEventDisposition::PersistNoStatusRegression,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_advances_monotonically() {
        let p = DrainLifecyclePhase::EventDrainActive;
        assert_eq!(
            p.advance_to(DrainLifecyclePhase::PromptActive),
            DrainLifecyclePhase::PromptActive
        );
        let p = DrainLifecyclePhase::TerminalPersisted;
        // Going backwards is ignored.
        assert_eq!(
            p.advance_to(DrainLifecyclePhase::PromptActive),
            DrainLifecyclePhase::PromptActive
        ); // re-entry allowed for new prompt
        let p = DrainLifecyclePhase::DrainTaskJoined;
        assert_eq!(
            p.advance_to(DrainLifecyclePhase::EventDrainActive),
            DrainLifecyclePhase::DrainTaskJoined
        );
    }

    #[test]
    fn late_policy_duplicate_terminal() {
        assert_eq!(
            late_event_disposition(true, Some("session.completed"), "session.completed"),
            LateEventDisposition::DuplicateTerminal
        );
    }

    #[test]
    fn late_policy_non_terminal_no_regression() {
        assert_eq!(
            late_event_disposition(true, Some("session.completed"), "agent.message.delta"),
            LateEventDisposition::PersistNoStatusRegression
        );
        assert_eq!(
            late_event_disposition(true, Some("session.completed"), "session.ready"),
            LateEventDisposition::PersistNoStatusRegression
        );
    }

    #[test]
    fn late_policy_process_exit_applies() {
        assert_eq!(
            late_event_disposition(true, Some("session.completed"), "runtime.process.exited"),
            LateEventDisposition::ApplyFully
        );
    }

    #[test]
    fn pre_terminal_always_full() {
        assert_eq!(
            late_event_disposition(false, None, "agent.message.delta"),
            LateEventDisposition::ApplyFully
        );
    }
}
