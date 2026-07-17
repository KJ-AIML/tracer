//! VS-10 — Persistence and reload (storage layer evidence).
//!
//! Steps (storage-scoped):
//! 1. Store ≥ N events for a session.
//! 2. Close the pool (app shutdown).
//! 3. Re-open; list session + events.
//!
//! Must observe: same sessionId; events identical by sequence/eventId;
//! status not falsely "running" after reconcile; unknown types preserved.

use serde_json::json;
use tempfile::TempDir;
use tracer_storage::{
    database_path, open_database, EventId, EventRecord, OpenOptions, ProjectId, ProjectRecord,
    ProjectStatus, SessionId, SessionRecord, SessionStatus, Severity, SqliteStorage,
};

#[tokio::test]
async fn vs10_persistence_and_reload() {
    let tmp = TempDir::new().unwrap();
    let db_path = database_path(tmp.path());

    let project_id = ProjectId::new();
    let session_id = SessionId::new();
    let now = tracer_storage::now_rfc3339();

    let mut expected: Vec<(i64, EventId, String)> = Vec::new();

    {
        let pool = open_database(&db_path, OpenOptions::default())
            .await
            .expect("open");
        let store = SqliteStorage::new(pool);

        store
            .insert_project(&ProjectRecord {
                project_id,
                name: "vs10".into(),
                root_path: "/tmp/vs10-project".into(),
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
                title: Some("VS-10 session".into()),
                status: SessionStatus::Running,
                runtime_kind: Some("acp-stdio".into()),
                runtime_session_id: Some("rt-sess-test".into()),
                capabilities: Some(json!({"cancellation": true})),
                last_error: None,
                active_agent_run_id: None,
                next_sequence: 1,
                created_at: now.clone(),
                updated_at: now.clone(),
            })
            .await
            .unwrap();

        let catalog = [
            ("session.created", json!({"title": "VS-10 session"})),
            ("runtime.process.started", json!({"pid": 1234})),
            ("runtime.process.ready", json!({"capabilities": {"promptStreaming": true}})),
            ("session.ready", json!({})),
            ("session.prompt.submitted", json!({"promptId": "p1", "text": "hi"})),
            (
                "adapter.protocol.unknown",
                json!({"vendor": "x", "rawSummary": "keep"}),
            ),
            ("agent.message.delta", json!({"delta": "Hello"})),
            ("session.completed", json!({"summary": "done"})),
        ];

        for (ty, payload) in catalog {
            let rec = EventRecord {
                event_version: 1,
                event_id: EventId::new(),
                sequence: 0,
                timestamp: now.clone(),
                project_id,
                session_id,
                agent_run_id: None,
                event_type: ty.into(),
                payload,
                adapter: if ty.starts_with("adapter.") {
                    Some(json!({"runtimeKind": "acp-stdio", "extra": true}))
                } else {
                    None
                },
                severity: Some(Severity::Info),
            };
            let stored = store.append_event(rec).await.expect("append");
            expected.push((stored.sequence, stored.event_id, ty.into()));
        }
        // pool / store dropped → shutdown
    }

    // Restart
    {
        let pool = open_database(&db_path, OpenOptions::default())
            .await
            .expect("reopen");
        let store = SqliteStorage::new(pool);

        let session = store.get_session(&session_id).await.expect("session");
        assert_eq!(session.session_id, session_id);
        assert_eq!(session.project_id, project_id);

        // Before reconcile, durable status may still be running (history truth).
        assert_eq!(session.status, SessionStatus::Running);

        // Boot reconcile: no live process → disconnected (F-S04).
        let report = store
            .reconcile_stale_live_sessions(SessionStatus::Disconnected)
            .await
            .expect("reconcile");
        assert!(report.sessions_updated.contains(&session_id));

        let session = store
            .get_session(&session_id)
            .await
            .expect("session after reconcile");
        assert_eq!(session.status, SessionStatus::Disconnected);

        let list = store
            .list_events(&session_id, 0, 200)
            .await
            .expect("events");
        assert_eq!(list.events.len(), expected.len());
        assert_eq!(list.latest_sequence, expected.len() as i64);

        for (i, ev) in list.events.iter().enumerate() {
            assert_eq!(ev.sequence, expected[i].0);
            assert_eq!(ev.event_id, expected[i].1);
            assert_eq!(ev.event_type, expected[i].2);
        }

        // Unknown type preserved
        let unknown = list
            .events
            .iter()
            .find(|e| e.event_type == "adapter.protocol.unknown")
            .expect("unknown event");
        assert_eq!(unknown.payload["vendor"], "x");
        assert_eq!(unknown.adapter.as_ref().unwrap()["extra"], true);
    }
}
