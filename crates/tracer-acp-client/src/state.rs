//! ACP session protocol state machine.
//!
//! Proves and enforces:
//! - process-ready ≠ authenticated
//! - process-ready ≠ session-ready
//! - session-ready ≠ prompt-complete
//!
//! Invalid transitions → [`TransitionError`] (typed) or controlled terminal phases.

use thiserror::Error;

/// Protocol-level phase for one runtime binding / ACP conversation.
///
/// Distinct from OS process phase (`tracer-process::ProcessPhase`) and from
/// control-plane [`tracer_domain::SessionStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolPhase {
    /// No process / transport available.
    ProcessUnavailable,
    /// Process is starting (spawn in flight).
    ProcessStarting,
    /// OS process alive; pipes open; ACP not yet initialized.
    ///
    /// **Not** authenticated. **Not** session-ready.
    ProcessAlive,
    /// `initialize` request in flight.
    Initializing,
    /// Initialize + capability negotiation completed (`runtime.process.ready` candidate).
    ///
    /// **Not** authenticated (unless auth not required). **Not** session-ready.
    ProtocolReady,
    /// Auth is required and not yet satisfied.
    AuthenticationRequired,
    /// Auth attempt failed.
    AuthenticationFailed,
    /// `session/new` or `session/load` in flight.
    CreatingSession,
    /// Runtime session exists; prompts may be submitted.
    SessionReady,
    /// Prompt submitted; waiting for stream/result.
    Prompting,
    /// Streaming agent updates.
    Streaming,
    /// Permission reverse-request pending.
    AwaitingApproval,
    /// Cancel in flight.
    Cancelling,
    /// Cancel completed (session may return to SessionReady).
    Cancelled,
    /// Prompt/run completed successfully.
    Completed,
    /// Terminal protocol/session failure.
    Failed,
    /// Transport disconnected / clean EOF.
    Disconnected,
    /// Runtime process crashed (unexpected exit).
    RuntimeCrashed,
}

impl std::fmt::Display for ProtocolPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ProtocolPhase {
    /// Wire-ish name for logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProcessUnavailable => "process_unavailable",
            Self::ProcessStarting => "process_starting",
            Self::ProcessAlive => "process_alive",
            Self::Initializing => "initializing",
            Self::ProtocolReady => "protocol_ready",
            Self::AuthenticationRequired => "authentication_required",
            Self::AuthenticationFailed => "authentication_failed",
            Self::CreatingSession => "creating_session",
            Self::SessionReady => "session_ready",
            Self::Prompting => "prompting",
            Self::Streaming => "streaming",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Cancelling => "cancelling",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Disconnected => "disconnected",
            Self::RuntimeCrashed => "runtime_crashed",
        }
    }

    /// Terminal failure/disconnect phases (no productive protocol ops until restart).
    ///
    /// `ProcessUnavailable` is an idle initial state, not a failure terminal.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Disconnected | Self::RuntimeCrashed
        )
    }

    /// Process is usable for ACP I/O (not necessarily protocol-ready).
    pub fn process_alive(self) -> bool {
        !matches!(
            self,
            Self::ProcessUnavailable
                | Self::ProcessStarting
                | Self::Disconnected
                | Self::RuntimeCrashed
                | Self::Failed
        )
    }

    /// ACP initialize + caps completed.
    pub fn is_protocol_ready(self) -> bool {
        matches!(
            self,
            Self::ProtocolReady
                | Self::AuthenticationRequired
                | Self::AuthenticationFailed
                | Self::CreatingSession
                | Self::SessionReady
                | Self::Prompting
                | Self::Streaming
                | Self::AwaitingApproval
                | Self::Cancelling
                | Self::Cancelled
                | Self::Completed
        )
    }

    /// Runtime session exists and can accept a new prompt (not mid-prompt).
    pub fn is_session_ready(self) -> bool {
        matches!(self, Self::SessionReady | Self::Completed | Self::Cancelled)
    }

    /// A prompt is actively running.
    pub fn is_prompt_active(self) -> bool {
        matches!(
            self,
            Self::Prompting | Self::Streaming | Self::AwaitingApproval | Self::Cancelling
        )
    }

    /// Auth has failed or is still required (not "authenticated").
    pub fn auth_blocks_session(self) -> bool {
        matches!(
            self,
            Self::AuthenticationRequired | Self::AuthenticationFailed
        )
    }
}

