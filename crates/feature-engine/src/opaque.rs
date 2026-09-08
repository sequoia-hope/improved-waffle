//! Opaque preservation of tagged objects a reader does not know
//! (`specs/waffle_v4_document_model.md` §2.5, Phase 1b).
//!
//! A `{"type": …}` object whose tag is unrecognized is handed back verbatim so
//! the caller can keep it (`Operation::Unknown`, file-format's
//! `TabKind::Unknown`, `SourceKind::Unknown`, `Locator::Unknown`) and re-emit
//! it byte-for-byte on save. A *malformed* object — no string `type`, or a
//! KNOWN tag whose fields do not parse — is still a hard error: opacity is
//! for the future, not for corruption.

use serde::de::{self, Deserializer};
use serde::Deserialize;
use serde_json::Value;

/// Deserialize a `{"type": …}` object as the known enum `K`, or hand back the
/// raw value when the tag is unrecognized. Malformed input is an error.
pub fn known_or_unknown<'de, D, K>(
    deserializer: D,
    known_tags: &[&str],
    what: &str,
) -> Result<Result<K, Value>, D::Error>
where
    D: Deserializer<'de>,
    K: for<'a> Deserialize<'a>,
{
    let value = Value::deserialize(deserializer)?;
    let tag = value.get("type").and_then(Value::as_str).ok_or_else(|| {
        de::Error::custom(format!("{what}: expected an object with a string `type`"))
    })?;
    if known_tags.contains(&tag) {
        serde_json::from_value::<K>(value.clone())
            .map(Ok)
            .map_err(|e| de::Error::custom(format!("{what} `{tag}`: {e}")))
    } else {
        Ok(Err(value))
    }
}

/// The `type` tag of an opaque value (`"?"` if it has none — which
/// `known_or_unknown` never produces).
pub fn type_tag(value: &Value) -> &str {
    value.get("type").and_then(Value::as_str).unwrap_or("?")
}
