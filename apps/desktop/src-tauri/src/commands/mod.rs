//! Tauri command handlers — thin glue over `tracer-control-plane`.
//!
//! No raw ACP, no direct SQLite, no process management.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};
use tauri::State;
use tracer_control_plane::{ControlPlane, ControlPlaneError, RuntimeCreateOptions};

/// Shared control plane state.
pub struct PlaneState {
    pub plane: Arc<ControlPlane>,
}

fn map_err(e: ControlPlaneError) -> String {
    serde_json::to_string(&e.to_command_error()).unwrap_or_else(|_| {
        json!({
            "errorClass": "InternalError",
            "message": e.to_string(),
            "retryable": false
        })
        .to_string()
    })
}

// --- App ---

#[tauri::command]
pub fn tracer_app_info(state: State<'_, PlaneState>) -> Result<Value, String> {
    serde_json::to_value(state.plane.app_info()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tracer_presentation_snapshot(state: State<'_, PlaneState>) -> Result<Value, String> {
    serde_json::to_value(state.plane.snapshot()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tracer_heli_status(state: State<'_, PlaneState>) -> Result<Value, String> {
    serde_json::to_value(state.plane.refresh_heli()).map_err(|e| e.to_string())
}

// --- Projects ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRegisterArgs {
    pub root_path: String,
    pub name: Option<String>,
}

#[tauri::command]
pub async fn tracer_project_register(
    state: State<'_, PlaneState>,
    args: ProjectRegisterArgs,
) -> Result<Value, String> {
    let p = state
        .plane
        .project_register(&args.root_path, args.name)
        .await
        .map_err(map_err)?;
    Ok(json!({ "project": p }))
}

#[tauri::command]
pub async fn tracer_project_list(state: State<'_, PlaneState>) -> Result<Value, String> {
    let projects = state.plane.project_list().await.map_err(map_err)?;
    Ok(json!({ "projects": projects }))
}

#[tauri::command]
pub async fn tracer_project_get(
    state: State<'_, PlaneState>,
    project_id: String,
) -> Result<Value, String> {
    let project = state
        .plane
        .project_get(&project_id)
        .await
        .map_err(map_err)?;
    Ok(json!({ "project": project }))
}

// --- Sessions ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListArgs {
    pub project_id: String,
    pub limit: Option<i64>,
}

#[tauri::command]
pub async fn tracer_session_list(
    state: State<'_, PlaneState>,
    args: SessionListArgs,
) -> Result<Value, String> {
    let sessions = state
        .plane
        .session_list(&args.project_id, args.limit.unwrap_or(50))
        .await
        .map_err(map_err)?;
    Ok(json!({ "sessions": sessions, "nextCursor": null }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateArgs {
    pub project_id: String,
    pub title: Option<String>,
    pub runtime: Option<RuntimeCreateOptions>,
}

#[tauri::command]
pub async fn tracer_session_create(
    state: State<'_, PlaneState>,
    args: SessionCreateArgs,
) -> Result<Value, String> {
    let runtime = args.runtime.unwrap_or_default();
    let session = state
        .plane
        .session_create(&args.project_id, args.title, runtime)
        .await
        .map_err(map_err)?;
    Ok(json!({ "session": session }))
}

#[tauri::command]
pub async fn tracer_session_get(
    state: State<'_, PlaneState>,
    session_id: String,
) -> Result<Value, String> {
    let session = state
        .plane
        .session_get(&session_id)
        .await
        .map_err(map_err)?;
    Ok(json!({ "session": session }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitPromptArgs {
    pub session_id: String,
    pub text: String,
}

#[tauri::command]
pub async fn tracer_session_submit_prompt(
    state: State<'_, PlaneState>,
    args: SubmitPromptArgs,
) -> Result<Value, String> {
    let result = state
        .plane
        .session_submit_prompt(&args.session_id, &args.text)
        .await
        .map_err(map_err)?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelArgs {
    pub session_id: String,
    pub scope: Option<String>,
}

#[tauri::command]
pub async fn tracer_session_cancel(
    state: State<'_, PlaneState>,
    args: CancelArgs,
) -> Result<Value, String> {
    let _ = args.scope;
    let result = state
        .plane
        .session_cancel(&args.session_id)
        .await
        .map_err(map_err)?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopArgs {
    pub session_id: String,
    pub force: Option<bool>,
}

#[tauri::command]
pub async fn tracer_session_stop(
    state: State<'_, PlaneState>,
    args: StopArgs,
) -> Result<Value, String> {
    state
        .plane
        .session_stop(&args.session_id, args.force.unwrap_or(false))
        .await
        .map_err(map_err)
}

// --- Events ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsListArgs {
    pub session_id: String,
    pub after_sequence: Option<i64>,
    pub limit: Option<i64>,
}

#[tauri::command]
pub async fn tracer_events_list(
    state: State<'_, PlaneState>,
    args: EventsListArgs,
) -> Result<Value, String> {
    let result = state
        .plane
        .events_list(
            &args.session_id,
            args.after_sequence.unwrap_or(0),
            args.limit.unwrap_or(200),
        )
        .await
        .map_err(map_err)?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

// --- Approvals ---

#[tauri::command]
pub fn tracer_approval_list_pending(
    state: State<'_, PlaneState>,
    session_id: String,
) -> Result<Value, String> {
    let approvals = state
        .plane
        .approval_list_pending(&session_id)
        .map_err(map_err)?;
    Ok(json!({ "approvals": approvals }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalResolveArgs {
    pub session_id: String,
    pub approval_id: String,
    pub decision: String,
    pub reason: Option<String>,
}

#[tauri::command]
pub async fn tracer_approval_resolve(
    state: State<'_, PlaneState>,
    args: ApprovalResolveArgs,
) -> Result<Value, String> {
    state
        .plane
        .approval_resolve(
            &args.session_id,
            &args.approval_id,
            &args.decision,
            args.reason,
        )
        .await
        .map_err(map_err)
}

// --- Runtime ---

#[tauri::command]
pub fn tracer_runtime_status(
    state: State<'_, PlaneState>,
    session_id: Option<String>,
) -> Result<Value, String> {
    let processes = state
        .plane
        .runtime_status(session_id.as_deref())
        .map_err(map_err)?;
    Ok(json!({ "processes": processes }))
}