/// Invalid state transition.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid ACP protocol transition: {from} -> {to} ({reason})")]
pub struct TransitionError {
    /// Previous phase.
    pub from: ProtocolPhase,
    /// Requested phase.
    pub to: ProtocolPhase,
    /// Why rejected.
    pub reason: String,
}

impl TransitionError {
    fn new(from: ProtocolPhase, to: ProtocolPhase, reason: impl Into<String>) -> Self {
        Self {
            from,
            to,
            reason: reason.into(),
        }
    }
}

/// Whether a transition is allowed.
pub fn is_valid_transition(from: ProtocolPhase, to: ProtocolPhase) -> bool {
    if from == to {
        return true;
    }
    // Terminal recoveries only via explicit reset to ProcessUnavailable / Starting
    if from.is_terminal() {
        return matches!(
            to,
            ProtocolPhase::ProcessUnavailable | ProtocolPhase::ProcessStarting
        );
    }
    use ProtocolPhase::*;
    match from {
        ProcessUnavailable => matches!(to, ProcessStarting | ProcessAlive | Failed | Disconnected),
        ProcessStarting => matches!(
            to,
            ProcessAlive | Failed | Disconnected | RuntimeCrashed | ProcessUnavailable
        ),
        ProcessAlive => matches!(
            to,
            Initializing | Failed | Disconnected | RuntimeCrashed | ProcessUnavailable
        ),
        Initializing => matches!(
            to,
            ProtocolReady
                | Failed
                | Disconnected
                | RuntimeCrashed
                | ProcessUnavailable
                | AuthenticationRequired
        ),
        ProtocolReady => matches!(
            to,
            AuthenticationRequired
                | AuthenticationFailed
                | CreatingSession
                | SessionReady
                | Failed
                | Disconnected
                | RuntimeCrashed
        ),
        AuthenticationRequired => matches!(
            to,
            ProtocolReady
                | AuthenticationFailed
                | CreatingSession
                | Failed
                | Disconnected
                | RuntimeCrashed
        ),
        AuthenticationFailed => matches!(
            to,
            AuthenticationRequired
                | ProtocolReady
                | Failed
                | Disconnected
                | RuntimeCrashed
                | ProcessUnavailable
        ),
        CreatingSession => matches!(
            to,
            SessionReady
                | AuthenticationRequired
                | AuthenticationFailed
                | Failed
                | Disconnected
                | RuntimeCrashed
                | ProtocolReady
        ),
        SessionReady => matches!(
            to,
            Prompting
                | Cancelling
                | Completed
                | Failed
                | Disconnected
                | RuntimeCrashed
                | CreatingSession
        ),
        Prompting => matches!(
            to,
            Streaming
                | AwaitingApproval
                | Cancelling
                | Completed
                | Cancelled
                | Failed
                | Disconnected
                | RuntimeCrashed
                | SessionReady
        ),
        Streaming => matches!(
            to,
            AwaitingApproval
                | Cancelling
                | Completed
                | Cancelled
                | Failed
                | Disconnected
                | RuntimeCrashed
                | SessionReady
                | Prompting
        ),
        AwaitingApproval => matches!(
            to,
            Streaming
                | Prompting
                | Cancelling
                | Completed
                | Cancelled
                | Failed
                | Disconnected
                | RuntimeCrashed
                | SessionReady
        ),
        Cancelling => matches!(
            to,
            Cancelled | Completed | SessionReady | Failed | Disconnected | RuntimeCrashed
        ),
        Cancelled => matches!(
            to,
            SessionReady | Prompting | Failed | Disconnected | RuntimeCrashed | Completed
        ),
        Completed => matches!(
            to,
            SessionReady | Prompting | Failed | Disconnected | RuntimeCrashed | CreatingSession
        ),
        Failed | Disconnected | RuntimeCrashed => false,
    }
}

/// Attempt a transition.
pub fn transition(
    from: ProtocolPhase,
    to: ProtocolPhase,
) -> Result<ProtocolPhase, TransitionError> {
    if is_valid_transition(from, to) {
        Ok(to)
    } else {
        Err(TransitionError::new(
            from,
            to,
            "transition not in allowed graph",
        ))
    }
}

