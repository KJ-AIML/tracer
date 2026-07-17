//! Process identity types.

use std::fmt;

use uuid::Uuid;

/// Tracer-side id for one managed process binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessId(Uuid);

impl ProcessId {
    /// Generate a new random process id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wrap an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Borrow the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Hyphenated string form.
    pub fn to_string_hyphenated(&self) -> String {
        self.0.to_string()
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
