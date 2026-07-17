//! Tracer identifiers for the storage layer.
//!
//! Canonical primary keys (`EventId`, `ProjectId`, `SessionId`, `AgentRunId`)
//! are re-exported from [`tracer_domain`] so storage does not maintain a second
//! domain model (W1.1 integration / W1-B ownership).
//!
//! Storage-local IDs (`ProcessId`, `ApprovalId`, `ArtifactId`) remain here until
//! W1-B or a later domain expansion owns them.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

// --- Canonical domain IDs (W1-B) ---------------------------------------------

pub use tracer_domain::{AgentRunId, EventId, ProjectId, SessionId, TracerId};

// --- Storage-local IDs (not yet in tracer-domain) ----------------------------

macro_rules! storage_id {
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

storage_id!(
    /// Runtime process summary row (storage / control-plane identity).
    ProcessId
);
storage_id!(
    /// Approval decision audit row.
    ApprovalId
);
storage_id!(
    /// Basic artifact row.
    ArtifactId
);
