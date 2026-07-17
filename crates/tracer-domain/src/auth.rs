//! Authentication product states (orthogonal to session status).

use serde::{Deserialize, Serialize};

/// Product-level authentication state for a runtime/session binding.
///
/// Distinguishes process-ready from authenticated session-ready (Stage 0.1 / UX matrix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationState {
    /// Runtime does not require sign-in (e.g. fake ACP).
    #[default]
    NotRequired,
    /// Auth is required and has not completed.
    Unauthenticated,
    /// Sign-in is in progress.
    InProgress,
    /// Authenticated successfully.
    Authenticated,
    /// Last auth attempt failed.
    Failed,
    /// Prior authentication expired mid-session.
    Expired,
}

impl AuthenticationState {
    /// All catalog values.
    pub const ALL: &'static [AuthenticationState] = &[
        Self::NotRequired,
        Self::Unauthenticated,
        Self::InProgress,
        Self::Authenticated,
        Self::Failed,
        Self::Expired,
    ];

    /// Wire string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotRequired => "not_required",
            Self::Unauthenticated => "unauthenticated",
            Self::InProgress => "in_progress",
            Self::Authenticated => "authenticated",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    /// Whether the user may submit prompts with respect to auth only.
    ///
    /// Session status must still be `ready` (or equivalent) independently.
    pub fn allows_prompt(self) -> bool {
        matches!(self, Self::NotRequired | Self::Authenticated)
    }

    /// Whether an auth banner / setup UI should be shown.
    pub fn requires_user_action(self) -> bool {
        matches!(
            self,
            Self::Unauthenticated | Self::Failed | Self::Expired | Self::InProgress
        )
    }
}

impl std::fmt::Display for AuthenticationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_prompt_gates() {
        assert!(AuthenticationState::NotRequired.allows_prompt());
        assert!(AuthenticationState::Authenticated.allows_prompt());
        assert!(!AuthenticationState::Unauthenticated.allows_prompt());
        assert!(!AuthenticationState::Failed.allows_prompt());
        assert!(!AuthenticationState::Expired.allows_prompt());
        assert!(!AuthenticationState::InProgress.allows_prompt());
    }

    #[test]
    fn auth_serde() {
        for s in AuthenticationState::ALL {
            let json = serde_json::to_string(s).unwrap();
            let back: AuthenticationState = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }
}