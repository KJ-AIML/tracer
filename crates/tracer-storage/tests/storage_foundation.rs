//! T3 storage foundation tests (temporary databases).
//!
//! Covers acceptance from WAVE_1_READINESS W1-E and TEST_STRATEGY §10:
//! fresh DB, migration rerun, ordered replay, unknown payload preservation,
//! interrupted write, path derivation, reload after reopen, F-S04 reconcile.

use serde_json::json;
use tempfile::TempDir;
use tracer_storage::{
    database_path, open_database, run_migrations, schema_logical_version, writer_policy,
    AgentRunId, EventId, EventRecord, OpenOptions, ProjectId, ProjectRecord, ProjectStatus,
    SessionId, SessionRecord, SessionStatus, Severity, SqliteStorage, StorageError,
    StorageErrorClass, SCHEMA_LOGICAL_VERSION,
};

async fn open_temp() -> (TempDir, SqliteStorage) {
    let tmp = TempDir::new().expect("tempdir");
    let path = database_path(tmp.path());
    let pool = open_database(&path, OpenOptions::default())
        .await
        .expect("open db");
    (tmp, SqliteStorage::new(pool))
}

async fn seed_project_session(store: &SqliteStorage) -> (ProjectId, SessionId) {
    let project_id = ProjectId::new();
    let session_id = SessionId::new();
    let now = tracer_storage::now_rfc3339();

    store
        .insert_project(&ProjectRecord {
            project_id,
            name: "demo".into(),
            root_path: "/tmp/demo-project".into(), // test-only placeholder path
            status: ProjectStatus::Ready,
            is_git: true,
            last_opened_at: Some(now.clone()),
            created_at: now.clone(),
            updated_at: now.clone(),
        })
        .await
        .expect("insert project");

    store
        .insert_session(&SessionRecord {
            session_id,
            project_id,
            title: Some("test session".into()),
            status: SessionStatus::Running,
            runtime_kind: Some("acp-stdio".into()),
            runtime_session_id: None,
            capabilities: Some(json!({"promptStreaming": true})),
            last_error: None,
            active_agent_run_id: Some(AgentRunId::new()),
            next_sequence: 1,
            created_at: now.clone(),
            updated_at: now,
        })
        .await
        .expect("insert session");

    (project_id, session_id)
}

fn sample_event(
    project_id: ProjectId,
    session_id: SessionId,
    event_type: &str,
    payload: serde_json::Value,
) -> EventRecord {
    EventRecord {
        event_version: 1,
        event_id: EventId::new(),
        sequence: 0, // assigned by append_event
        timestamp: "2026-07-17T12:00:00.000Z".into(),
        project_id,
        session_id,
        agent_run_id: None,
        event_type: event_type.into(),
        payload,
        adapter: None,
        severity: Some(Severity::Info),
    }
}

#[tokio::test]
async fn fresh_database_and_migrations() {
    let (_tmp, store) = open_temp().await;
    let ver = schema_logical_version(store.pool())
        .await
        .expect("schema version");
    assert_eq!(ver, SCHEMA_LOGICAL_VERSION);

    let policy = writer_policy(store.pool()).await.expect("writer policy");
    assert_eq!(policy, "control_plane_only");
}

#[tokio::test]
async fn migration_rerun_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let path = database_path(tmp.path());
    let pool = open_database(&path, OpenOptions::default())
        .await
        .expect("open");
    // Re-run migrations against the same pool.
    run_migrations(&pool).await.expect("rerun migrations");
    run_migrations(&pool).await.expect("second rerun");

    let store = SqliteStorage::new(pool);
    let (project_id, session_id) = seed_project_session(&store).await;
    let e = store
        .append_event(sample_event(
            project_id,
            session_id,
            "session.created",
            json!({}),
        ))
        .await
        .expect("append");
    assert_eq!(e.sequence, 1);
}

#[tokio::test]
async fn ordered_event_replay() {
    let (_tmp, store) = open_temp().await;
    let (project_id, session_id) = seed_project_session(&store).await;

    let types = [
        "session.created",
        "runtime.process.started",
        "runtime.process.ready",
        "session.ready",
        "session.prompt.submitted",
        "agent.message.delta",
        "session.completed",
    ];

    let mut expected_ids = Vec::new();
    for t in types {
        let e = store
            .append_event(sample_event(project_id, session_id, t, json!({"n": t})))
            .await
            .expect("append");
        expected_ids.push((e.sequence, e.event_id));
    }

    let list = store
        .list_events(&session_id, 0, 200)
        .await
        .expect("list");
    assert_eq!(list.events.len(), types.len());
    assert_eq!(list.latest_sequence, types.len() as i64);

    for (i, ev) in list.events.iter().enumerate() {
        assert_eq!(ev.sequence, (i + 1) as i64);
        assert_eq!(ev.event_id, expected_ids[i].1);
        assert_eq!(ev.event_type, types[i]);
        // strictly increasing
        if i > 0 {
            assert!(ev.sequence > list.events[i - 1].sequence);
        }
    }

    // Pagination: after_sequence
    let page = store
        .list_events(&session_id, 3, 2)
        .await
        .expect("page");
    assert_eq!(page.events.len(), 2);
    assert_eq!(page.events[0].sequence, 4);
    assert_eq!(page.events[1].sequence, 5);
}

