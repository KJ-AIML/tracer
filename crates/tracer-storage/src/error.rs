//! Storage error types mapped to the Tauri command surface `errorClass` values.
//!
//! See `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md` (`StorageError`, `NotFound`,
//! `AlreadyExists`, `InvalidArgument`).

use thiserror::Error;

/// Stable error class strings for command / control-plane mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageErrorClass {
    /// Generic durable persistence failure (disk full, IO, SQLite error).
    StorageError,
    /// Requested entity does not exist.
    NotFound,
    /// Unique constraint / duplicate identity.
    AlreadyExists,
    /// Caller supplied invalid arguments or violated domain constraints.
    InvalidArgument,
    /// Schema migration failed; app must refuse unsafe start (F-S02).
    MigrationFailed,
    /// Internal invariant broken.
    InternalError,
}

impl StorageErrorClass {
    /// Contract `errorClass` string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StorageError => "StorageError",
            Self::NotFound => "NotFound",
            Self::AlreadyExists => "AlreadyExists",
            Self::InvalidArgument => "InvalidArgument",
            Self::MigrationFailed => "StorageError",
            Self::InternalError => "InternalError",
        }
    }

    /// Whether a retry might succeed without code/schema changes.
    pub fn retryable(self) -> bool {
        matches!(self, Self::StorageError)
    }
}

/// Error returned by storage repository and database operations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage error: {message}")]
    Database {
        message: String,
        #[source]
        source: Option<sqlx::Error>,
    },

    #[error("migration failed: {message}")]
    Migration { message: String },

    #[error("not found: {entity} `{id}`")]
    NotFound { entity: &'static str, id: String },

    #[error("already exists: {entity} `{id}`")]
    AlreadyExists { entity: &'static str, id: String },

    #[error("invalid argument: {message}")]
    InvalidArgument { message: String },

    #[error("sequence conflict for session `{session_id}`: expected {expected}, got {got}")]
    SequenceConflict {
        session_id: String,
        expected: i64,
        got: i64,
    },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("internal storage error: {message}")]
    Internal { message: String },
}

impl StorageError {
    /// Map to the stable command-surface error class.
    pub fn error_class(&self) -> StorageErrorClass {
        match self {
            Self::Database { .. } => StorageErrorClass::StorageError,
            Self::Migration { .. } => StorageErrorClass::MigrationFailed,
            Self::NotFound { .. } => StorageErrorClass::NotFound,
            Self::AlreadyExists { .. } => StorageErrorClass::AlreadyExists,
            Self::InvalidArgument { .. } => StorageErrorClass::InvalidArgument,
            Self::SequenceConflict { .. } => StorageErrorClass::InvalidArgument,
            Self::Serialization(_) => StorageErrorClass::InvalidArgument,
            Self::Internal { .. } => StorageErrorClass::InternalError,
        }
    }

    pub fn retryable(&self) -> bool {
        self.error_class().retryable()
    }

    pub fn from_sqlx(err: sqlx::Error) -> Self {
        // SQLite unique / constraint violations → AlreadyExists when possible.
        if let sqlx::Error::Database(ref db) = err {
            let code = db.code().map(|c| c.to_string()).unwrap_or_default();
            let msg = db.message().to_string();
            // SQLite: 1555 SQLITE_CONSTRAINT_PRIMARYKEY, 2067 UNIQUE, 787 FK, etc.
            if code == "1555" || code == "2067" || msg.contains("UNIQUE constraint failed") {
                return Self::AlreadyExists {
                    entity: "record",
                    id: msg,
                };
            }
        }
        Self::Database {
            message: err.to_string(),
            source: Some(err),
        }
    }

    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }

    pub fn already_exists(entity: &'static str, id: impl Into<String>) -> Self {
        Self::AlreadyExists {
            entity,
            id: id.into(),
        }
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: message.into(),
        }
    }
}

/// Result alias for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;
