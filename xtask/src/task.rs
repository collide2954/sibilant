use crate::step::Step;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Task {
    PreCommit,
    PrePush,
}

impl Task {
    const ALL: [Self; 2] = [Self::PreCommit, Self::PrePush];

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
            Self::PreCommit => &[Step::FORMAT, Step::LINT],
            Self::PrePush => &[Step::TEST],
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::PreCommit => "pre-commit",
            Self::PrePush => "pre-push",
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
        assert_eq!(Task::from_name("pre-commit"), Some(Task::PreCommit));
        assert_eq!(Task::from_name("pre-push"), Some(Task::PrePush));
    }

    #[test]
    fn rejects_an_unknown_name() {
        assert_eq!(Task::from_name("deploy"), None);
    }

    #[test]
    fn usage_lists_every_task() {
        assert_eq!(Task::usage(), "usage: cargo xtask <pre-commit|pre-push>");
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
}
