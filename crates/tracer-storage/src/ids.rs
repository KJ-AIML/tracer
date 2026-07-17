//! Stub Tracer domain identifiers (UUID strings).
//!
//! W1-B owns the canonical `tracer-domain` crate. Until that crate is integrated
//! into the workspace, storage uses these contract-compatible stubs so W1-E can
//! land independently (WAVE_1_READINESS: "stub IDs if B slightly lags").

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! tracer_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            pub fn as_str(&self) -> String {
                self.0.to_string()
            }

            pub fn parse(s: &str) -> Result<Self, uuid::Error> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }
    };
}

tracer_id!(
    /// Unique identifier for a normalized event instance.
    EventId
);
tracer_id!(
    /// Tracer project identifier.
    ProjectId
);
tracer_id!(
    /// Tracer session identifier.
    SessionId
);
tracer_id!(
    /// Active agent run within a session.
    AgentRunId
);
tracer_id!(
    /// Runtime process summary row.
    ProcessId
);
tracer_id!(
    /// Approval decision audit row.
    ApprovalId
);
tracer_id!(
    /// Basic artifact row.
    ArtifactId
);
