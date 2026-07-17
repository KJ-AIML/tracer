//! Tracer desktop Tauri bootstrap (W1-A).
//!
//! **Scope:** window shell only — enough for `tauri dev` / frontend hosting.
//! **Handoff to W1-F:** register `tracer_*` commands, event stream `tracer://events`,
//! compose process/adapter/storage crates. Do **not** put ACP/runtime/storage
//! business logic in this file during W1-A.
//!
//! **Migrations:** `src-tauri/migrations/` is owned by W1-E — do not add here.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![app_shell_info])
        .run(tauri::generate_context!())
        .expect("error while running Tracer desktop shell");
}

/// Temporary stub command so the shell binary exposes something invoke-able.
/// REPLACE_WHEN_W1F_CONTROL_PLANE_AVAILABLE — use `tracer_app_info` contract name.
#[tauri::command]
fn app_shell_info() -> serde_json::Value {
    serde_json::json!({
        "name": "tracer-desktop",
        "module": "W1-A",
        "mode": "shell-stub",
        "note": "Control plane commands land in W1-F (tracer_app_info, tracer_session_*, tracer://events)"
    })
}
