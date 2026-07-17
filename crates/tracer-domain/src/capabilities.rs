//! Negotiated runtime capability set (Tracer view).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Tracer capability keys from `RUNTIME_ADAPTER_CONTRACT_V1.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Capabilities {
    /// Message/progress deltas will stream.
    pub prompt_streaming: bool,
    /// Runtime supports cancel without killing process.
    pub cancellation: bool,
    /// Plan snapshots/patches available.
    pub plan_updates: bool,
    /// Tool start/update/complete available.
    pub tool_calls: bool,
    /// Permission requests will be emitted.
    pub approvals: bool,
    /// File change events available.
    pub file_change_notifications: bool,
    /// Terminal stream events available.
    pub terminal_output: bool,
    /// Runtime can resume prior runtime session ids.
    pub session_resume: bool,
    /// Unknown vendor capability keys preserved for debugging (not for product branching).
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub unknown: Map<String, Value>,
}

impl Capabilities {
    /// Empty / all-false set.
    pub fn none() -> Self {
        Self::default()
    }

    /// Minimum viable slice defaults: streaming on; others off.
    pub fn slice_minimum() -> Self {
        Self {
            prompt_streaming: true,
            ..Self::default()
        }
    }

    /// Full feature set (e.g. rich fake runtime).
    pub fn all_enabled() -> Self {
        Self {
            prompt_streaming: true,
            cancellation: true,
            plan_updates: true,
            tool_calls: true,
            approvals: true,
            file_change_notifications: true,
            terminal_output: true,
            session_resume: true,
            unknown: Map::new(),
        }
    }

    /// Merge unknown vendor keys from a raw JSON object without dropping them.
    pub fn preserve_unknown_from(&mut self, raw: &Map<String, Value>) {
        const KNOWN: &[&str] = &[
            "promptStreaming",
            "cancellation",
            "planUpdates",
            "toolCalls",
            "approvals",
            "fileChangeNotifications",
            "terminalOutput",
            "sessionResume",
            "unknown",
        ];
        for (k, v) in raw {
            if !KNOWN.contains(&k.as_str()) {
                self.unknown.insert(k.clone(), v.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn capabilities_camel_case_round_trip() {
        let caps = Capabilities {
            prompt_streaming: true,
            cancellation: true,
            plan_updates: false,
            tool_calls: true,
            approvals: true,
            file_change_notifications: false,
            terminal_output: false,
            session_resume: false,
            unknown: Map::new(),
        };
        let v = serde_json::to_value(&caps).unwrap();
        assert_eq!(v["promptStreaming"], true);
        assert_eq!(v["planUpdates"], false);
        let back: Capabilities = serde_json::from_value(v).unwrap();
        assert_eq!(back, caps);
    }

    #[test]
    fn unknown_vendor_caps_preserved() {
        let raw = json!({
            "promptStreaming": true,
            "cancellation": false,
            "x.ai/experimental": true,
            "vendorFeature": { "level": 2 }
        });
        let obj = raw.as_object().unwrap();
        let mut caps: Capabilities = serde_json::from_value(raw.clone()).unwrap();
        // serde will ignore unknown at root — re-preserve explicitly
        caps.preserve_unknown_from(obj);
        assert!(caps.prompt_streaming);
        assert!(!caps.cancellation);
        assert_eq!(caps.unknown.get("x.ai/experimental"), Some(&json!(true)));
        assert_eq!(caps.unknown.get("vendorFeature"), Some(&json!({ "level": 2 })));
    }
}