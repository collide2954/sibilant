use sibilant_core::project::Project;
use sibilant_project::{Copies, Location};

const UNTITLED: &str = "Untitled";
const CHANGED: &str = "● ";

#[derive(Debug)]
pub(super) struct Document {
    location: Option<Location>,
    saved: Project,
    copies: Copies,
}

impl Document {
    pub(super) fn new(project: &Project) -> Self {
        Self {
            location: None,
            saved: project.clone(),
            copies: Copies::default(),
        }
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
