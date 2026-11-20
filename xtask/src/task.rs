use crate::step::Step;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Task {
    Ci,
    PreCommit,
    PrePush,
    InstallHooks,
}

impl Task {
    const ALL: [Self; 4] = [Self::Ci, Self::PreCommit, Self::PrePush, Self::InstallHooks];

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|task| task.name() == name)
    }

    pub(crate) fn usage() -> String {
        format!(
            "usage: cargo xtask <{}>",
            Self::ALL.map(Self::name).join("|")
        )
    }

    pub(crate) fn steps(self) -> &'static [Step] {
        match self {
            Self::Ci => &[Step::FORMAT, Step::LINT, Step::TEST, Step::DOC, Step::DENY],
            Self::PreCommit => &[Step::FORMAT, Step::LINT],
            Self::PrePush => &[Step::TEST],
            Self::InstallHooks => &[Step::INSTALL_HOOKS],
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Ci => "ci",
            Self::PreCommit => "pre-commit",
            Self::PrePush => "pre-push",
            Self::InstallHooks => "install-hooks",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Task;

    fn commands(task: Task) -> Vec<String> {
        task.steps().iter().map(ToString::to_string).collect()
    }

    #[test]
    fn finds_each_task_by_name() {
        assert_eq!(Task::from_name("ci"), Some(Task::Ci));
        assert_eq!(Task::from_name("pre-commit"), Some(Task::PreCommit));
        assert_eq!(Task::from_name("pre-push"), Some(Task::PrePush));
        assert_eq!(Task::from_name("install-hooks"), Some(Task::InstallHooks));
    }

    #[test]
    fn rejects_an_unknown_name() {
        assert_eq!(Task::from_name("deploy"), None);
    }

    #[test]
    fn usage_lists_every_task() {
        assert_eq!(
            Task::usage(),
            "usage: cargo xtask <ci|pre-commit|pre-push|install-hooks>"
        );
    }

    #[test]
    fn pre_commit_checks_format_then_lints() {
        assert_eq!(
            commands(Task::PreCommit),
            [
                "cargo fmt --all --check",
                "cargo clippy --workspace --all-targets -- -D warnings",
            ]
        );
    }

    #[test]
    fn pre_push_runs_the_tests() {
        assert_eq!(commands(Task::PrePush), ["cargo test --workspace"]);
    }

    #[test]
    fn ci_runs_every_check() {
        assert_eq!(
            commands(Task::Ci),
            [
                "cargo fmt --all --check",
                "cargo clippy --workspace --all-targets -- -D warnings",
                "cargo test --workspace",
                "RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps",
                "cargo deny check",
            ]
        );
    }

    #[test]
    fn install_hooks_points_git_at_the_hooks_folder() {
        assert_eq!(
            commands(Task::InstallHooks),
            ["git config core.hooksPath .githooks"]
        );
    }
}
