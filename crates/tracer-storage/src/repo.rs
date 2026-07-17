//! Repository interfaces and SQLite implementations.
//!
//! Other modules must depend on these APIs rather than embedding SQL.
//! Intended sole writer: **control plane** (see crate-level docs).

use crate::db::DbPool;
use crate::error::{StorageError, StorageResult};
use crate::ids::{
    AgentRunId, ApprovalId, ArtifactId, EventId, ProcessId, ProjectId, SessionId, TracerId,
};
use crate::models::{
    ApprovalDecision, ApprovalDecisionRecord, ArtifactRecord, EventList, EventRecord,
    ProjectRecord, ProjectStatus, ReconcileReport, RuntimeProcessRecord, RuntimeProcessStatus,
    SessionRecord, SessionStatus, SessionStatusStorageExt, Severity, SeverityStorageExt,
};
use crate::timeutil::now_rfc3339;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::{Sqlite, Transaction};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Traits (repository interfaces)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn insert(&self, project: &ProjectRecord) -> StorageResult<()>;
    async fn get(&self, project_id: &ProjectId) -> StorageResult<ProjectRecord>;
    async fn get_by_root_path(&self, root_path: &str) -> StorageResult<Option<ProjectRecord>>;
    async fn list(&self) -> StorageResult<Vec<ProjectRecord>>;
    async fn update(&self, project: &ProjectRecord) -> StorageResult<()>;
    async fn remove(&self, project_id: &ProjectId, delete_history: bool) -> StorageResult<()>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn insert(&self, session: &SessionRecord) -> StorageResult<()>;
    async fn get(&self, session_id: &SessionId) -> StorageResult<SessionRecord>;
    async fn list_by_project(
        &self,
        project_id: &ProjectId,
        limit: i64,
    ) -> StorageResult<Vec<SessionRecord>>;
    async fn update_status(
        &self,
        session_id: &SessionId,
        status: SessionStatus,
    ) -> StorageResult<()>;
    async fn update(&self, session: &SessionRecord) -> StorageResult<()>;
    /// Mark sessions that still look "live" as `disconnected` after restart (F-S04).
    async fn reconcile_stale_live_sessions(
        &self,
        target: SessionStatus,
    ) -> StorageResult<ReconcileReport>;
}

#[async_trait]
pub trait EventRepository: Send + Sync {
    /// Insert a single event. Sequence must equal the session's `next_sequence`
    /// (or use [`SqliteStorage::append_event`] which assigns it).
    async fn insert(&self, event: &EventRecord) -> StorageResult<()>;
    /// List events for a session ordered by ascending `sequence`.
    async fn list(
        &self,
        session_id: &SessionId,
        after_sequence: i64,
        limit: i64,
    ) -> StorageResult<EventList>;
    async fn get(&self, session_id: &SessionId, event_id: &EventId) -> StorageResult<EventRecord>;
}

#[async_trait]
pub trait RuntimeProcessRepository: Send + Sync {
    async fn insert(&self, process: &RuntimeProcessRecord) -> StorageResult<()>;
    async fn update(&self, process: &RuntimeProcessRecord) -> StorageResult<()>;
    async fn list_by_session(
        &self,
        session_id: &SessionId,
    ) -> StorageResult<Vec<RuntimeProcessRecord>>;
}

#[async_trait]
pub trait ApprovalRepository: Send + Sync {
    async fn insert(&self, decision: &ApprovalDecisionRecord) -> StorageResult<()>;
    async fn list_by_session(
        &self,
        session_id: &SessionId,
    ) -> StorageResult<Vec<ApprovalDecisionRecord>>;
}

#[async_trait]
pub trait ArtifactRepository: Send + Sync {
    async fn insert(&self, artifact: &ArtifactRecord) -> StorageResult<()>;
    async fn list_by_session(&self, session_id: &SessionId) -> StorageResult<Vec<ArtifactRecord>>;
}

// ---------------------------------------------------------------------------
// SQLite facade
// ---------------------------------------------------------------------------

/// Primary storage handle used by the control plane.
///
/// # Writer policy
///
/// Only the control plane should construct and use [`SqliteStorage`] against
/// the primary database file. Runtime processes must not write here.
#[derive(Clone)]
pub struct SqliteStorage {
    pool: DbPool,
}

