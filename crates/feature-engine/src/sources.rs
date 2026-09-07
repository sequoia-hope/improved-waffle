//! Document-level source content store (`specs/waffle_v4_document_model.md`
//! §2.3). A `.waffle` v4 document lists the external content it depends on
//! in a `sources` table; the *bytes* of each source reach the engine through
//! this store — from the file's own `embed` cache, or supplied at runtime by
//! the host after it fetched the source through its locator.
//!
//! The store is document-scoped, not feature-tree-scoped: it survives tab
//! switches and is not part of undo history (sources are assets, not edits).

use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Content bytes for the sources a document references, keyed by source id.
#[derive(Debug, Clone, Default)]
pub struct SourceStore {
    contents: HashMap<Uuid, Arc<[u8]>>,
}

impl SourceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) the bytes for a source id.
    pub fn insert(&mut self, id: Uuid, bytes: impl Into<Arc<[u8]>>) {
        self.contents.insert(id, bytes.into());
    }

    /// Convenience for text sources (STEP, `.kicad_pcb`, `.waffle`).
    pub fn insert_text(&mut self, id: Uuid, text: &str) {
        self.insert(id, Arc::<[u8]>::from(text.as_bytes()));
    }

    pub fn get(&self, id: Uuid) -> Option<&Arc<[u8]>> {
        self.contents.get(&id)
    }

    /// The content as text (lossy UTF-8; every Phase-1 source kind is text).
    pub fn text(&self, id: Uuid) -> Option<String> {
        self.contents
            .get(&id)
            .map(|b| String::from_utf8_lossy(b).into_owned())
    }

    pub fn contains(&self, id: Uuid) -> bool {
        self.contents.contains_key(&id)
    }

    pub fn remove(&mut self, id: Uuid) -> Option<Arc<[u8]>> {
        self.contents.remove(&id)
    }

    pub fn ids(&self) -> impl Iterator<Item = Uuid> + '_ {
        self.contents.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.contents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    pub fn clear(&mut self) {
        self.contents.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_round_trips_text_and_bytes() {
        let mut store = SourceStore::new();
        let id = Uuid::new_v4();
        assert!(!store.contains(id));
        store.insert_text(id, "ISO-10303-21;");
        assert!(store.contains(id));
        assert_eq!(store.text(id).as_deref(), Some("ISO-10303-21;"));
        assert_eq!(store.get(id).map(|b| b.len()), Some(13));
        assert_eq!(store.len(), 1);
        assert!(store.remove(id).is_some());
        assert!(store.is_empty());
    }
}
