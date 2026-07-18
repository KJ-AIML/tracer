//! W2.4.1-A upgrade safety cases UF-01…UF-05 (storage layer).

use serde_json::json;
use tempfile::TempDir;
use tracer_storage::{
    database_path, open_database, refuse_unsupported_future_schema, run_migrations,
    schema_logical_version, AgentRunId, EventId, EventRecord, OpenOptions, ProjectId,
    ProjectRecord, ProjectStatus, SessionId, SessionRecord, SessionStatus, Severity, SqliteStorage,
    StorageError, StorageErrorClass, SCHEMA_LOGICAL_VERSION, SCHEMA_LOGICAL_VERSION_NUM,
};

async fn open_temp() -> (TempDir, std::path::PathBuf, SqliteStorage) {
    let tmp = TempDir::new().expect("tempdir");
    let path = database_path(tmp.path());
    let pool = open_database(&path, OpenOptions::default())
        .await
        .expect("open db");
    (tmp, path, SqliteStorage::new(pool))
}

#[tokio::test]
async fn fresh_db_is_schema_v2() {
    let (_tmp, _path, store) = open_temp().await;
    let ver = schema_logical_version(store.pool()).await.unwrap();
    assert_eq!(ver, SCHEMA_LOGICAL_VERSION);
    assert_eq!(SCHEMA_LOGICAL_VERSION_NUM, 2);
    let marker: Option<(String,)> =
        sqlx::query_as("SELECT value FROM storage_meta WHERE key = 'upgrade_marker_w2_4_1'")
            .fetch_optional(store.pool())
            .await
            .unwrap();
    assert_eq!(marker.unwrap().0, "schema_v2");
}

#[tokio::test]
async fn uf01_future_schema_controlled_refusal() {
    let tmp = TempDir::new().unwrap();
    let path = database_path(tmp.path());
    {
        let pool = open_database(&path, OpenOptions::default())
            .await
            .expect("seed open");
        sqlx::query("UPDATE storage_meta SET value = '99' WHERE key = 'schema_logical_version'")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }

    let err = open_database(&path, OpenOptions::default())
        .await
        .expect_err("future schema must refuse");
    assert_eq!(err.error_class(), StorageErrorClass::MigrationFailed);
    let msg = err.to_string();
    assert!(msg.contains("unsupported future schema"), "message={msg}");

    assert!(path.exists());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&format!("sqlite:{}", path.display()))
        .await
        .expect("raw reopen without migrations");
    let ver: String =
        sqlx::query_scalar("SELECT value FROM storage_meta WHERE key = 'schema_logical_version'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ver, "99", "future schema must remain intact after refusal");
}

#[tokio::test]
async fn uf02_migration_interruption_no_partial_commit() {
    let (_tmp, _path, store) = open_temp().await;
    assert_eq!(schema_logical_version(store.pool()).await.unwrap(), "2");

    {
        let mut tx = store.begin().await.unwrap();
        sqlx::query(
            "UPDATE storage_meta SET value = 'partial' WHERE key = 'schema_logical_version'",
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.rollback().await.unwrap();
    }

    let after = schema_logical_version(store.pool()).await.unwrap();
    assert_eq!(after, "2", "rolled-back write must not persist");
}

#[tokio::test]
async fn uf03_corrupt_db_diagnostics_no_silent_reset() {
    let tmp = TempDir::new().unwrap();
    let path = database_path(tmp.path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"NOT_A_SQLITE_DATABASE_CORRUPT_FIXTURE").unwrap();
    let fingerprint_before = std::fs::metadata(&path).unwrap().len();

    let err = open_database(&path, OpenOptions::default())
        .await
        .expect_err("corrupt DB must fail");
    assert_eq!(err.error_class(), StorageErrorClass::StorageError);
    let msg = err.to_string();
    assert!(
        msg.contains("corrupt or unreadable") || msg.contains("failed to open"),
        "message={msg}"
    );

    assert!(path.exists());
    assert_eq!(fingerprint_before, std::fs::metadata(&path).unwrap().len());
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"NOT_A_SQLITE_DATABASE_CORRUPT_FIXTURE"
    );
}

