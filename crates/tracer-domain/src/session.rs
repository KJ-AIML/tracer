//! Session lifecycle status model.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Control-plane session status catalog (W0-A / event protocol).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session record is being created.
    Creating,
    /// Runtime process is starting / initializing.
    StartingRuntime,
    /// Session may accept prompts.
    Ready,
    /// Agent run is active.
    Running,
    /// Blocked on user/policy approval.
    AwaitingApproval,
    /// Cancellation in progress.
    Cancelling,
    /// Terminal: completed successfully (run or session per control plane).
    Completed,
    /// Terminal: failed.
    Failed,
    /// Terminal: process disconnected unexpectedly.
    Disconnected,
    /// Terminal: stopped (user stop / cancelled end state).
    Stopped,
}

impl SessionStatus {
    /// All catalog values in a stable order.
    pub const ALL: &'static [SessionStatus] = &[
        Self::Creating,
        Self::StartingRuntime,
        Self::Ready,
        Self::Running,
        Self::AwaitingApproval,
        Self::Cancelling,
        Self::Completed,
        Self::Failed,
        Self::Disconnected,
        Self::Stopped,
    ];

    /// Wire string form (snake_case).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Creating => "creating",
            Self::StartingRuntime => "starting_runtime",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Cancelling => "cancelling",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Disconnected => "disconnected",
            Self::Stopped => "stopped",
        }
    }

    /// Parse wire string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "creating" => Some(Self::Creating),
            "starting_runtime" => Some(Self::StartingRuntime),
            "ready" => Some(Self::Ready),
            "running" => Some(Self::Running),
            "awaiting_approval" => Some(Self::AwaitingApproval),
            "cancelling" => Some(Self::Cancelling),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "disconnected" => Some(Self::Disconnected),
            "stopped" => Some(Self::Stopped),
            _ => None,
        }
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether the status is terminal (no further productive transitions expected).
pub fn is_terminal(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Completed
            | SessionStatus::Failed
            | SessionStatus::Disconnected
            | SessionStatus::Stopped
    )
}

/// Validate an allowed status transition for the vertical slice.
///
/// Control plane is authoritative; this encodes the documented happy/failure graph.
pub fn is_valid_transition(from: SessionStatus, to: SessionStatus) -> bool {
    if from == to {
        return true;
    }
    use SessionStatus::*;
    match from {
        Creating => matches!(
            to,
            StartingRuntime | Ready | Failed | Stopped | Disconnected
        ),
        StartingRuntime => matches!(to, Ready | Failed | Disconnected | Stopped),
        Ready => matches!(
            to,
            Running | Cancelling | Completed | Failed | Disconnected | Stopped
        ),
        Running => matches!(
            to,
            AwaitingApproval
                | Ready
                | Cancelling
                | Completed
                | Failed
                | Disconnected
                | Stopped
        ),
        AwaitingApproval => matches!(
            to,
            Running | Ready | Cancelling | Failed | Disconnected | Stopped | Completed
        ),
        Cancelling => matches!(to, Stopped | Failed | Disconnected | Ready | Completed),
        // Terminal states: only identity transition (handled above).
        Completed | Failed | Disconnected | Stopped => false,
    }
}

/// Error when a status transition is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid session status transition: {from} -> {to}")]
pub struct StatusTransitionError {
    /// Previous status.
    pub from: SessionStatus,
    /// Requested status.
    pub to: SessionStatus,
}

/// Attempt a transition; returns the new status or an error.
pub fn transition(
    from: SessionStatus,
    to: SessionStatus,
) -> Result<SessionStatus, StatusTransitionError> {
    if is_valid_transition(from, to) {
        Ok(to)
    } else {
        Err(StatusTransitionError { from, to })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_round_trip() {
        for s in SessionStatus::ALL {
            assert_eq!(SessionStatus::parse(s.as_str()), Some(*s));
            let json = serde_json::to_string(s).unwrap();
            let back: SessionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn happy_path_transitions() {
        assert!(is_valid_transition(
            SessionStatus::Creating,
            SessionStatus::StartingRuntime
        ));
        assert!(is_valid_transition(
            SessionStatus::StartingRuntime,
            SessionStatus::Ready
        ));
        assert!(is_valid_transition(
            SessionStatus::Ready,
            SessionStatus::Running
        ));
        assert!(is_valid_transition(
            SessionStatus::Running,
            SessionStatus::AwaitingApproval
        ));
        assert!(is_valid_transition(
            SessionStatus::AwaitingApproval,
            SessionStatus::Running
        ));
        assert!(is_valid_transition(
            SessionStatus::Running,
            SessionStatus::Completed
        ));
    }

    #[test]
    fn terminal_closed() {
        assert!(is_terminal(SessionStatus::Failed));
        assert!(!is_valid_transition(
            SessionStatus::Failed,
            SessionStatus::Ready
        ));
        assert!(!is_valid_transition(
            SessionStatus::Completed,
            SessionStatus::Running
        ));
    }
}