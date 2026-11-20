use sibilant_core::project::Project;

const UNTITLED: &str = "Untitled";
const CHANGED: &str = "● ";

#[derive(Debug)]
pub(super) struct Document {
    saved: Project,
}

impl Document {
    pub(super) fn new(project: &Project) -> Self {
        Self {
            saved: project.clone(),
        }
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
        format!("{changed}{UNTITLED} — Sibilant")
    }
}
