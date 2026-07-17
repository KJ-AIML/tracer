//! Adapter readiness: process + protocol + auth + session (distinct gates).

use tracer_acp_client::ProtocolPhase;
use tracer_domain::AuthenticationState;

/// Composed readiness view for control plane / W1-F.
///
/// # Normative proofs
///
/// - `process_alive` alone is **never** enough for prompts
/// - `protocol_ready` (initialize) ≠ `authenticated`
/// - `protocol_ready` ≠ `session_ready`
/// - `session_ready` ≠ prompt complete (`prompt_active`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterReadiness {
    /// OS process alive.
    pub process_alive: bool,
    /// ACP initialize + caps done.
    pub protocol_ready: bool,
    /// Auth product state.
    pub auth_state: AuthenticationState,
    /// Runtime session exists and may accept a new prompt.
    pub session_ready: bool,
    /// Prompt currently in flight.
    pub prompt_active: bool,
    /// Protocol phase string.
    pub protocol_phase: &'static str,
}

impl AdapterReadiness {
    /// Build from components.
    pub fn new(
        process_alive: bool,
        phase: ProtocolPhase,
        auth_state: AuthenticationState,
        session_ready: bool,
    ) -> Self {
        Self {
            process_alive,
            protocol_ready: phase.is_protocol_ready(),
            auth_state,
            session_ready,
            prompt_active: phase.is_prompt_active(),
            protocol_phase: phase.as_str(),
        }
    }

    /// All gates for submitting a prompt.
    pub fn may_accept_prompt(&self) -> bool {
        self.process_alive
            && self.protocol_ready
            && self.session_ready
            && !self.prompt_active
            && self.auth_state.allows_prompt()
    }
}