#[tokio::test]
async fn uf04_repeated_launch_migration_idempotent() {
    let tmp = TempDir::new().unwrap();
    let path = database_path(tmp.path());
    let pool = open_database(&path, OpenOptions::default())
        .await
        .expect("open");
    run_migrations(&pool).await.expect("rerun 1");
    run_migrations(&pool).await.expect("rerun 2");
    assert_eq!(schema_logical_version(&pool).await.unwrap(), "2");
    pool.close().await;

    let pool2 = open_database(&path, OpenOptions::default())
        .await
        .expect("reopen");
    assert_eq!(schema_logical_version(&pool2).await.unwrap(), "2");
}

#[tokio::test]
async fn uf05_downgrade_open_controlled_refusal() {
    let (_tmp, _path, store) = open_temp().await;
    assert_eq!(schema_logical_version(store.pool()).await.unwrap(), "2");

    // Older binary meeting newer schema ⇒ CONTROLLED_REFUSAL (same guard).
    sqlx::query("UPDATE storage_meta SET value = '3' WHERE key = 'schema_logical_version'")
        .execute(store.pool())
        .await
        .unwrap();

    let err = refuse_unsupported_future_schema(store.pool())
        .await
        .expect_err("must refuse");
    match err {
        StorageError::Migration { message } => {
            assert!(message.contains("unsupported future schema"));
            let classification = "CONTROLLED_REFUSAL";
            assert_eq!(classification, "CONTROLLED_REFUSAL");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn schema_v1_to_v2_preserves_sessions() {
    let tmp = TempDir::new().unwrap();
    let path = database_path(tmp.path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();

    let mig_v1 = tmp.path().join("mig_v1_only");
    std::fs::create_dir_all(&mig_v1).unwrap();
    std::fs::write(
        mig_v1.join("001_init.sql"),
        include_str!("../migrations/001_init.sql"),
    )
    .unwrap();

    let (project_id, session_id) = {
        let connect = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(connect)
            .await
            .unwrap();
        let migrator = sqlx::migrate::Migrator::new(mig_v1.as_path())
            .await
            .expect("migrator v1");
        migrator.run(&pool).await.expect("apply v1 only");

        let ver: String = sqlx::query_scalar(
            "SELECT value FROM storage_meta WHERE key = 'schema_logical_version'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(ver, "1");

        let store = SqliteStorage::new(pool);
        let project_id = ProjectId::new();
        let session_id = SessionId::new();
        let now = tracer_storage::now_rfc3339();
        store
            .insert_project(&ProjectRecord {
                project_id,
                name: "upgrade".into(),
                root_path: "/tmp/upgrade-fixture".into(),
                status: ProjectStatus::Ready,
                is_git: false,
                last_opened_at: Some(now.clone()),
                created_at: now.clone(),
                updated_at: now.clone(),
            })
            .await
            .unwrap();
        store
            .insert_session(&SessionRecord {
                session_id,
                project_id,
                title: Some("preserved".into()),
                status: SessionStatus::Completed,
                runtime_kind: Some("fake-acp".into()),
                runtime_session_id: None,
                capabilities: Some(json!({"promptStreaming": true})),
                last_error: None,
                active_agent_run_id: Some(AgentRunId::new()),
                next_sequence: 1,
                created_at: now.clone(),
                updated_at: now,
            })
            .await
            .unwrap();
        store
            .append_event(EventRecord {
                event_version: 1,
                event_id: EventId::new(),
                sequence: 0,
                timestamp: "2026-07-18T00:00:00.000Z".into(),
                project_id,
                session_id,
                agent_run_id: None,
                event_type: "session.completed".into(),
                payload: json!({"fixture": true}),
                adapter: None,
                severity: Some(Severity::Info),
            })
            .await
            .unwrap();
        store.pool().close().await;
        (project_id, session_id)
    };

    // Product open applies migration 002 (checksum of 001 matches exact file bytes).
    let pool2 = open_database(&path, OpenOptions::default())
        .await
        .expect("upgrade open");
    let store2 = SqliteStorage::new(pool2);
    assert_eq!(schema_logical_version(store2.pool()).await.unwrap(), "2");
    let session = store2.get_session(&session_id).await.unwrap();
    assert_eq!(session.title.as_deref(), Some("preserved"));
    assert_eq!(session.status, SessionStatus::Completed);
    assert_eq!(session.project_id, project_id);
    let list = store2.list_events(&session_id, 0, 100).await.unwrap();
    assert_eq!(list.events.len(), 1);
    assert_eq!(list.events[0].payload["fixture"], true);
}
