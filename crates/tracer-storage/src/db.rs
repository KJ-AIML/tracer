//! SQLite pool open, journal configuration, and migrations.
//!
//! ## Writer policy
//!
//! The **control plane** is the sole planned writer of the primary Tracer
//! database. Runtime sidecars and UI processes must not open this database
//! for writes (F-S05 / ADR-001).
//!
//! ## Crash safety
//!
//! WAL journal mode + synchronous=NORMAL (or FULL) and explicit transactions
//! provide crash-safe commits. Interrupted transactions roll back (F-S03).

use crate::error::{StorageError, StorageResult};
use crate::{SCHEMA_LOGICAL_VERSION, SCHEMA_LOGICAL_VERSION_NUM};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};

/// Type alias for the primary pool.
pub type DbPool = Pool<Sqlite>;

/// Open options for the primary database.
#[derive(Debug, Clone)]
pub struct OpenOptions {
    /// Maximum connections in the pool (keep small; single-writer product).
    pub max_connections: u32,
    /// Create the database file if missing.
    pub create_if_missing: bool,
    /// Run embedded migrations after open.
    pub run_migrations: bool,
    /// SQLite busy timeout.
    pub busy_timeout: Duration,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            max_connections: 5,
            create_if_missing: true,
            run_migrations: true,
            busy_timeout: Duration::from_secs(5),
        }
    }
}

/// Open (or create) the primary Tracer database at `db_path`.
///
/// Applies:
/// - `foreign_keys = ON`
/// - WAL journal mode
/// - `synchronous = NORMAL` (crash-safe with WAL)
/// - embedded migrations when `run_migrations` is true
pub async fn open_database(db_path: impl AsRef<Path>, opts: OpenOptions) -> StorageResult<DbPool> {
    let path = db_path.as_ref();

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Database {
                    message: format!("failed to create database directory: {e}"),
                    source: None,
                })?;
        }
    }

    let mut connect = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))
        .map_err(|e| StorageError::Database {
            message: format!("invalid database path: {e}"),
            source: Some(e),
        })?
        .create_if_missing(opts.create_if_missing)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(opts.busy_timeout);

    // sqlx 0.8: optimize settings are on connect options above.
    let _ = &mut connect;

    let pool = SqlitePoolOptions::new()
        .max_connections(opts.max_connections)
        .connect_with(connect)
        .await
        .map_err(|e| StorageError::Database {
            message: format!(
                "failed to open database at {} (corrupt or unreadable DB is refused — no silent reset): {e}",
                path.display()
            ),
            source: Some(e),
        })?;

    // Verify journal mode (defensive — options should already set WAL).
    verify_journal_mode(&pool).await?;

    // UF-01: refuse unsupported future schema before applying migrations.
    // Never silently reset or destructive-downgrade prior user data.
    if opts.run_migrations {
        refuse_unsupported_future_schema(&pool).await?;
        run_migrations(&pool).await?;
        ensure_schema_matches_crate(&pool).await?;
    }

    info!(path = %path.display(), "tracer storage database open");
    Ok(pool)
}

/// Open an in-memory database (tests).
pub async fn open_in_memory() -> StorageResult<DbPool> {
    let connect = SqliteConnectOptions::from_str("sqlite::memory:")
        .map_err(|e| StorageError::Database {
            message: format!("invalid in-memory URL: {e}"),
            source: Some(e),
        })?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(connect)
        .await
        .map_err(StorageError::from_sqlx)?;

    run_migrations(&pool).await?;
    ensure_schema_matches_crate(&pool).await?;
    Ok(pool)
}

/// Apply embedded SQL migrations. Safe to re-run (idempotent via sqlx migrate bookkeeping).
///
/// Each migration runs in a transaction (sqlx default). An interrupted migration
/// does not leave a partially-applied valid schema (UF-02).
pub async fn run_migrations(pool: &DbPool) -> StorageResult<()> {
    // Migrations are embedded from crates/tracer-storage/migrations at compile time.
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| StorageError::Migration {
            message: e.to_string(),
        })?;
    Ok(())
}