#[tokio::test]
async fn unknown_event_type_and_payload_preserved() {
    let (_tmp, store) = open_temp().await;
    let (project_id, session_id) = seed_project_session(&store).await;

    let payload = json!({
        "knownField": 1,
        "vendorSpecificWeirdField": {"nested": true, "arr": [1, 2, 3]},
        "extra": "keep-me"
    });

    let mut event = sample_event(
        project_id,
        session_id,
        "adapter.protocol.unknown",
        payload.clone(),
    );
    event.adapter = Some(json!({
        "runtimeKind": "acp-stdio",
        "rawRef": "opaque-fragment",
        "futureAdapterField": 42
    }));

    let stored = store.append_event(event).await.expect("append");
    assert_eq!(stored.sequence, 1);

    let loaded = store
        .get_event(&session_id, &stored.event_id)
        .await
        .expect("get");

    assert_eq!(loaded.event_type, "adapter.protocol.unknown");
    assert_eq!(loaded.payload, payload);
    assert_eq!(
        loaded.adapter.as_ref().unwrap()["futureAdapterField"],
        42
    );
    assert_eq!(
        loaded.adapter.as_ref().unwrap()["rawRef"],
        "opaque-fragment"
    );

    // Full envelope round-trip
    let env = loaded.to_envelope_json();
    let back = EventRecord::from_envelope_json(&env).expect("parse envelope");
    assert_eq!(back.event_id, loaded.event_id);
    assert_eq!(back.payload["vendorSpecificWeirdField"]["nested"], true);
}

#[tokio::test]
async fn interrupted_write_rolls_back() {
    let (_tmp, store) = open_temp().await;
    let (project_id, session_id) = seed_project_session(&store).await;

    // Begin a transaction, insert an event, then drop/rollback without commit.
    {
        let mut tx = store.begin().await.expect("begin");
        let event = EventRecord {
            event_version: 1,
            event_id: EventId::new(),
            sequence: 1,
            timestamp: "2026-07-17T12:00:00.000Z".into(),
            project_id,
            session_id,
            agent_run_id: None,
            event_type: "session.created".into(),
            payload: json!({}),
            adapter: None,
            severity: Some(Severity::Info),
        };
        let payload = serde_json::to_string(&event.payload).unwrap();
        let envelope = serde_json::to_string(&event.to_envelope_json()).unwrap();
        sqlx::query(
            r#"
            INSERT INTO events (
                event_id, session_id, project_id, agent_run_id, sequence,
                event_version, event_type, severity, timestamp,
                payload_json, adapter_json, envelope_json
            ) VALUES (?1, ?2, ?3, NULL, ?4, 1, ?5, 'info', ?6, ?7, NULL, ?8)
            "#,
        )
        .bind(event.event_id.as_str())
        .bind(event.session_id.as_str())
        .bind(event.project_id.as_str())
        .bind(event.sequence)
        .bind(&event.event_type)
        .bind(&event.timestamp)
        .bind(payload)
        .bind(envelope)
        .execute(&mut *tx)
        .await
        .expect("insert in tx");

        // Explicit rollback (drop would also roll back).
        tx.rollback().await.expect("rollback");
    }

    let list = store
        .list_events(&session_id, 0, 100)
        .await
        .expect("list");
    assert!(
        list.events.is_empty(),
        "rolled-back event must not be durable"
    );

    // Session next_sequence unchanged; append still gets sequence 1.
    let e = store
        .append_event(sample_event(
            project_id,
            session_id,
            "session.created",
            json!({}),
        ))
        .await
        .expect("append after rollback");
    assert_eq!(e.sequence, 1);
}

#[tokio::test]
async fn reload_after_reopen() {
    let tmp = TempDir::new().unwrap();
    let path = database_path(tmp.path());
    let (project_id, session_id, event_ids) = {
        let pool = open_database(&path, OpenOptions::default())
            .await
            .expect("open");
        let store = SqliteStorage::new(pool);
        let (project_id, session_id) = seed_project_session(&store).await;
        let mut event_ids = Vec::new();
        for i in 0..5 {
            let e = store
                .append_event(sample_event(
                    project_id,
                    session_id,
                    "agent.message.delta",
                    json!({"i": i}),
                ))
                .await
                .expect("append");
            event_ids.push(e.event_id);
        }
        // pool drops here
        (project_id, session_id, event_ids)
    };

    // Re-open (simulates app restart).
    let pool = open_database(&path, OpenOptions::default())
        .await
        .expect("reopen");
    let store = SqliteStorage::new(pool);

    let session = store
        .get_session(&session_id)
        .await
        .expect("session after restart");
    assert_eq!(session.session_id, session_id);
    assert_eq!(session.project_id, project_id);
    assert_eq!(session.status, SessionStatus::Running); // still as stored

    let list = store
        .list_events(&session_id, 0, 100)
        .await
        .expect("list after restart");
    assert_eq!(list.events.len(), 5);
    for (i, ev) in list.events.iter().enumerate() {
        assert_eq!(ev.sequence, (i + 1) as i64);
        assert_eq!(ev.event_id, event_ids[i]);
        assert_eq!(ev.payload["i"], i as i64);
    }
}

