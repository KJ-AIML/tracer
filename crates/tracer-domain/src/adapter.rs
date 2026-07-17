//! Optional adapter metadata on envelopes (runtime correlation + vendor preserve).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Adapter metadata attached to a normalized envelope.
///
/// Runtime-native identifiers live only here (or in payload fields explicitly
/// marked runtime-native). Unknown vendor fields are preserved in `extensions`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AdapterMetadata {
    /// Runtime kind (e.g. `acp-stdio`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_kind: Option<String>,
    /// Runtime-native session id (not a Tracer primary key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_session_id: Option<String>,
    /// Opaque or truncated reference to raw frame storage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_ref: Option<String>,
    /// Size-bounded raw fragment for debugging (never secrets).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_fragment: Option<Value>,
    /// Runtime method / notification name when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_method: Option<String>,
    /// Vendor / unknown keys preserved without product branching.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extensions: Map<String, Value>,
}

impl AdapterMetadata {
    /// Construct with runtime kind only.
    pub fn kind(runtime_kind: impl Into<String>) -> Self {
        Self {
            runtime_kind: Some(runtime_kind.into()),
            ..Self::default()
        }
    }

    /// Insert an extension key (unknown vendor metadata).
    pub fn with_extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }

    /// Preserve unknown fields from a raw object into `extensions`.
    pub fn preserve_unknown_from(mut self, raw: &Map<String, Value>) -> Self {
        const KNOWN: &[&str] = &[
            "runtimeKind",
            "runtimeSessionId",
            "rawRef",
            "rawFragment",
            "runtimeMethod",
            "extensions",
        ];
        for (k, v) in raw {
            if !KNOWN.contains(&k.as_str()) {
                self.extensions.insert(k.clone(), v.clone());
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn vendor_extensions_round_trip() {
        let meta = AdapterMetadata::kind("acp-stdio")
            .with_extension("x.ai/traceId", json!("abc"))
            .with_extension("vendorPayload", json!({ "nested": 1 }));
        let v = serde_json::to_value(&meta).unwrap();
        assert_eq!(v["runtimeKind"], "acp-stdio");
        assert_eq!(v["extensions"]["x.ai/traceId"], "abc");
        let back: AdapterMetadata = serde_json::from_value(v).unwrap();
        assert_eq!(back, meta);
    }
}
