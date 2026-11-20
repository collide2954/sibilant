use std::sync::Arc;

use super::{EditError, Project, SceneId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene {
    id: SceneId,
    name: String,
}

impl Scene {
    pub fn id(&self) -> SceneId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Project {
    pub fn scenes(&self) -> impl ExactSizeIterator<Item = &Scene> {
        self.scenes.iter().map(Arc::as_ref)
    }

    pub fn scene(&self, id: SceneId) -> Option<&Scene> {
        self.scenes().find(|scene| scene.id == id)
    }

    pub fn add_scene(&self, id: SceneId, name: impl Into<String>) -> Result<Self, EditError> {
        if self.scene(id).is_some() {
            return Err(EditError::DuplicateScene(id));
        }
        let mut project = self.clone();
        let name = name.into();
        project.scenes.push(Arc::new(Scene { id, name }));
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use crate::project::{EditError, Project, Scene, SceneId};

    fn with_scenes(names: &[&str]) -> Project {
        (1..)
            .zip(names)
            .fold(Project::default(), |project, (id, &name)| {
                project.add_scene(SceneId::new(id), name).unwrap()
            })
    }

    #[test]
    fn a_new_project_has_no_scenes() {
        assert_eq!(Project::default().scenes().len(), 0);
    }

    #[test]
    fn adding_a_scene_puts_it_below_the_other_scenes() {
        let project = with_scenes(&["Intro", "Verse"]);
        let names: Vec<_> = project.scenes().map(Scene::name).collect();
        assert_eq!(names, ["Intro", "Verse"]);
    }

    #[test]
    fn finds_a_scene_by_its_id() {
        let project = with_scenes(&["Intro", "Verse"]);
        let scene = project.scene(SceneId::new(2)).unwrap();
        assert_eq!((scene.id(), scene.name()), (SceneId::new(2), "Verse"));
        assert!(project.scene(SceneId::new(3)).is_none());
    }

    #[test]
    fn a_scene_id_can_be_used_only_once() {
        let project = with_scenes(&["Intro"]);
        assert_eq!(
            project.add_scene(SceneId::new(1), "Verse"),
            Err(EditError::DuplicateScene(SceneId::new(1)))
        );
    }
}
