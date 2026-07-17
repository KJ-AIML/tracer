//! Stable Tracer-owned identifiers (UUID string form).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Marker trait for Tracer primary-key identifiers.
pub trait TracerId: Sized + fmt::Display {
    /// Create a new random v4 UUID identifier.
    fn new() -> Self;
    /// Underlying UUID.
    fn as_uuid(&self) -> Uuid;
    /// Canonical lowercase hyphenated string form.
    fn as_str(&self) -> String {
        self.as_uuid().to_string()
    }
}

macro_rules! tracer_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl TracerId for $name {
            fn new() -> Self {
                Self(Uuid::new_v4())
            }
            fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl $name {
            /// Create a new random v4 UUID identifier.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Construct from a UUID.
            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            /// Parse from a UUID string.
            pub fn parse(s: &str) -> Result<Self, uuid::Error> {
                Ok(Self(Uuid::parse_str(s)?))
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
    /// Active agent run within a session (optional on some events).
    AgentRunId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_round_trip_json() {
        let id = EventId::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: EventId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
        // transparent: JSON is a bare string
        assert!(json.starts_with('"'));
    }
}