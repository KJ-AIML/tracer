//! Process-layer readiness vs adapter/session readiness.
//!
//! # Normative distinction (W1-C)
//!
//! | Layer | Meaning | Owner |
//! |---|---|---|
//! | **Process alive** | OS child running; stdin/stdout/stderr pipes available | process manager |
//! | **Protocol ready** | ACP `initialize` + capability negotiation succeeded; `runtime.process.ready` | ACP adapter |
//! | **Authenticated** | Runtime auth method completed when required | adapter / control plane |
//! | **Session ready** | `session/new` (or load) succeeded; prompts allowed | control plane |
//!
//! The process manager **must never** claim protocol, auth, or session readiness.
//! Callers that treat [`ProcessPhase::Alive`] as prompt-ready violate F-A05.

use crate::event::ExitInfo;

/// OS-level phase of a managed process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessPhase {
    /// Spawn is in progress (rarely observed outside the manager).
    Starting,
    /// Child is alive with pipes. Adapter may begin ACP handshake.
    ///
    /// **Not** protocol-ready, authenticated, or session-ready.
    Alive,
    /// Stop was requested; waiting for exit or force-kill.
    Stopping {
        /// Whether the stop was requested by the control plane.
        expected: bool,
    },
    /// Process has exited.
    Exited(ExitInfo),
    /// Terminal failure without a usable process handle.
    Failed,
}

impl ProcessPhase {
    /// `true` only while the OS process is believed running.
    pub fn is_alive(&self) -> bool {
        matches!(self, Self::Alive | Self::Stopping { .. })
    }

    /// Process-manager "running enough to hand pipes to an adapter".
    pub fn is_process_alive(&self) -> bool {
        matches!(self, Self::Alive)
    }
}

/// Explicit readiness view for control-plane composition.
///
/// All adapter/session flags stay `false` here — only process liveness is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessView {
    /// OS process is alive (pipes usable).
    pub process_alive: bool,
    /// Always `false` from the process manager. Adapter sets this after initialize.
    pub protocol_ready: bool,
    /// Always `false` from the process manager.
    pub authenticated: bool,
    /// Always `false` from the process manager.
    pub session_ready: bool,
}

impl ReadinessView {
    /// Build the process-only view from a phase.
    pub fn from_phase(phase: &ProcessPhase) -> Self {
        Self {
            process_alive: phase.is_process_alive(),
            protocol_ready: false,
            authenticated: false,
            session_ready: false,
        }
    }

    /// Prompts must not be accepted on process-alive alone.
    pub fn may_accept_prompt(&self) -> bool {
        self.process_alive && self.protocol_ready && self.session_ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alive_is_not_prompt_ready() {
        let view = ReadinessView::from_phase(&ProcessPhase::Alive);
        assert!(view.process_alive);
        assert!(!view.protocol_ready);
        assert!(!view.authenticated);
        assert!(!view.session_ready);
        assert!(!view.may_accept_prompt());
    }

    #[test]
    fn starting_not_alive() {
        let view = ReadinessView::from_phase(&ProcessPhase::Starting);
        assert!(!view.process_alive);
        assert!(!view.may_accept_prompt());
    }
}