/// Mutable session protocol state with readiness proofs.
#[derive(Debug, Clone)]
pub struct SessionProtocolState {
    phase: ProtocolPhase,
    /// Process-layer alive (from process manager).
    process_alive: bool,
    /// Auth satisfied for session creation / prompts.
    authenticated: bool,
    /// Runtime session id when known.
    runtime_session_id: Option<String>,
    /// Last error message if failed.
    last_error: Option<String>,
}

impl Default for SessionProtocolState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionProtocolState {
    /// Fresh state: no process.
    pub fn new() -> Self {
        Self {
            phase: ProtocolPhase::ProcessUnavailable,
            process_alive: false,
            authenticated: false,
            runtime_session_id: None,
            last_error: None,
        }
    }

    /// Current phase.
    pub fn phase(&self) -> ProtocolPhase {
        self.phase
    }

    /// OS process alive (set by adapter from process manager).
    pub fn process_alive(&self) -> bool {
        self.process_alive
    }

    /// Protocol ready (initialize succeeded).
    pub fn protocol_ready(&self) -> bool {
        self.phase.is_protocol_ready()
    }

    /// Authenticated (or auth not required and session path open).
    pub fn authenticated(&self) -> bool {
        self.authenticated
    }

    /// Session ready for a new prompt.
    pub fn session_ready(&self) -> bool {
        self.phase.is_session_ready() && self.runtime_session_id.is_some()
    }

    /// Runtime session id.
    pub fn runtime_session_id(&self) -> Option<&str> {
        self.runtime_session_id.as_deref()
    }

    /// Last error.
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// May accept a user prompt (all three gates).
    pub fn may_accept_prompt(&self) -> bool {
        self.process_alive
            && self.protocol_ready()
            && self.session_ready()
            && !self.phase.is_prompt_active()
    }

    /// Apply a phase transition.
    pub fn set_phase(&mut self, to: ProtocolPhase) -> Result<(), TransitionError> {
        let next = transition(self.phase, to)?;
        self.phase = next;
        Ok(())
    }

    /// Force phase (for terminal events that must always land).
    pub fn force_phase(&mut self, to: ProtocolPhase) {
        self.phase = to;
    }

