//! Redact secrets and machine-local noise from evidence.

use serde_json::{Map, Value};
use std::path::Path;

/// Patterns that indicate secret material (case-insensitive substring).
const SECRET_KEY_NEEDLES: &[&str] = &[
    "token",
    "authorization",
    "api_key",
    "apikey",
    "api-key",
    "secret",
    "password",
    "passwd",
    "credential",
    "cookie",
    "session_key",
    "access_key",
    "refresh_token",
    "bearer",
    "x-api-key",
    "private_key",
];

const REDACTED: &str = "[REDACTED]";

/// Sanitize free text: redact obvious bearer tokens / key assignments.
pub fn sanitize_text(input: &str) -> String {
    let mut out = input.to_string();

    // Bearer tokens
    out = redact_regex_like(&out, "bearer ");
    // sk- / xai- style prefixes (common API key shapes) — keep prefix class only
    out = redact_prefixed_keys(&out);

    // Absolute Windows user paths → generic
    out = scrub_user_paths(&out);
    out
}

fn redact_regex_like(input: &str, prefix: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();
    let lower_bytes = lower.as_bytes();
    let p = prefix.as_bytes();

    while i < bytes.len() {
        if i + p.len() <= lower_bytes.len() && &lower_bytes[i..i + p.len()] == p {
            result.push_str(&input[i..i + p.len()]);
            i += p.len();
            // skip token chars
            let start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'"' {
                i += 1;
            }
            if i > start {
                result.push_str(REDACTED);
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

fn redact_prefixed_keys(input: &str) -> String {
    // Replace long tokens that look like sk-... or xai-... keys
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        // detect sk- or xai-
        if c == 's' || c == 'S' {
            let mut buf = String::from(c);
            if matches!(chars.peek(), Some('k') | Some('K')) {
                buf.push(chars.next().unwrap());
                if matches!(chars.peek(), Some('-')) {
                    buf.push(chars.next().unwrap());
                    // consume secret body
                    let mut body = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                            body.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if body.len() >= 8 {
                        out.push_str("sk-");
                        out.push_str(REDACTED);
                        continue;
                    } else {
                        out.push_str(&buf);
                        out.push_str(&body);
                        continue;
                    }
                }
            }
            out.push_str(&buf);
            continue;
        }
        out.push(c);
    }
    out
}

fn scrub_user_paths(input: &str) -> String {
    // C:\Users\<name>\... or /home/<name>/... or /Users/<name>/...
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while !rest.is_empty() {
        let markers = ["\\Users\\", "/Users/", "/home/"];
        let mut best: Option<(usize, &str)> = None;
        for m in markers {
            if let Some(idx) = rest.find(m) {
                match best {
                    Some((bi, _)) if idx >= bi => {}
                    _ => best = Some((idx, m)),
                }
            }
        }
        let Some((idx, marker)) = best else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..idx + marker.len()]);
        let after_marker = &rest[idx + marker.len()..];
        // Already scrubbed?
        if after_marker.starts_with("<user>") {
            out.push_str("<user>");
            rest = &after_marker["<user>".len()..];
            continue;
        }
        let end_rel = after_marker
            .find(|c: char| c == '\\' || c == '/' || c.is_whitespace() || c == '"')
            .unwrap_or(after_marker.len());
        if end_rel == 0 {
            // No username segment; emit marker only and advance one char to avoid loop.
            rest = &rest[idx + marker.len()..];
            if rest.is_empty() {
                break;
            }
            // Consume one char so we cannot re-find the same marker forever.
            let mut chars = rest.chars();
            if let Some(c) = chars.next() {
                out.push(c);
            }
            rest = chars.as_str();
            continue;
        }
        out.push_str("<user>");
        rest = &after_marker[end_rel..];
    }
    out
}

/// Recursively sanitize JSON values for evidence persistence.
pub fn sanitize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                if key_looks_secret(k) {
                    out.insert(k.clone(), Value::String(REDACTED.into()));
                } else {
                    out.insert(k.clone(), sanitize_json(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(sanitize_json).collect()),
        Value::String(s) => Value::String(sanitize_text(s)),
        other => other.clone(),
    }
}

fn key_looks_secret(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SECRET_KEY_NEEDLES.iter().any(|n| lower.contains(n))
}

/// Path for evidence — prefer relative display under repo root.
pub fn display_path(path: &Path, repo_root: Option<&Path>) -> String {
    if let Some(root) = repo_root {
        if let Ok(rel) = path.strip_prefix(root) {
            return rel.display().to_string().replace('\\', "/");
        }
    }
    sanitize_text(&path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_bearer() {
        let s = sanitize_text("Authorization: Bearer abcdef1234567890");
        assert!(s.contains(REDACTED));
        assert!(!s.contains("abcdef1234567890"));
    }

    #[test]
    fn redacts_sk_keys() {
        let s = sanitize_text("key=sk-abcdefghijklmnopqrstuv");
        assert!(s.contains(REDACTED));
        assert!(!s.contains("abcdefghijklmnopqrstuv"));
    }

    #[test]
    fn redacts_json_secret_keys() {
        let v = json!({
            "authMethods": [{"id": "xai.api_key"}],
            "access_token": "super-secret",
            "message": "Authentication required"
        });
        let s = sanitize_json(&v);
        assert_eq!(s["access_token"], REDACTED);
        assert_eq!(s["message"], "Authentication required");
    }

    #[test]
    fn scrubs_user_paths() {
        let s = sanitize_text(r"C:\Users\Alice\project\file.txt");
        assert!(s.contains("<user>"));
        assert!(!s.contains("Alice"));
    }
}
