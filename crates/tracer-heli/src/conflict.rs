//! Advisory path-claim overlap detection (read-only).

use crate::types::TaskRecord;

/// Severity of a detected path-claim conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSeverity {
    /// Overlap on paths marked shared — coordinate carefully.
    Low,
    /// Hard owns/owns or forbidden/owns overlap.
    High,
}

/// One advisory path-claim conflict between two tasks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathClaimConflict {
    /// Severity.
    pub severity: ConflictSeverity,
    /// First task id.
    pub task_a: String,
    /// Second task id.
    pub task_b: String,
    /// Claim from task A.
    pub claim_a: String,
    /// Claim from task B.
    pub claim_b: String,
    /// Conflict kind.
    pub kind: ConflictKind,
    /// Human recommendation.
    pub recommendation: String,
    /// Whether both sides mark the path shared.
    pub explicitly_shared: bool,
}

/// Kind of path-claim conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Two exclusive owns claims overlap.
    OwnsOwns,
    /// One task forbids a path another owns.
    ForbiddenOwns,
}

/// Detect advisory path-claim overlaps among active tasks.
///
/// If `focus_task_id` is set, only conflicts involving that task are returned.
pub fn detect_path_claim_conflicts(
    tasks: &[TaskRecord],
    focus_task_id: Option<&str>,
) -> Vec<PathClaimConflict> {
    let active: Vec<&TaskRecord> = tasks.iter().filter(|t| t.is_active()).collect();
    let mut conflicts = Vec::new();

    for i in 0..active.len() {
        for j in (i + 1)..active.len() {
            let a = active[i];
            let b = active[j];
            if let Some(focus) = focus_task_id {
                if a.task_id != focus && b.task_id != focus {
                    continue;
                }
            }

            let a_shared: std::collections::HashSet<&str> =
                a.path_claims.shared.iter().map(String::as_str).collect();
            let b_shared: std::collections::HashSet<&str> =
                b.path_claims.shared.iter().map(String::as_str).collect();

            for pa in &a.path_claims.owns {
                for pb in &b.path_claims.owns {
                    if !paths_overlap(pa, pb) {
                        continue;
                    }
                    let explicitly_shared = a_shared.contains(pa.as_str())
                        || a_shared.contains(pb.as_str())
                        || b_shared.contains(pa.as_str())
                        || b_shared.contains(pb.as_str());
                    conflicts.push(PathClaimConflict {
                        severity: if explicitly_shared {
                            ConflictSeverity::Low
                        } else {
                            ConflictSeverity::High
                        },
                        task_a: a.task_id.clone(),
                        task_b: b.task_id.clone(),
                        claim_a: pa.clone(),
                        claim_b: pb.clone(),
                        kind: ConflictKind::OwnsOwns,
                        recommendation: if explicitly_shared {
                            "paths marked shared — coordinate carefully".into()
                        } else {
                            format!(
                                "Mark shared or assign one owner for overlapping paths ({pa} vs {pb})"
                            )
                        },
                        explicitly_shared,
                    });
                }
            }

            for forbidden in &a.path_claims.forbidden {
                for pb in &b.path_claims.owns {
                    if paths_overlap(forbidden, pb) {
                        conflicts.push(PathClaimConflict {
                            severity: ConflictSeverity::High,
                            task_a: a.task_id.clone(),
                            task_b: b.task_id.clone(),
                            claim_a: forbidden.clone(),
                            claim_b: pb.clone(),
                            kind: ConflictKind::ForbiddenOwns,
                            recommendation: format!(
                                "Task {} owns path forbidden by {}",
                                b.task_id, a.task_id
                            ),
                            explicitly_shared: false,
                        });
                    }
                }
            }

            for forbidden in &b.path_claims.forbidden {
                for pa in &a.path_claims.owns {
                    if paths_overlap(forbidden, pa) {
                        conflicts.push(PathClaimConflict {
                            severity: ConflictSeverity::High,
                            task_a: b.task_id.clone(),
                            task_b: a.task_id.clone(),
                            claim_a: forbidden.clone(),
                            claim_b: pa.clone(),
                            kind: ConflictKind::ForbiddenOwns,
                            recommendation: format!(
                                "Task {} owns path forbidden by {}",
                                a.task_id, b.task_id
                            ),
                            explicitly_shared: false,
                        });
                    }
                }
            }
        }
    }

    conflicts
}

