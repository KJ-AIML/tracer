//! Tracer desktop Tauri bootstrap with W1-F control plane commands + W2-B E2E hooks.
//!
//! Commands are thin glue over `tracer-control-plane`. No raw ACP, no direct
//! SQLite from handlers, no process management outside the control plane.
//!
//! E2E / harness may import [`control_plane`] and [`commands::plane_*`] handlers
//! without launching the WebView — same composition path as the real app.

pub mod commands;
pub mod control_plane;

use std::sync::Arc;

use commands::PlaneState;
use control_plane::build_control_plane;

/// Re-export registered command names for harness / contract checks.
pub use commands::REGISTERED_COMMANDS;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // L3-J: load temp DB / fake ACP / heli probe from --tracer-e2e-env=file when present.
    control_plane::apply_e2e_env_from_cli();

    // Tokio runtime for async control plane open before tauri loop.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let db_path = control_plane::resolve_database_path_for_e2e(None);
    let plane = rt
        .block_on(build_control_plane(db_path))
        .unwrap_or_else(|e| {
            eprintln!("control plane open failed (shell will still start): {e}");
            // Fallback: try in-memory again so commands can report StorageError paths.
            rt.block_on(async {
                build_control_plane(None)
                    .await
                    .expect("in-memory control plane")
            })
        });

    // Test-only readiness marker for L3-J harness (env TRACER_E2E_READY_MARKER).
    control_plane::write_e2e_ready_marker();

    // Keep runtime alive for control plane background storage.
    std::mem::forget(rt);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(PlaneState {
            plane: Arc::clone(&plane),
        })
        .invoke_handler(tauri::generate_handler![
            commands::tracer_app_info,
            commands::tracer_presentation_snapshot,
            commands::tracer_presentation_focus,
            commands::tracer_heli_status,
            commands::tracer_e2e_env,
            commands::tracer_project_list,
            commands::tracer_project_register,
            commands::tracer_project_get,
            commands::tracer_session_list,
            commands::tracer_session_create,
            commands::tracer_session_get,
            commands::tracer_session_submit_prompt,
            commands::tracer_session_cancel,
            commands::tracer_session_stop,
            commands::tracer_events_list,
            commands::tracer_approval_list_pending,
            commands::tracer_approval_resolve,
            commands::tracer_runtime_status,
            app_shell_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tracer desktop");
}

/// Legacy shell stub (kept for W1-A smoke compatibility).
#[tauri::command]
fn app_shell_info() -> serde_json::Value {
    serde_json::json!({
        "name": "tracer-desktop",
        "module": "W1-F",
        "mode": "control-plane",
        "note": "tracer_* commands registered via tracer-control-plane",
        "e2eHooks": true
    })
}