#[tokio::test]
async fn reconcile_stale_running_after_restart() {
    let (_tmp, store) = open_temp().await;
    let (_project_id, session_id) = seed_project_session(&store).await;
    assert_eq!(
        store.get_session(&session_id).await.unwrap().status,
        SessionStatus::Running
    );

    let report = store
        .reconcile_stale_live_sessions(SessionStatus::Disconnected)
        .await
        .expect("reconcile");

    assert!(report.sessions_updated.contains(&session_id));
    assert_eq!(report.target_status, SessionStatus::Disconnected);

    let session = store.get_session(&session_id).await.expect("get");
    assert_eq!(session.status, SessionStatus::Disconnected);
    assert!(!session.status.implies_live_process());
}

#[tokio::test]
async fn database_path_uses_app_data_root() {
    let root = std::path::Path::new("platform-app-data");
    let path = database_path(root);
    assert!(path.ends_with("tracer/tracer.db") || path.ends_with(r"tracer\tracer.db"));
    let s = path.to_string_lossy();
    assert!(!s.contains("/Users/"));
    assert!(!s.contains(r"C:\Users"));
}

#[tokio::test]
async fn storage_error_mapping() {
    let (_tmp, store) = open_temp().await;
    let err = store
        .get_project(&ProjectId::new())
        .await
        .expect_err("missing project");
    assert_eq!(err.error_class(), StorageErrorClass::NotFound);
    assert_eq!(err.error_class().as_str(), "NotFound");
    assert!(!err.retryable());

    let db_err = StorageError::Database {
        message: "disk full".into(),
        source: None,
    };
    assert_eq!(db_err.error_class().as_str(), "StorageError");
    assert!(db_err.retryable());
}

#[tokio::test]
async fn no_secrets_columns_in_schema() {
    let (_tmp, store) = open_temp().await;
    // Enumerate columns across user tables; refuse secret-ish names.
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT m.name AS table_name, p.name AS column_name
        FROM sqlite_master m
        JOIN pragma_table_info(m.name) p
        WHERE m.type = 'table' AND m.name NOT LIKE 'sqlx_%' AND m.name NOT LIKE 'sqlite_%'
        "#,
    )
    .fetch_all(store.pool())
    .await
    .expect("schema introspect");

    let forbidden = ["token", "password", "secret", "api_key", "apikey", "credential"];
    for (table, col) in &rows {
        let col_l = col.to_lowercase();
        for bad in forbidden {
            assert!(
                !col_l.contains(bad),
                "table `{table}` column `{col}` looks like a secrets column"
            );
        }
    }
}

#[tokio::test]
async fn batch_append_assigns_contiguous_sequences() {
    let (_tmp, store) = open_temp().await;
    let (project_id, session_id) = seed_project_session(&store).await;

    let batch = vec![
        sample_event(project_id, session_id, "session.created", json!({})),
        sample_event(project_id, session_id, "runtime.process.ready", json!({})),
        sample_event(project_id, session_id, "session.ready", json!({})),
    ];
    let out = store.append_events(batch).await.expect("batch");
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].sequence, 1);
    assert_eq!(out[1].sequence, 2);
    assert_eq!(out[2].sequence, 3);

    let session = store.get_session(&session_id).await.unwrap();
    assert_eq!(session.next_sequence, 4);
}

#[tokio::test]
async fn append_events_is_transactional() {
    // If one insert would fail mid-batch, nothing commits.
    let (_tmp, store) = open_temp().await;
    let (project_id, session_id) = seed_project_session(&store).await;

    // First succeed a single event.
    store
        .append_event(sample_event(
            project_id,
            session_id,
            "session.created",
            json!({}),
        ))
        .await
        .unwrap();

    // Force a duplicate event_id in a batch so unique constraint fails.
    let id = EventId::new();
    let mut a = sample_event(project_id, session_id, "x", json!({}));
    let mut b = sample_event(project_id, session_id, "y", json!({}));
    a.event_id = id;
    b.event_id = id;
    let err = store.append_events(vec![a, b]).await.expect_err("dup id");
    assert!(matches!(
        err.error_class(),
        StorageErrorClass::AlreadyExists | StorageErrorClass::StorageError
    ));

    let list = store.list_events(&session_id, 0, 100).await.unwrap();
    assert_eq!(list.events.len(), 1, "failed batch must not leave partial rows");
    assert_eq!(list.latest_sequence, 1);
}