/// Parse a logical schema version string (`"1"`, `"2"`, …).
pub fn parse_schema_version(raw: &str) -> StorageResult<u32> {
    raw.trim()
        .parse::<u32>()
        .map_err(|_| StorageError::Migration {
            message: format!("unparseable schema_logical_version `{raw}`"),
        })
}

/// Controlled refusal when an existing DB advertises a newer logical schema
/// than this binary understands (UF-01). Does not mutate the database.
///
/// Fresh databases (no `storage_meta` yet) are allowed through so migrations
/// can create the schema.
pub async fn refuse_unsupported_future_schema(pool: &DbPool) -> StorageResult<()> {
    let raw = match get_meta(pool, "schema_logical_version").await {
        Ok(v) => v,
        Err(StorageError::Database { message, .. })
            if message.contains("no such table: storage_meta")
                || message.contains("no such table: main.storage_meta") =>
        {
            return Ok(());
        }
        Err(e) => {
            // Also tolerate missing-table surfaced via sqlx Database errors.
            let s = e.to_string();
            if s.contains("no such table") && s.contains("storage_meta") {
                return Ok(());
            }
            return Err(e);
        }
    };
    let Some(raw) = raw else {
        return Ok(());
    };
    let found = parse_schema_version(&raw)?;
    if found > SCHEMA_LOGICAL_VERSION_NUM {
        return Err(StorageError::Migration {
            message: format!(
                "unsupported future schema_logical_version={found} \
                 (binary supports {SCHEMA_LOGICAL_VERSION}); \
                 refusing start — no destructive downgrade or silent reset"
            ),
        });
    }
    Ok(())
}

async fn ensure_schema_matches_crate(pool: &DbPool) -> StorageResult<()> {
    let ver = schema_logical_version(pool).await?;
    if ver != SCHEMA_LOGICAL_VERSION {
        return Err(StorageError::Migration {
            message: format!(
                "schema_logical_version after migrations is `{ver}`, \
                 expected `{SCHEMA_LOGICAL_VERSION}`"
            ),
        });
    }
    Ok(())
}

/// Classify opening a newer-migrated DB with an older binary (UF-05).
///
/// Returns a stable token for release docs / upgrade harness:
/// `CONTROLLED_REFUSAL` when the older binary cannot safely open the DB.
pub async fn classify_downgrade_open(pool: &DbPool) -> StorageResult<&'static str> {
    refuse_unsupported_future_schema(pool).await?;
    Ok("SUPPORTED")
}

async fn verify_journal_mode(pool: &DbPool) -> StorageResult<()> {
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(pool)
        .await
        .map_err(StorageError::from_sqlx)?;
    let mode_l = mode.to_lowercase();
    if mode_l != "wal" && mode_l != "memory" {
        // memory DBs may report "memory"; file DBs must be WAL.
        warn!(journal_mode = %mode, "unexpected journal mode");
        return Err(StorageError::Database {
            message: format!("expected WAL journal mode, got `{mode}`"),
            source: None,
        });
    }
    Ok(())
}

/// Read a storage_meta value.
pub async fn get_meta(pool: &DbPool, key: &str) -> StorageResult<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM storage_meta WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(StorageError::from_sqlx)?;
    Ok(row.map(|r| r.0))
}

/// Logical schema version recorded by migration seed data.
pub async fn schema_logical_version(pool: &DbPool) -> StorageResult<String> {
    get_meta(pool, "schema_logical_version")
        .await?
        .ok_or_else(|| StorageError::Internal {
            message: "schema_logical_version missing after migrations".into(),
        })
}

/// Writer policy marker (must be `control_plane_only`).
pub async fn writer_policy(pool: &DbPool) -> StorageResult<String> {
    get_meta(pool, "writer_policy")
        .await?
        .ok_or_else(|| StorageError::Internal {
            message: "writer_policy missing after migrations".into(),
        })
}
