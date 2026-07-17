//! Presentation severity hints on event envelopes.

use serde::{Deserialize, Serialize};

/// Presentation hint for UI/timeline styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational (default).
    #[default]
    Info,
    /// Warning that does not necessarily fail the session.
    Warn,
    /// Error-class presentation.
    Error,
}

impl Severity {
    /// Wire string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_serde() {
        assert_eq!(serde_json::to_string(&Severity::Info).unwrap(), "\"info\"");
        assert_eq!(serde_json::to_string(&Severity::Warn).unwrap(), "\"warn\"");
        assert_eq!(
            serde_json::to_string(&Severity::Error).unwrap(),
            "\"error\""
        );
        let s: Severity = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(s, Severity::Error);
    }
}
