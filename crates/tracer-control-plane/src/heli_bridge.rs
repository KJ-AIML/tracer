//! Read-only Heli status bridge (W1-H). Missing Heli must not crash the app.

use std::path::Path;

use tracer_heli::{try_load_workspace_status, WorkspaceProbe};

use crate::types::HeliStatusView;

/// Probe Heli from `start` and build a presentation view.
///
/// Never panics; never mutates Heli state.
pub fn probe_heli(start: impl AsRef<Path>) -> HeliStatusView {
    let start = start.as_ref();
    match WorkspaceProbe::probe(start) {
        WorkspaceProbe::Missing { .. } => HeliStatusView::unavailable(
            "Heli workspace not found above probe path; runtime and history remain usable",
        ),
        WorkspaceProbe::Found { root } => match try_load_workspace_status(start) {
            Ok(Some(status)) => {
                let mode = format!("{:?}", status.mode);
                let summary = format!(
                    "Heli workspace at {} (mode={mode}, tasks={}, sessions={})",
                    root.display(),
                    status.tasks.len(),
                    status.sessions.len()
                );
                HeliStatusView {
                    available: true,
                    workspace_root: Some(root.display().to_string()),
                    mode: Some(mode),
                    summary,
                    warnings: status.warnings,
                }
            }
            Ok(None) => HeliStatusView {
                available: true,
                workspace_root: Some(root.display().to_string()),
                mode: None,
                summary: format!(
                    "Heli workspace found at {} but status load returned None",
                    root.display()
                ),
                warnings: vec!["status load empty".into()],
            },
            Err(e) => HeliStatusView {
                available: true,
                workspace_root: Some(root.display().to_string()),
                mode: None,
                summary: format!(
                    "Heli workspace found at {} but status load failed: {e}",
                    root.display()
                ),
                warnings: vec![e.to_string()],
            },
        },
    }
}
