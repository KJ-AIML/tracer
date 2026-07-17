//! Deterministic fixture tests for tracer-heli read-only status adapter.
//!
//! Fixtures are copied into a temporary directory so upward discovery does not
//! accidentally attach to the developer parent HeliHarness workspace.

use std::fs;
use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use tracer_heli::{
    find_workspace_root, load_workspace_status, try_load_workspace_status, LeaseState,
    WorkspaceMode, WorkspaceProbe, WorktreeSource,
};

fn fixture_src(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copy a fixture tree into a fresh temp dir (outside the parent heli workspace).
fn materialize(name: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dest = tmp.path().join(name);
    copy_dir_all(&fixture_src(name), &dest).expect("copy fixture");
    (tmp, dest)
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

#[test]
fn probe_missing_workspace_is_safe() {
    let (tmp, root) = materialize("no_workspace");
    let start = root.join("nested");
    let probe = WorkspaceProbe::probe(&start);
    assert!(
        !probe.is_found(),
        "temp fixture must not see parent heli; probe={probe:?} tmp={:?}",
        tmp.path()
    );

    let opt = try_load_workspace_status(&start).expect("io ok");
    assert!(opt.is_none());

    let err = load_workspace_status(&start).unwrap_err();
    assert!(err.is_workspace_not_found());
}

#[test]
fn discovers_workspace_from_nested_worktree() {
    let (_tmp, root) = materialize("minimal_workspace");
    let nested = root.join("repos").join("worktrees").join("agent-a");
    let found = find_workspace_root(&nested).expect("found");
    let expected = find_workspace_root(&root).unwrap();
    assert_eq!(found, expected);
}

#[test]
fn loads_minimal_workspace_status() {
    let (_tmp, root) = materialize("minimal_workspace");
    let start = root.join("repos").join("worktrees").join("agent-a");
    let status = load_workspace_status(&start).expect("status");

    assert_eq!(status.mode, WorkspaceMode::Concurrent);
    assert_eq!(status.harness_version.as_deref(), Some("0.5.24"));
    assert_eq!(status.default_target_repo(), Some("tracer"));

    let index = status.index.as_ref().expect("index");
    assert_eq!(index.repos.len(), 2);
    assert!(index
        .repos
        .iter()
        .any(|r| r.name == "tracer" && r.default_target));

    assert_eq!(status.tasks.len(), 2);
    assert!(status
        .sessions
        .iter()
        .any(|s| s.session_id == "heli-ses-demo-writer"));

    let demo = status.task_view("tracer-w1-demo").expect("demo view");
    assert_eq!(demo.writer, "heli-ses-demo-writer");
    assert_eq!(demo.lease_state, LeaseState::Active);
    assert_eq!(demo.worktree_source, WorktreeSource::WriteLease);
    assert!(
        demo.worktree.contains("demo-worktree"),
        "worktree={}",
        demo.worktree
    );
    assert_eq!(demo.observer_count, 1);
    assert_eq!(demo.repo, "tracer");
    assert!(
        demo.warnings
            .iter()
            .any(|w| w.contains("task metadata worktree")),
        "warnings={:?}",
        demo.warnings
    );

    let binding = status.binding_for_task("tracer-w1-demo").expect("binding");
    assert_eq!(
        binding.default_session_id.as_deref(),
        Some("heli-ses-demo-writer")
    );
    assert!(binding.host_bindings.contains_key("grok-build"));

    let other = status.task_view("tracer-w1-other").expect("other");
    assert_eq!(other.lease_state, LeaseState::None);
    assert_eq!(other.writer, "none");
}

#[test]
fn path_claim_forbidden_overlap_detected() {
    let (_tmp, root) = materialize("minimal_workspace");
    let status = load_workspace_status(&root).expect("status");
    assert!(
        !status.conflicts.is_empty(),
        "expected path-claim conflicts, got none"
    );
    assert!(status.conflicts.iter().any(|c| {
        (c.task_a == "tracer-w1-other" || c.task_b == "tracer-w1-other")
            && (c.claim_a.contains("agent-workflows") || c.claim_b.contains("agent-workflows"))
    }));
}

#[test]
fn stale_lease_projection() {
    let (_tmp, root) = materialize("stale_lease_workspace");
    let status = load_workspace_status(&root).expect("status");
    let view = status.task_view("stale-task").expect("view");
    assert_eq!(view.lease_state, LeaseState::Stale);
    assert_eq!(view.worktree_source, WorktreeSource::StaleLease);
    assert!(view.worktree.contains("stale-wt"));
    assert!(view.warnings.iter().any(|w| w.contains("stale")));
}
