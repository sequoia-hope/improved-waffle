use feature_engine::types::FeatureTree;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::document::{LoadedDocument, WaffleDocument};
use crate::errors::LoadError;
use crate::metadata::{DocumentMetadata, ProjectMetadata, Tab, TabKind};
use crate::save::FORMAT_VERSION;
use crate::sources::SourceEntry;

/// The top-level v2 file structure for deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct WaffleFileRaw {
    pub format: String,
    pub version: u32,
    pub project: ProjectMetadata,
    pub features: FeatureTree,
}

/// V3 file structure for deserialization.
#[derive(Debug, Clone, Deserialize)]
struct WaffleFileV3Raw {
    pub document: DocumentMetadata,
    pub tabs: Vec<Tab>,
    pub active_tab: String,
}

/// V4 file structure for deserialization.
#[derive(Debug, Clone, Deserialize)]
struct WaffleFileV4Raw {
    #[allow(dead_code)]
    pub format: String,
    #[allow(dead_code)]
    pub version: u32,
    #[allow(dead_code)]
    #[serde(default)]
    pub min_reader_version: u32,
    pub document: DocumentMetadata,
    #[serde(default)]
    pub sources: Vec<SourceEntry>,
    pub tabs: Vec<Tab>,
    pub active_tab: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Shared envelope validation: `format` identifier, `version`, and
/// `min_reader_version` (absent in pre-2026-08-28 files ⇒ 0 ⇒ passes). A file
/// that requires a newer reader fails with a clean [`LoadError::FutureVersion`]
/// instead of a raw serde parse error further down.
fn check_envelope(value: &Value) -> Result<u32, LoadError> {
    if value.get("format").and_then(|f| f.as_str()) != Some("waffle-iron") {
        let fmt = value
            .get("format")
            .and_then(|f| f.as_str())
            .unwrap_or("unknown")
            .to_string();
        return Err(LoadError::UnknownFormat(fmt));
    }

    let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let min_reader = value
        .get("min_reader_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let required = version.max(min_reader);
    if required > FORMAT_VERSION {
        return Err(LoadError::FutureVersion {
            file_version: required,
            supported_version: FORMAT_VERSION,
        });
    }
    Ok(version)
}

fn parse_err(e: impl std::fmt::Display) -> LoadError {
    LoadError::ParseError(e.to_string())
}

/// Load any supported version (v1–v4) as a v4 document. Older files are
/// migrated on the way in (v1→v2 value scaling, v2→v3 tab wrapping, v3→v4
/// identity/sources — `crate::migrate`). Non-fatal findings come back as
/// warnings.
pub fn load_document(json: &str) -> Result<LoadedDocument, LoadError> {
    let value: Value = serde_json::from_str(json).map_err(parse_err)?;
    let version = check_envelope(&value)?;

    let mut warnings = Vec::new();
    let document = if version >= 4 && value.get("tabs").is_some() {
        let id_missing = value.get("document").and_then(|d| d.get("id")).is_none();
        let raw: WaffleFileV4Raw = serde_json::from_value(value).map_err(parse_err)?;
        if id_missing {
            warnings.push(format!(
                "document.id was absent; minted {} (writers must emit it)",
                raw.document.id
            ));
        }
        WaffleDocument {
            document: raw.document,
            sources: raw.sources,
            tabs: raw.tabs,
            active_tab: raw.active_tab,
            extra: raw.extra,
        }
    } else if version >= 3 && value.get("tabs").is_some() {
        let raw: WaffleFileV3Raw = serde_json::from_value(value).map_err(parse_err)?;
        let (doc, w) = crate::migrate::migrate_v3_to_v4(raw.document, raw.tabs, raw.active_tab);
        warnings.extend(w);
        doc
    } else {
        // v1/v2 flat shape → wrap into one tab (v3 shape) → v4.
        let (tree, meta) = load_project_from_value(value)?;
        let document = DocumentMetadata::from(&meta);
        let tab = Tab::part("Part 1", tree);
        let active = tab.id.clone();
        let (doc, w) = crate::migrate::migrate_v3_to_v4(document, vec![tab], active);
        warnings.extend(w);
        doc
    };

    warnings.extend(document.validate()?);
    Ok(LoadedDocument { document, warnings })
}

/// Single-tree API: the active tab's feature tree (falling back to the first
/// tab) with any packed source content inlined as legacy payloads, plus
/// project metadata. Handles v1–v4.
pub fn load_project(json: &str) -> Result<(FeatureTree, ProjectMetadata), LoadError> {
    let loaded = load_document(json)?;
    let doc = loaded.document;
    let tab = doc
        .active_tab()
        .or_else(|| doc.tabs.first())
        .ok_or_else(|| LoadError::ParseError("no tabs in document".to_string()))?;
    let mut tree = match &tab.kind {
        TabKind::Part { features, .. } => features.clone(),
        TabKind::Assembly { .. } | TabKind::Unknown(_) => {
            return Err(LoadError::ParseError(format!(
                "active tab `{}` has kind `{}`, which cannot be opened as a part",
                tab.name,
                tab.kind.type_tag()
            )))
        }
    };
    let _ = doc.inline_payloads_into(&mut tree);
    Ok((tree, ProjectMetadata::from(&doc.document)))
}

/// Internal: load a v1/v2 project from a pre-parsed JSON value.
fn load_project_from_value(value: Value) -> Result<(FeatureTree, ProjectMetadata), LoadError> {
    let raw: WaffleFileRaw = serde_json::from_value(value).map_err(parse_err)?;

    // Validate format identifier
    if raw.format != "waffle-iron" {
        return Err(LoadError::UnknownFormat(raw.format));
    }

    // Apply migrations if needed (version < current, but only content migrations up to v2)
    let tree = if raw.version < 2 {
        crate::migrate::migrate(raw.features, raw.version, 2)?
    } else {
        raw.features
    };

    Ok((tree, raw.project))
}