fn paths_overlap(a: &str, b: &str) -> bool {
    let pa = a.replace('\\', "/");
    let pb = b.replace('\\', "/");
    if pa.is_empty() || pb.is_empty() {
        return false;
    }
    if pa == pb {
        return true;
    }
    if match_glob(&pa, &pb) || match_glob(&pb, &pa) {
        return true;
    }
    let na = pa.trim_end_matches("/**").trim_end_matches("**");
    let nb = pb.trim_end_matches("/**").trim_end_matches("**");
    if !na.is_empty()
        && !nb.is_empty()
        && (na == nb || na.starts_with(&format!("{nb}/")) || nb.starts_with(&format!("{na}/")))
    {
        return true;
    }
    false
}

/// Minimal glob: `**` and `*` only; path uses `/`.
fn match_glob(pattern: &str, file_path: &str) -> bool {
    let mut re = String::from("^");
    let p: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < p.len() {
        let c = p[i];
        if c == '*' && i + 1 < p.len() && p[i + 1] == '*' {
            re.push_str(".*");
            i += 2;
            if i < p.len() && p[i] == '/' {
                i += 1;
            }
        } else if c == '*' {
            re.push_str("[^/]*");
            i += 1;
        } else if ".+^${}()|[]\\".contains(c) {
            re.push('\\');
            re.push(c);
            i += 1;
        } else {
            re.push(c);
            i += 1;
        }
    }
    re.push('$');
    regex_is_match(&re, file_path)
}

/// Tiny anchored matcher avoiding a regex crate dependency.
fn regex_is_match(pattern: &str, text: &str) -> bool {
    // Convert our limited regex to a recursive glob-style match instead.
    // `pattern` was built with ^...$ — strip anchors and interpret.
    let inner = pattern
        .strip_prefix('^')
        .and_then(|s| s.strip_suffix('$'))
        .unwrap_or(pattern);
    match_regex_limited(inner, text)
}

fn match_regex_limited(pat: &str, text: &str) -> bool {
    // Interpret: `.*` , `[^/]*`, and escaped literals / normal chars.
    fn rec(pat: &[u8], text: &[u8]) -> bool {
        if pat.is_empty() {
            return text.is_empty();
        }
        // `.*`
        if pat.starts_with(b".*") {
            // greedy: try consuming 0..len
            for k in 0..=text.len() {
                if rec(&pat[2..], &text[k..]) {
                    return true;
                }
            }
            return false;
        }
        // `[^/]*`
        if pat.starts_with(b"[^/]*") {
            let mut i = 0;
            loop {
                if rec(&pat[5..], &text[i..]) {
                    return true;
                }
                if i >= text.len() || text[i] == b'/' {
                    break;
                }
                i += 1;
            }
            return false;
        }
        // escaped char `\.` etc.
        if pat[0] == b'\\' && pat.len() >= 2 {
            if text.is_empty() || text[0] != pat[1] {
                return false;
            }
            return rec(&pat[2..], &text[1..]);
        }
        if text.is_empty() || text[0] != pat[0] {
            return false;
        }
        rec(&pat[1..], &text[1..])
    }
    rec(pat.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PathClaims, TaskRecord, TaskSource, TaskTarget, YoloFlag};

    fn task(id: &str, owns: &[&str], forbidden: &[&str], shared: &[&str]) -> TaskRecord {
        TaskRecord {
            schema_version: 1,
            task_id: id.into(),
            title: None,
            status: Some("active".into()),
            program_id: None,
            parent_task_id: None,
            source: TaskSource::default(),
            target: TaskTarget::default(),
            mode: Some("strict".into()),
            revision: Some(1),
            path_claims: PathClaims {
                owns: owns.iter().map(|s| (*s).into()).collect(),
                reads: vec![],
                shared: shared.iter().map(|s| (*s).into()).collect(),
                forbidden: forbidden.iter().map(|s| (*s).into()).collect(),
            },
            yolo: YoloFlag::default(),
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn detects_owns_overlap() {
        let tasks = vec![
            task("a", &["docs/**"], &[], &[]),
            task("b", &["docs/contracts/**"], &[], &[]),
        ];
        let c = detect_path_claim_conflicts(&tasks, None);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].kind, ConflictKind::OwnsOwns);
        assert_eq!(c[0].severity, ConflictSeverity::High);
    }

    #[test]
    fn no_conflict_when_disjoint() {
        let tasks = vec![
            task("a", &["docs/ux/**"], &[], &[]),
            task("b", &["docs/testing/**"], &[], &[]),
        ];
        assert!(detect_path_claim_conflicts(&tasks, None).is_empty());
    }

    #[test]
    fn shared_overlap_is_low() {
        let tasks = vec![
            task("a", &["docs/shared/**"], &[], &["docs/shared/**"]),
            task("b", &["docs/shared/**"], &[], &["docs/shared/**"]),
        ];
        let c = detect_path_claim_conflicts(&tasks, None);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].severity, ConflictSeverity::Low);
        assert!(c[0].explicitly_shared);
    }
}
