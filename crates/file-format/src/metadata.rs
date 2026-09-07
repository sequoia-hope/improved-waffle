use chrono::{DateTime, Utc};
use feature_engine::types::FeatureTree;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::sources::known_or_unknown;

/// Project metadata for the single-tree API (`save_project`/`load_project`).
/// Kept for the assay generators and other consumers that deal in one
/// feature tree; the document model is [`DocumentMetadata`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Human-readable project name.
    pub name: String,
    /// When the project was first created.
    pub created: DateTime<Utc>,
    /// When the project was last modified.
    pub modified: DateTime<Utc>,
    /// Display unit preference (mm, cm, m, in, ft). None for legacy v1 files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_unit: Option<String>,
}

impl ProjectMetadata {
    /// Create metadata with the given name and current timestamp.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            created: now,
            modified: now,
            display_unit: None,
        }
    }

    /// Create metadata with a display unit preference.
    pub fn with_display_unit(mut self, unit: impl Into<String>) -> Self {
        self.display_unit = Some(unit.into());
        self
    }
}

impl From<&DocumentMetadata> for ProjectMetadata {
    fn from(d: &DocumentMetadata) -> Self {
        ProjectMetadata {
            name: d.name.clone(),
            created: d.created,
            modified: d.modified,
            display_unit: d.display_unit.clone(),
        }
    }
}

/// Document-level metadata (v3+; `id` since v4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Stable identity of the document (v4 §2.1): survives rename, provider
    /// move and fork. Writers always emit it; a reader that finds it absent
    /// mints one (this serde default) and the loader warns.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub name: String,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_unit: Option<String>,
    /// Unknown keys preserved across load → save (v4 §2.6).
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl DocumentMetadata {
    /// Fresh metadata: new id, timestamps now, no unit.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            created: now,
            modified: now,
            display_unit: None,
            extra: Map::new(),
        }
    }

    pub fn with_display_unit(mut self, unit: impl Into<String>) -> Self {
        self.display_unit = Some(unit.into());
        self
    }
}

impl From<&ProjectMetadata> for DocumentMetadata {
    fn from(m: &ProjectMetadata) -> Self {
        DocumentMetadata {
            id: Uuid::new_v4(),
            name: m.name.clone(),
            created: m.created,
            modified: m.modified,
            display_unit: m.display_unit.clone(),
            extra: Map::new(),
        }
    }
}

/// A single tab in a document.
///
/// `id` is a document-level key matched by `active_tab` and, since v4, by
/// cross-tab references. New tabs get UUID strings; the type stays a
/// free-form `String` because legacy documents carry non-UUID ids (the
/// literal `"default"`) that must keep loading — the v3→v4 migration rewrites
/// those to fresh UUIDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub name: String,
    pub kind: TabKind,
    /// Unknown keys preserved across load → save (v4 §2.6).
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Tab {
    /// A Part tab with a fresh UUID id.
    pub fn part(name: impl Into<String>, features: FeatureTree) -> Self {
        Tab {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            kind: TabKind::Part {
                features,
                preview_mesh: None,
            },
            extra: Map::new(),
        }
    }

    /// The tab's feature tree, if it is a Part.
    pub fn features(&self) -> Option<&FeatureTree> {
        match &self.kind {
            TabKind::Part { features, .. } => Some(features),
            TabKind::Unknown(_) => None,
        }
    }

    pub fn features_mut(&mut self) -> Option<&mut FeatureTree> {
        match &mut self.kind {
            TabKind::Part { features, .. } => Some(features),
            TabKind::Unknown(_) => None,
        }
    }
}

/// The kind/content of a tab.
///
/// Known kinds in v4.0: `Part`. A well-formed `{"type": …}` this reader does
/// not know (`Assembly`, `Drawing`, …) is kept as [`TabKind::Unknown`] and
/// re-emitted verbatim (v4 §2.5), so adding a tab kind is not a
/// `MIN_READER_VERSION` bump. A malformed known kind is still a parse error.
// One TabKind lives per tab, not per vertex; boxing the FeatureTree to shrink
// the Unknown variant's discriminant would touch every `TabKind::Part` site
// for no measurable gain (same call as `Operation` in feature-engine).
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum TabKind {
    Part {
        features: FeatureTree,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preview_mesh: Option<PreviewMesh>,
    },
    #[serde(untagged)]
    Unknown(Value),
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum KnownTabKind {
    Part {
        features: FeatureTree,
        #[serde(default)]
        preview_mesh: Option<PreviewMesh>,
    },
}

const TAB_KIND_TAGS: &[&str] = &["Part"];

impl<'de> Deserialize<'de> for TabKind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(
            match known_or_unknown::<D, KnownTabKind>(d, TAB_KIND_TAGS, "tab kind")? {
                Ok(KnownTabKind::Part {
                    features,
                    preview_mesh,
                }) => TabKind::Part {
                    features,
                    preview_mesh,
                },
                Err(v) => TabKind::Unknown(v),
            },
        )
    }
}

impl TabKind {
    /// The `type` tag as written in the file.
    pub fn type_tag(&self) -> &str {
        match self {
            TabKind::Part { .. } => "Part",
            TabKind::Unknown(v) => v.get("type").and_then(Value::as_str).unwrap_or("?"),
        }
    }
}

/// A lightweight mesh for 3D thumbnail previews.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewMesh {
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
}
