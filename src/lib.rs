// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-viewer: viewer component model.
//!
//! Real implementation: viewer_items, events, validation, deterministic storage.
//! Replaces the placeholder stub.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerItem {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub subject: String,
    pub detail: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerError {
    EmptyViewerItemName,
    DuplicateViewerItem(String),
    EmptyEventSubject,
}

impl fmt::Display for ViewerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyViewerItemName => write!(f, "viewer_item name must not be empty"),
            Self::DuplicateViewerItem(id) => write!(f, "duplicate viewer_item {:?}", id),
            Self::EmptyEventSubject => write!(f, "event subject must not be empty"),
        }
    }
}

impl std::error::Error for ViewerError {}

pub struct Viewer {
    pub name: String,
    viewer_items: BTreeMap<String, ViewerItem>,
    events: BTreeMap<String, Event>,
}

impl Viewer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            viewer_items: BTreeMap::new(),
            events: BTreeMap::new(),
        }
    }

    pub fn add_viewer_item(&mut self, item: ViewerItem) -> Result<(), ViewerError> {
        if item.name.trim().is_empty() {
            return Err(ViewerError::EmptyViewerItemName);
        }
        let id = item.id.clone();
        if self.viewer_items.contains_key(&id) {
            return Err(ViewerError::DuplicateViewerItem(id.clone()));
        }
        self.viewer_items.insert(id, item);
        Ok(())
    }

    pub fn viewer_item_count(&self) -> usize {
        self.viewer_items.len()
    }

    pub fn get_viewer_item(&self, id: &str) -> Option<&ViewerItem> {
        self.viewer_items.get(id)
    }

    pub fn add_event(&mut self, event: Event) -> Result<(), ViewerError> {
        if event.subject.trim().is_empty() {
            return Err(ViewerError::EmptyEventSubject);
        }
        self.events.insert(event.id.clone(), event);
        Ok(())
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn get_event(&self, id: &str) -> Option<&Event> {
        self.events.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_viewer_item(id: &str, name: &str) -> ViewerItem {
        ViewerItem {
            id: id.into(),
            name: name.into(),
            kind: "default".into(),
            detail: String::new(),
        }
    }

    #[test]
    fn test_create() {
        let m = Viewer::new("Viewer");
        assert_eq!(m.name, "Viewer");
        assert_eq!(m.viewer_item_count(), 0);
        assert_eq!(m.event_count(), 0);
    }

    #[test]
    fn test_add_viewer_item() {
        let mut m = Viewer::new("Viewer");
        m.add_viewer_item(sample_viewer_item("a1", "Alpha")).unwrap();
        assert_eq!(m.viewer_item_count(), 1);
        assert!(m.get_viewer_item("a1").is_some());
    }

    #[test]
    fn test_empty_name_rejected() {
        let mut m = Viewer::new("Viewer");
        assert!(m.add_viewer_item(sample_viewer_item("x", "   ")).is_err());
    }

    #[test]
    fn test_duplicate_rejected() {
        let mut m = Viewer::new("Viewer");
        m.add_viewer_item(sample_viewer_item("x", "One")).unwrap();
        assert!(m.add_viewer_item(sample_viewer_item("x", "Two")).is_err());
    }

    #[test]
    fn test_event_validation() {
        let mut m = Viewer::new("Viewer");
        let bad = Event { id: "e1".into(), subject: "".into(), detail: String::new(), timestamp: 0 };
        assert!(m.add_event(bad).is_err());
        let good = Event { id: "e1".into(), subject: "boot".into(), detail: String::new(), timestamp: 1 };
        m.add_event(good).unwrap();
        assert_eq!(m.event_count(), 1);
    }
}