    /// Mark process starting.
    pub fn on_process_starting(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::ProcessStarting)?;
        self.process_alive = false;
        Ok(())
    }

    /// Mark process alive (still not protocol-ready / auth / session-ready).
    pub fn on_process_alive(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::ProcessAlive)?;
        self.process_alive = true;
        // Prove gates remain closed:
        debug_assert!(!self.protocol_ready() || self.phase == ProtocolPhase::ProcessAlive);
        Ok(())
    }

    /// Begin initialize.
    pub fn on_initialize_start(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::Initializing)
    }

    /// Initialize succeeded.
    pub fn on_initialize_ok(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::ProtocolReady)
    }

    /// Mark auth required (process may still be protocol-ready).
    pub fn on_auth_required(&mut self) -> Result<(), TransitionError> {
        self.authenticated = false;
        self.set_phase(ProtocolPhase::AuthenticationRequired)
    }

    /// Auth succeeded.
    pub fn on_authenticated(&mut self) {
        self.authenticated = true;
        // Stay in ProtocolReady if we were auth-required
        if self.phase == ProtocolPhase::AuthenticationRequired
            || self.phase == ProtocolPhase::AuthenticationFailed
        {
            let _ = self.set_phase(ProtocolPhase::ProtocolReady);
        }
    }

    /// Auth failed.
    pub fn on_auth_failed(&mut self, message: impl Into<String>) -> Result<(), TransitionError> {
        self.authenticated = false;
        self.last_error = Some(message.into());
        self.set_phase(ProtocolPhase::AuthenticationFailed)
    }

    /// Begin session/new.
    pub fn on_session_create_start(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::CreatingSession)
    }

    /// Session created.
    pub fn on_session_ready(
        &mut self,
        runtime_session_id: impl Into<String>,
    ) -> Result<(), TransitionError> {
        self.runtime_session_id = Some(runtime_session_id.into());
        self.authenticated = true;
        self.set_phase(ProtocolPhase::SessionReady)
    }

    /// Prompt submitted.
    pub fn on_prompt_start(&mut self) -> Result<(), TransitionError> {
        if !self.may_accept_prompt() && !self.phase.is_session_ready() {
            return Err(TransitionError::new(
                self.phase,
                ProtocolPhase::Prompting,
                "session not ready for prompt",
            ));
        }
        self.set_phase(ProtocolPhase::Prompting)
    }

    /// Streaming update observed.
    pub fn on_streaming(&mut self) -> Result<(), TransitionError> {
        if matches!(
            self.phase,
            ProtocolPhase::Prompting | ProtocolPhase::Streaming | ProtocolPhase::AwaitingApproval
        ) {
            if self.phase != ProtocolPhase::Streaming {
                self.set_phase(ProtocolPhase::Streaming)?;
            }
            Ok(())
        } else if self.phase.is_prompt_active() {
            Ok(())
        } else {
            Err(TransitionError::new(
                self.phase,
                ProtocolPhase::Streaming,
                "not in a prompt",
            ))
        }
    }

    /// Permission pending.
    pub fn on_awaiting_approval(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::AwaitingApproval)
    }

    /// Cancel started.
    pub fn on_cancel_start(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::Cancelling)
    }

    /// Cancel done — return to session ready.
    pub fn on_cancelled(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::Cancelled)?;
        // Reusable session
        let _ = self.set_phase(ProtocolPhase::SessionReady);
        Ok(())
    }

    /// Prompt completed.
    pub fn on_prompt_completed(&mut self) -> Result<(), TransitionError> {
        self.set_phase(ProtocolPhase::Completed)?;
        let _ = self.set_phase(ProtocolPhase::SessionReady);
        Ok(())
    }

    /// Protocol/session failure.
    pub fn on_failed(&mut self, message: impl Into<String>) {
        self.last_error = Some(message.into());
        self.force_phase(ProtocolPhase::Failed);
    }

    /// Disconnect / EOF.
    pub fn on_disconnected(&mut self, _expected: bool) {
        self.process_alive = false;
        self.force_phase(ProtocolPhase::Disconnected);
    }

    /// Crash.
    pub fn on_crashed(&mut self, message: impl Into<String>) {
        self.process_alive = false;
        self.last_error = Some(message.into());
        self.force_phase(ProtocolPhase::RuntimeCrashed);
    }

    /// Reset for a fresh process restart.
    pub fn reset_for_restart(&mut self) {
        self.phase = ProtocolPhase::ProcessUnavailable;
        self.process_alive = false;
        self.authenticated = false;
        self.runtime_session_id = None;
        self.last_error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_ready_not_authenticated_or_session_ready() {
        let mut s = SessionProtocolState::new();
        s.on_process_starting().unwrap();
        s.on_process_alive().unwrap();
        s.on_initialize_start().unwrap();
        s.on_initialize_ok().unwrap();

        assert!(s.process_alive());
        assert!(s.protocol_ready());
        assert!(!s.authenticated());
        assert!(!s.session_ready());
        assert!(!s.may_accept_prompt());
    }

    #[test]
    fn session_ready_not_prompt_complete() {
        let mut s = SessionProtocolState::new();
        s.on_process_starting().unwrap();
        s.on_process_alive().unwrap();
        s.on_initialize_start().unwrap();
        s.on_initialize_ok().unwrap();
        s.on_authenticated();
        s.on_session_create_start().unwrap();
        s.on_session_ready("rt-1").unwrap();

        assert!(s.session_ready());
        assert!(s.may_accept_prompt());
        assert!(!s.phase.is_prompt_active());

        s.on_prompt_start().unwrap();
        assert!(s.phase.is_prompt_active());
        assert!(!s.may_accept_prompt());
        // prompt not complete until on_prompt_completed
        assert_ne!(s.phase(), ProtocolPhase::Completed);
    }

    #[test]
    fn invalid_transition_errors() {
        let mut s = SessionProtocolState::new();
        let err = s.on_prompt_start().unwrap_err();
        assert_eq!(err.from, ProtocolPhase::ProcessUnavailable);
    }

    #[test]
    fn auth_required_blocks_session_ready() {
        let mut s = SessionProtocolState::new();
        s.on_process_alive().unwrap();
        s.on_initialize_start().unwrap();
        s.on_initialize_ok().unwrap();
        s.on_auth_required().unwrap();
        assert!(s.protocol_ready());
        assert!(!s.authenticated());
        assert!(!s.session_ready());
        assert!(!s.may_accept_prompt());
    }
}