impl SqliteStorage {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Begin an explicit transaction for multi-step durable writes.
    pub async fn begin(&self) -> StorageResult<Transaction<'_, Sqlite>> {
        self.pool.begin().await.map_err(StorageError::from_sqlx)
    }

    // --- Inherent wrappers (avoid trait method ambiguity at call sites) ---

    /// Insert a project row.
    pub async fn insert_project(&self, project: &ProjectRecord) -> StorageResult<()> {
        ProjectRepository::insert(self, project).await
    }

    /// Get a project by id.
    pub async fn get_project(&self, project_id: &ProjectId) -> StorageResult<ProjectRecord> {
        ProjectRepository::get(self, project_id).await
    }

    /// List all projects.
    pub async fn list_projects(&self) -> StorageResult<Vec<ProjectRecord>> {
        ProjectRepository::list(self).await
    }

    /// Insert a session row.
    pub async fn insert_session(&self, session: &SessionRecord) -> StorageResult<()> {
        SessionRepository::insert(self, session).await
    }

    /// Get a session by id.
    pub async fn get_session(&self, session_id: &SessionId) -> StorageResult<SessionRecord> {
        SessionRepository::get(self, session_id).await
    }

    /// List sessions for a project.
    pub async fn list_sessions(
        &self,
        project_id: &ProjectId,
        limit: i64,
    ) -> StorageResult<Vec<SessionRecord>> {
        SessionRepository::list_by_project(self, project_id, limit).await
    }

    /// Update session status.
    pub async fn update_session_status(
        &self,
        session_id: &SessionId,
        status: SessionStatus,
    ) -> StorageResult<()> {
        SessionRepository::update_status(self, session_id, status).await
    }

    /// Reconcile stale live session statuses after restart (F-S04).
    pub async fn reconcile_stale_live_sessions(
        &self,
        target: SessionStatus,
    ) -> StorageResult<ReconcileReport> {
        SessionRepository::reconcile_stale_live_sessions(self, target).await
    }

    /// List events for a session ordered by ascending sequence.
    pub async fn list_events(
        &self,
        session_id: &SessionId,
        after_sequence: i64,
        limit: i64,
    ) -> StorageResult<EventList> {
        EventRepository::list(self, session_id, after_sequence, limit).await
    }

    /// Get a single event by id within a session.
    pub async fn get_event(
        &self,
        session_id: &SessionId,
        event_id: &EventId,
    ) -> StorageResult<EventRecord> {
        EventRepository::get(self, session_id, event_id).await
    }

    /// Insert a runtime process summary.
    pub async fn insert_runtime_process(
        &self,
        process: &RuntimeProcessRecord,
    ) -> StorageResult<()> {
        RuntimeProcessRepository::insert(self, process).await
    }

    /// Insert an approval decision.
    pub async fn insert_approval(&self, decision: &ApprovalDecisionRecord) -> StorageResult<()> {
        ApprovalRepository::insert(self, decision).await
    }

    /// Insert a basic artifact.
    pub async fn insert_artifact(&self, artifact: &ArtifactRecord) -> StorageResult<()> {
        ArtifactRepository::insert(self, artifact).await
    }

    /// Append an event, assigning the next monotonic sequence for the session
    /// inside a single transaction (crash-safe: all-or-nothing).
    ///
    /// Returns the event with `sequence` filled in.
    pub async fn append_event(&self, mut event: EventRecord) -> StorageResult<EventRecord> {
        let mut tx = self.begin().await?;

        let session_id = event.session_id.as_str();
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT next_sequence FROM sessions WHERE session_id = ?1")
                .bind(&session_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(StorageError::from_sqlx)?;

        let next = match row {
            Some((n,)) => n,
            None => {
                return Err(StorageError::not_found("session", session_id));
            }
        };

        // If caller already set sequence, it must match.
        if event.sequence > 0 && event.sequence != next {
            return Err(StorageError::SequenceConflict {
                session_id: session_id.clone(),
                expected: next,
                got: event.sequence,
            });
        }
        event.sequence = next;

        insert_event_tx(&mut tx, &event).await?;

        let updated_at = now_rfc3339();
        sqlx::query(
            "UPDATE sessions SET next_sequence = ?1, updated_at = ?2 WHERE session_id = ?3",
        )
        .bind(next + 1)
        .bind(&updated_at)
        .bind(&session_id)
        .execute(&mut *tx)
        .await
        .map_err(StorageError::from_sqlx)?;

        tx.commit().await.map_err(StorageError::from_sqlx)?;
        Ok(event)
    }

    /// Append multiple events in one transaction with contiguous sequences.
    pub async fn append_events(
        &self,
        mut events: Vec<EventRecord>,
    ) -> StorageResult<Vec<EventRecord>> {
        if events.is_empty() {
            return Ok(events);
        }

        // All events must share a session.
        let session_id = events[0].session_id;
        if events.iter().any(|e| e.session_id != session_id) {
            return Err(StorageError::invalid_argument(
                "append_events requires a single sessionId",
            ));
        }

        let mut tx = self.begin().await?;
        let sid = session_id.as_str();
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT next_sequence FROM sessions WHERE session_id = ?1")
                .bind(&sid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(StorageError::from_sqlx)?;

        let mut next = match row {
            Some((n,)) => n,
            None => return Err(StorageError::not_found("session", sid)),
        };

        for event in &mut events {
            event.sequence = next;
            insert_event_tx(&mut tx, event).await?;
            next += 1;
        }

        let updated_at = now_rfc3339();
        sqlx::query(
            "UPDATE sessions SET next_sequence = ?1, updated_at = ?2 WHERE session_id = ?3",
        )
        .bind(next)
        .bind(&updated_at)
        .bind(&sid)
        .execute(&mut *tx)
        .await
        .map_err(StorageError::from_sqlx)?;

        tx.commit().await.map_err(StorageError::from_sqlx)?;
        Ok(events)
    }
}

// ---------------------------------------------------------------------------
// Project repo
// ---------------------------------------------------------------------------

#[async_trait]
impl ProjectRepository for SqliteStorage {
    async fn insert(&self, project: &ProjectRecord) -> StorageResult<()> {
        let res = sqlx::query(
            r#"
            INSERT INTO projects (
                project_id, name, root_path, status, is_git,
                last_opened_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(project.project_id.as_str())
        .bind(&project.name)
        .bind(&project.root_path)
        .bind(project.status.as_str())
        .bind(if project.is_git { 1 } else { 0 })
        .bind(&project.last_opened_at)
        .bind(&project.created_at)
        .bind(&project.updated_at)
        .execute(&self.pool)
        .await;

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                let mapped = StorageError::from_sqlx(e);
                if matches!(mapped, StorageError::AlreadyExists { .. }) {
                    Err(StorageError::already_exists(
                        "project",
                        project.project_id.as_str(),
                    ))
                } else {
                    Err(mapped)
                }
            }
        }
    }

    async fn get(&self, project_id: &ProjectId) -> StorageResult<ProjectRecord> {
        let row = sqlx::query_as::<_, ProjectRow>(
            r#"
            SELECT project_id, name, root_path, status, is_git,
                   last_opened_at, created_at, updated_at
            FROM projects WHERE project_id = ?1
            "#,
        )
        .bind(project_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        match row {
            Some(r) => r.into_record(),
            None => Err(StorageError::not_found("project", project_id.as_str())),
        }
    }

    async fn get_by_root_path(&self, root_path: &str) -> StorageResult<Option<ProjectRecord>> {
        let row = sqlx::query_as::<_, ProjectRow>(
            r#"
            SELECT project_id, name, root_path, status, is_git,
                   last_opened_at, created_at, updated_at
            FROM projects WHERE root_path = ?1
            "#,
        )
        .bind(root_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        match row {
            Some(r) => Ok(Some(r.into_record()?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> StorageResult<Vec<ProjectRecord>> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            r#"
            SELECT project_id, name, root_path, status, is_git,
                   last_opened_at, created_at, updated_at
            FROM projects
            ORDER BY COALESCE(last_opened_at, created_at) DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        rows.into_iter().map(|r| r.into_record()).collect()
    }

    async fn update(&self, project: &ProjectRecord) -> StorageResult<()> {
        let res = sqlx::query(
            r#"
            UPDATE projects SET
                name = ?2, root_path = ?3, status = ?4, is_git = ?5,
                last_opened_at = ?6, updated_at = ?7
            WHERE project_id = ?1
            "#,
        )
        .bind(project.project_id.as_str())
        .bind(&project.name)
        .bind(&project.root_path)
        .bind(project.status.as_str())
        .bind(if project.is_git { 1 } else { 0 })
        .bind(&project.last_opened_at)
        .bind(&project.updated_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        if res.rows_affected() == 0 {
            return Err(StorageError::not_found(
                "project",
                project.project_id.as_str(),
            ));
        }
        Ok(())
    }

    async fn remove(&self, project_id: &ProjectId, delete_history: bool) -> StorageResult<()> {
        if !delete_history {
            // Keep sessions/events only if we also keep the project row; without
            // history deletion we still remove the project registration but
            // CASCADE would wipe children. For non-history-delete, reject if
            // sessions exist so control plane can choose explicitly.
            let count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE project_id = ?1")
                    .bind(project_id.as_str())
                    .fetch_one(&self.pool)
                    .await
                    .map_err(StorageError::from_sqlx)?;
            if count.0 > 0 {
                return Err(StorageError::invalid_argument(
                    "project has sessions; pass delete_history=true to remove registration and history",
                ));
            }
        }

        let res = sqlx::query("DELETE FROM projects WHERE project_id = ?1")
            .bind(project_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(StorageError::from_sqlx)?;

        if res.rows_affected() == 0 {
            return Err(StorageError::not_found("project", project_id.as_str()));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Session repo
// ---------------------------------------------------------------------------

#[async_trait]
impl SessionRepository for SqliteStorage {
    async fn insert(&self, session: &SessionRecord) -> StorageResult<()> {
        let caps = session
            .capabilities
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let last_err = session
            .last_error
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let agent = session.active_agent_run_id.as_ref().map(|id| id.as_str());

        let res = sqlx::query(
            r#"
            INSERT INTO sessions (
                session_id, project_id, title, status, runtime_kind,
                runtime_session_id, capabilities_json, last_error_json,
                active_agent_run_id, next_sequence, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )
        .bind(session.session_id.as_str())
        .bind(session.project_id.as_str())
        .bind(&session.title)
        .bind(session.status.as_str())
        .bind(&session.runtime_kind)
        .bind(&session.runtime_session_id)
        .bind(caps)
        .bind(last_err)
        .bind(agent)
        .bind(session.next_sequence)
        .bind(&session.created_at)
        .bind(&session.updated_at)
        .execute(&self.pool)
        .await;

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                let mapped = StorageError::from_sqlx(e);
                if matches!(mapped, StorageError::AlreadyExists { .. }) {
                    Err(StorageError::already_exists(
                        "session",
                        session.session_id.as_str(),
                    ))
                } else {
                    Err(mapped)
                }
            }
        }
    }

    async fn get(&self, session_id: &SessionId) -> StorageResult<SessionRecord> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT session_id, project_id, title, status, runtime_kind,
                   runtime_session_id, capabilities_json, last_error_json,
                   active_agent_run_id, next_sequence, created_at, updated_at
            FROM sessions WHERE session_id = ?1
            "#,
        )
        .bind(session_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        match row {
            Some(r) => r.into_record(),
            None => Err(StorageError::not_found("session", session_id.as_str())),
        }
    }

    async fn list_by_project(
        &self,
        project_id: &ProjectId,
        limit: i64,
    ) -> StorageResult<Vec<SessionRecord>> {
        let limit = if limit <= 0 { 50 } else { limit };
        let rows = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT session_id, project_id, title, status, runtime_kind,
                   runtime_session_id, capabilities_json, last_error_json,
                   active_agent_run_id, next_sequence, created_at, updated_at
            FROM sessions
            WHERE project_id = ?1
            ORDER BY updated_at DESC
            LIMIT ?2
            "#,
        )
        .bind(project_id.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        rows.into_iter().map(|r| r.into_record()).collect()
    }

    async fn update_status(
        &self,
        session_id: &SessionId,
        status: SessionStatus,
    ) -> StorageResult<()> {
        let updated_at = now_rfc3339();
        let res =
            sqlx::query("UPDATE sessions SET status = ?2, updated_at = ?3 WHERE session_id = ?1")
                .bind(session_id.as_str())
                .bind(status.as_str())
                .bind(&updated_at)
                .execute(&self.pool)
                .await
                .map_err(StorageError::from_sqlx)?;

        if res.rows_affected() == 0 {
            return Err(StorageError::not_found("session", session_id.as_str()));
        }
        Ok(())
    }

    async fn update(&self, session: &SessionRecord) -> StorageResult<()> {
        let caps = session
            .capabilities
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let last_err = session
            .last_error
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let agent = session.active_agent_run_id.as_ref().map(|id| id.as_str());

        let res = sqlx::query(
            r#"
            UPDATE sessions SET
                project_id = ?2, title = ?3, status = ?4, runtime_kind = ?5,
                runtime_session_id = ?6, capabilities_json = ?7, last_error_json = ?8,
                active_agent_run_id = ?9, next_sequence = ?10, updated_at = ?11
            WHERE session_id = ?1
            "#,
        )
        .bind(session.session_id.as_str())
        .bind(session.project_id.as_str())
        .bind(&session.title)
        .bind(session.status.as_str())
        .bind(&session.runtime_kind)
        .bind(&session.runtime_session_id)
        .bind(caps)
        .bind(last_err)
        .bind(agent)
        .bind(session.next_sequence)
        .bind(&session.updated_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        if res.rows_affected() == 0 {
            return Err(StorageError::not_found(
                "session",
                session.session_id.as_str(),
            ));
        }
        Ok(())
    }

    async fn reconcile_stale_live_sessions(
        &self,
        target: SessionStatus,
    ) -> StorageResult<ReconcileReport> {
        if target.implies_live_process() {
            return Err(StorageError::invalid_argument(
                "reconcile target status must be terminal (disconnected/stopped/failed)",
            ));
        }

        let live_statuses = [
            SessionStatus::Creating,
            SessionStatus::StartingRuntime,
            SessionStatus::Ready,
            SessionStatus::Running,
            SessionStatus::AwaitingApproval,
            SessionStatus::Cancelling,
        ];

        let mut examined = 0usize;
        let mut updated = Vec::new();
        let updated_at = now_rfc3339();

        for status in live_statuses {
            let rows: Vec<(String,)> =
                sqlx::query_as("SELECT session_id FROM sessions WHERE status = ?1")
                    .bind(status.as_str())
                    .fetch_all(&self.pool)
                    .await
                    .map_err(StorageError::from_sqlx)?;

            for (sid,) in rows {
                examined += 1;
                let res = sqlx::query(
                    "UPDATE sessions SET status = ?2, updated_at = ?3 WHERE session_id = ?1",
                )
                .bind(&sid)
                .bind(target.as_str())
                .bind(&updated_at)
                .execute(&self.pool)
                .await
                .map_err(StorageError::from_sqlx)?;

                if res.rows_affected() > 0 {
                    updated.push(SessionId::parse(&sid).map_err(|e| StorageError::Internal {
                        message: format!("invalid session_id in db: {e}"),
                    })?);
                }
            }
        }

        Ok(ReconcileReport {
            sessions_examined: examined,
            sessions_updated: updated,
            target_status: target,
        })
    }
}

// ---------------------------------------------------------------------------
// Event repo
// ---------------------------------------------------------------------------

#[async_trait]
impl EventRepository for SqliteStorage {
    async fn insert(&self, event: &EventRecord) -> StorageResult<()> {
        if event.sequence < 1 {
            return Err(StorageError::invalid_argument(
                "sequence must be >= 1 (prefer append_event for assignment)",
            ));
        }
        let mut tx = self.begin().await?;
        insert_event_tx(&mut tx, event).await?;

        // Advance next_sequence if this fills the expected slot.
        let sid = event.session_id.as_str();
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT next_sequence FROM sessions WHERE session_id = ?1")
                .bind(&sid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(StorageError::from_sqlx)?;

        if let Some((next,)) = row {
            if event.sequence == next {
                let updated_at = now_rfc3339();
                sqlx::query(
                    "UPDATE sessions SET next_sequence = ?1, updated_at = ?2 WHERE session_id = ?3",
                )
                .bind(next + 1)
                .bind(&updated_at)
                .bind(&sid)
                .execute(&mut *tx)
                .await
                .map_err(StorageError::from_sqlx)?;
            } else if event.sequence > next {
                return Err(StorageError::SequenceConflict {
                    session_id: sid,
                    expected: next,
                    got: event.sequence,
                });
            }
            // sequence < next: allow idempotent historical insert only if row exists
            // (unique index will reject duplicates).
        } else {
            return Err(StorageError::not_found("session", sid));
        }

        tx.commit().await.map_err(StorageError::from_sqlx)?;
        Ok(())
    }

    async fn list(
        &self,
        session_id: &SessionId,
        after_sequence: i64,
        limit: i64,
    ) -> StorageResult<EventList> {
        let limit = if limit <= 0 { 200 } else { limit };
        let rows = sqlx::query_as::<_, EventRow>(
            r#"
            SELECT event_id, session_id, project_id, agent_run_id, sequence,
                   event_version, event_type, severity, timestamp,
                   payload_json, adapter_json, envelope_json
            FROM events
            WHERE session_id = ?1 AND sequence > ?2
            ORDER BY sequence ASC
            LIMIT ?3
            "#,
        )
        .bind(session_id.as_str())
        .bind(after_sequence)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        let events: Result<Vec<_>, _> = rows.into_iter().map(|r| r.into_record()).collect();
        let events = events?;

        let latest: (i64,) =
            sqlx::query_as("SELECT COALESCE(MAX(sequence), 0) FROM events WHERE session_id = ?1")
                .bind(session_id.as_str())
                .fetch_one(&self.pool)
                .await
                .map_err(StorageError::from_sqlx)?;

        Ok(EventList {
            events,
            latest_sequence: latest.0,
        })
    }

    async fn get(&self, session_id: &SessionId, event_id: &EventId) -> StorageResult<EventRecord> {
        let row = sqlx::query_as::<_, EventRow>(
            r#"
            SELECT event_id, session_id, project_id, agent_run_id, sequence,
                   event_version, event_type, severity, timestamp,
                   payload_json, adapter_json, envelope_json
            FROM events
            WHERE session_id = ?1 AND event_id = ?2
            "#,
        )
        .bind(session_id.as_str())
        .bind(event_id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        match row {
            Some(r) => r.into_record(),
            None => Err(StorageError::not_found("event", event_id.as_str())),
        }
    }
}

async fn insert_event_tx(
    tx: &mut Transaction<'_, Sqlite>,
    event: &EventRecord,
) -> StorageResult<()> {
    let payload = serde_json::to_string(&event.payload)?;
    let adapter = event
        .adapter
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let envelope = serde_json::to_string(&event.to_envelope_json())?;
    let severity = event.severity.unwrap_or_default().as_str();
    let agent = event.agent_run_id.as_ref().map(|id| id.as_str());

    let res = sqlx::query(
        r#"
        INSERT INTO events (
            event_id, session_id, project_id, agent_run_id, sequence,
            event_version, event_type, severity, timestamp,
            payload_json, adapter_json, envelope_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
    )
    .bind(event.event_id.as_str())
    .bind(event.session_id.as_str())
    .bind(event.project_id.as_str())
    .bind(agent)
    .bind(event.sequence)
    .bind(event.event_version as i64)
    .bind(&event.event_type)
    .bind(severity)
    .bind(&event.timestamp)
    .bind(payload)
    .bind(adapter)
    .bind(envelope)
    .execute(&mut **tx)
    .await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            let mapped = StorageError::from_sqlx(e);
            if matches!(mapped, StorageError::AlreadyExists { .. }) {
                Err(StorageError::already_exists(
                    "event",
                    event.event_id.as_str(),
                ))
            } else {
                Err(mapped)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime process / approval / artifact
// ---------------------------------------------------------------------------

#[async_trait]
impl RuntimeProcessRepository for SqliteStorage {
    async fn insert(&self, process: &RuntimeProcessRecord) -> StorageResult<()> {
        let args = process
            .args
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO runtime_processes (
                process_id, session_id, pid, executable, args_json, cwd,
                status, exit_code, exit_signal, started_at, ended_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(process.process_id.as_str())
        .bind(process.session_id.as_str())
        .bind(process.pid)
        .bind(&process.executable)
        .bind(args)
        .bind(&process.cwd)
        .bind(process.status.as_str())
        .bind(process.exit_code)
        .bind(&process.exit_signal)
        .bind(&process.started_at)
        .bind(&process.ended_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;
        Ok(())
    }

    async fn update(&self, process: &RuntimeProcessRecord) -> StorageResult<()> {
        let args = process
            .args
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        let res = sqlx::query(
            r#"
            UPDATE runtime_processes SET
                pid = ?2, executable = ?3, args_json = ?4, cwd = ?5,
                status = ?6, exit_code = ?7, exit_signal = ?8,
                started_at = ?9, ended_at = ?10
            WHERE process_id = ?1
            "#,
        )
        .bind(process.process_id.as_str())
        .bind(process.pid)
        .bind(&process.executable)
        .bind(args)
        .bind(&process.cwd)
        .bind(process.status.as_str())
        .bind(process.exit_code)
        .bind(&process.exit_signal)
        .bind(&process.started_at)
        .bind(&process.ended_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        if res.rows_affected() == 0 {
            return Err(StorageError::not_found(
                "runtime_process",
                process.process_id.as_str(),
            ));
        }
        Ok(())
    }

    async fn list_by_session(
        &self,
        session_id: &SessionId,
    ) -> StorageResult<Vec<RuntimeProcessRecord>> {
        let rows = sqlx::query_as::<_, ProcessRow>(
            r#"
            SELECT process_id, session_id, pid, executable, args_json, cwd,
                   status, exit_code, exit_signal, started_at, ended_at
            FROM runtime_processes
            WHERE session_id = ?1
            ORDER BY started_at ASC
            "#,
        )
        .bind(session_id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        rows.into_iter().map(|r| r.into_record()).collect()
    }
}

#[async_trait]
impl ApprovalRepository for SqliteStorage {
    async fn insert(&self, decision: &ApprovalDecisionRecord) -> StorageResult<()> {
        let details = decision
            .details
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let event_id = decision.event_id.as_ref().map(|id| id.as_str());

        sqlx::query(
            r#"
            INSERT INTO approval_decisions (
                approval_id, session_id, event_id, decision, decided_at, details_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(decision.approval_id.as_str())
        .bind(decision.session_id.as_str())
        .bind(event_id)
        .bind(decision.decision.as_str())
        .bind(&decision.decided_at)
        .bind(details)
        .execute(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;
        Ok(())
    }

    async fn list_by_session(
        &self,
        session_id: &SessionId,
    ) -> StorageResult<Vec<ApprovalDecisionRecord>> {
        let rows = sqlx::query_as::<_, ApprovalRow>(
            r#"
            SELECT approval_id, session_id, event_id, decision, decided_at, details_json
            FROM approval_decisions
            WHERE session_id = ?1
            ORDER BY decided_at ASC
            "#,
        )
        .bind(session_id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        rows.into_iter().map(|r| r.into_record()).collect()
    }
}

#[async_trait]
impl ArtifactRepository for SqliteStorage {
    async fn insert(&self, artifact: &ArtifactRecord) -> StorageResult<()> {
        let metadata = artifact
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO artifacts (
                artifact_id, session_id, project_id, kind, path,
                summary, metadata_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(artifact.artifact_id.as_str())
        .bind(artifact.session_id.as_str())
        .bind(artifact.project_id.as_str())
        .bind(&artifact.kind)
        .bind(&artifact.path)
        .bind(&artifact.summary)
        .bind(metadata)
        .bind(&artifact.created_at)
        .execute(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;
        Ok(())
    }

    async fn list_by_session(&self, session_id: &SessionId) -> StorageResult<Vec<ArtifactRecord>> {
        let rows = sqlx::query_as::<_, ArtifactRow>(
            r#"
            SELECT artifact_id, session_id, project_id, kind, path,
                   summary, metadata_json, created_at
            FROM artifacts
            WHERE session_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from_sqlx)?;

        rows.into_iter().map(|r| r.into_record()).collect()
    }
}

// ---------------------------------------------------------------------------
// Row mappers
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct ProjectRow {
    project_id: String,
    name: String,
    root_path: String,
    status: String,
    is_git: i64,
    last_opened_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl ProjectRow {
    fn into_record(self) -> StorageResult<ProjectRecord> {
        Ok(ProjectRecord {
            project_id: ProjectId::parse(&self.project_id).map_err(|e| StorageError::Internal {
                message: format!("invalid project_id: {e}"),
            })?,
            name: self.name,
            root_path: self.root_path,
            status: ProjectStatus::parse(&self.status).ok_or_else(|| StorageError::Internal {
                message: format!("invalid project status `{}`", self.status),
            })?,
            is_git: self.is_git != 0,
            last_opened_at: self.last_opened_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    session_id: String,
    project_id: String,
    title: Option<String>,
    status: String,
    runtime_kind: Option<String>,
    runtime_session_id: Option<String>,
    capabilities_json: Option<String>,
    last_error_json: Option<String>,
    active_agent_run_id: Option<String>,
    next_sequence: i64,
    created_at: String,
    updated_at: String,
}

impl SessionRow {
    fn into_record(self) -> StorageResult<SessionRecord> {
        let capabilities = match self.capabilities_json {
            Some(s) => Some(serde_json::from_str(&s)?),
            None => None,
        };
        let last_error = match self.last_error_json {
            Some(s) => Some(serde_json::from_str(&s)?),
            None => None,
        };
        let active_agent_run_id = match self.active_agent_run_id {
            Some(s) => Some(AgentRunId::parse(&s).map_err(|e| StorageError::Internal {
                message: format!("invalid agent_run_id: {e}"),
            })?),
            None => None,
        };

        Ok(SessionRecord {
            session_id: SessionId::parse(&self.session_id).map_err(|e| StorageError::Internal {
                message: format!("invalid session_id: {e}"),
            })?,
            project_id: ProjectId::parse(&self.project_id).map_err(|e| StorageError::Internal {
                message: format!("invalid project_id: {e}"),
            })?,
            title: self.title,
            status: SessionStatus::parse(&self.status).ok_or_else(|| StorageError::Internal {
                message: format!("invalid session status `{}`", self.status),
            })?,
            runtime_kind: self.runtime_kind,
            runtime_session_id: self.runtime_session_id,
            capabilities,
            last_error,
            active_agent_run_id,
            next_sequence: self.next_sequence,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    event_id: String,
    session_id: String,
    project_id: String,
    agent_run_id: Option<String>,
    sequence: i64,
    event_version: i64,
    event_type: String,
    severity: String,
    timestamp: String,
    payload_json: String,
    adapter_json: Option<String>,
    envelope_json: String,
}

impl EventRow {
    fn into_record(self) -> StorageResult<EventRecord> {
        // Prefer full envelope for unknown-field preservation on round-trip.
        if let Ok(value) = serde_json::from_str::<JsonValue>(&self.envelope_json) {
            if let Ok(rec) = EventRecord::from_envelope_json(&value) {
                return Ok(rec);
            }
        }

        let payload: JsonValue = serde_json::from_str(&self.payload_json)?;
        let adapter = match self.adapter_json {
            Some(s) => Some(serde_json::from_str(&s)?),
            None => None,
        };
        let agent_run_id = match self.agent_run_id {
            Some(s) => Some(AgentRunId::parse(&s).map_err(|e| StorageError::Internal {
                message: format!("invalid agent_run_id: {e}"),
            })?),
            None => None,
        };

        Ok(EventRecord {
            event_version: self.event_version as u32,
            event_id: EventId::parse(&self.event_id).map_err(|e| StorageError::Internal {
                message: format!("invalid event_id: {e}"),
            })?,
            sequence: self.sequence,
            timestamp: self.timestamp,
            project_id: ProjectId::parse(&self.project_id).map_err(|e| StorageError::Internal {
                message: format!("invalid project_id: {e}"),
            })?,
            session_id: SessionId::parse(&self.session_id).map_err(|e| StorageError::Internal {
                message: format!("invalid session_id: {e}"),
            })?,
            agent_run_id,
            event_type: self.event_type,
            payload,
            adapter,
            severity: Severity::parse(&self.severity),
        })
    }
}

#[derive(sqlx::FromRow)]
struct ProcessRow {
    process_id: String,
    session_id: String,
    pid: Option<i64>,
    executable: Option<String>,
    args_json: Option<String>,
    cwd: Option<String>,
    status: String,
    exit_code: Option<i64>,
    exit_signal: Option<String>,
    started_at: String,
    ended_at: Option<String>,
}

impl ProcessRow {
    fn into_record(self) -> StorageResult<RuntimeProcessRecord> {
        let args = match self.args_json {
            Some(s) => Some(serde_json::from_str(&s)?),
            None => None,
        };
        Ok(RuntimeProcessRecord {
            process_id: ProcessId::parse(&self.process_id).map_err(|e| StorageError::Internal {
                message: format!("invalid process_id: {e}"),
            })?,
            session_id: SessionId::parse(&self.session_id).map_err(|e| StorageError::Internal {
                message: format!("invalid session_id: {e}"),
            })?,
            pid: self.pid,
            executable: self.executable,
            args,
            cwd: self.cwd,
            status: RuntimeProcessStatus::parse(&self.status).ok_or_else(|| {
                StorageError::Internal {
                    message: format!("invalid process status `{}`", self.status),
                }
            })?,
            exit_code: self.exit_code,
            exit_signal: self.exit_signal,
            started_at: self.started_at,
            ended_at: self.ended_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ApprovalRow {
    approval_id: String,
    session_id: String,
    event_id: Option<String>,
    decision: String,
    decided_at: String,
    details_json: Option<String>,
}

impl ApprovalRow {
    fn into_record(self) -> StorageResult<ApprovalDecisionRecord> {
        let details = match self.details_json {
            Some(s) => Some(serde_json::from_str(&s)?),
            None => None,
        };
        let event_id = match self.event_id {
            Some(s) => Some(EventId::parse(&s).map_err(|e| StorageError::Internal {
                message: format!("invalid event_id: {e}"),
            })?),
            None => None,
        };
        Ok(ApprovalDecisionRecord {
            approval_id: ApprovalId::parse(&self.approval_id).map_err(|e| {
                StorageError::Internal {
                    message: format!("invalid approval_id: {e}"),
                }
            })?,
            session_id: SessionId::parse(&self.session_id).map_err(|e| StorageError::Internal {
                message: format!("invalid session_id: {e}"),
            })?,
            event_id,
            decision: ApprovalDecision::parse(&self.decision).ok_or_else(|| {
                StorageError::Internal {
                    message: format!("invalid decision `{}`", self.decision),
                }
            })?,
            decided_at: self.decided_at,
            details,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ArtifactRow {
    artifact_id: String,
    session_id: String,
    project_id: String,
    kind: String,
    path: Option<String>,
    summary: Option<String>,
    metadata_json: Option<String>,
    created_at: String,
}

impl ArtifactRow {
    fn into_record(self) -> StorageResult<ArtifactRecord> {
        let metadata = match self.metadata_json {
            Some(s) => Some(serde_json::from_str(&s)?),
            None => None,
        };
        Ok(ArtifactRecord {
            artifact_id: ArtifactId::parse(&self.artifact_id).map_err(|e| {
                StorageError::Internal {
                    message: format!("invalid artifact_id: {e}"),
                }
            })?,
            session_id: SessionId::parse(&self.session_id).map_err(|e| StorageError::Internal {
                message: format!("invalid session_id: {e}"),
            })?,
            project_id: ProjectId::parse(&self.project_id).map_err(|e| StorageError::Internal {
                message: format!("invalid project_id: {e}"),
            })?,
            kind: self.kind,
            path: self.path,
            summary: self.summary,
            metadata,
            created_at: self.created_at,
        })
    }
}

// Silence unused import if FromStr is only used via parse methods.
#[allow(dead_code)]
fn _assert_from_str() {
    let _ = SessionId::from_str;
}
