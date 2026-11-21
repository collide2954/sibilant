use std::sync::atomic::{AtomicU64, Ordering};

use sibilant_core::project::Project;
use sibilant_project::{Copies, Location};

const UNTITLED: &str = "Untitled";
const CHANGED: &str = "● ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DocumentId(u64);

impl DocumentId {
    fn unique() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub(super) struct Document {
    id: DocumentId,
    location: Option<Location>,
    saved: Project,
    copied: Option<Project>,
    copies: Copies,
}

impl Document {
    pub(super) fn new(project: &Project, location: Option<Location>) -> Self {
        Self {
            id: DocumentId::unique(),
            location,
            saved: project.clone(),
            copied: None,
            copies: Copies::default(),
        }
    }

    pub(super) fn id(&self) -> DocumentId {
        self.id
    }

    pub(super) fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }

    pub(super) fn copies(&self) -> &Copies {
        &self.copies
    }

    pub(super) fn name(&self) -> String {
        self.location
            .as_ref()
            .map_or_else(|| UNTITLED.to_owned(), Location::name)
    }

    pub(super) fn is_changed(&self, present: &Project) -> bool {
        *present != self.saved
    }

    pub(super) fn needs_copy(&self, present: &Project) -> bool {
        self.is_changed(present) && self.copied.as_ref() != Some(present)
    }

    pub(super) fn copied(&mut self, project: Project, copies: Copies) {
        self.copied = Some(project);
        self.copies = copies;
    }

    pub(super) fn title(&self, present: &Project) -> String {
        let changed = if self.is_changed(present) {
            CHANGED
        } else {
            ""
        };
        format!("{changed}{} — Sibilant", self.name())
    }

    pub(super) fn saved(&mut self, project: Project, location: Location, copies: Copies) {
        self.saved = project;
        self.location = Some(location);
        self.copies = copies;
    }
}
