use std::fmt;
use std::io;
use std::process::{Command, ExitStatus};

#[derive(Debug)]
pub(crate) struct Step {
    program: &'static str,
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
}

impl Step {
    pub(crate) const FORMAT: Self = Self::cargo(&["fmt", "--all", "--check"]);
    pub(crate) const LINT: Self = Self::cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ]);
    pub(crate) const TEST: Self = Self::cargo(&["test", "--workspace"]);
    pub(crate) const DOC: Self = Self::cargo(&["doc", "--workspace", "--no-deps"])
        .with_env(&[("RUSTDOCFLAGS", "-D warnings")]);
    pub(crate) const DENY: Self = Self::cargo(&["deny", "check"]);
    pub(crate) const INSTALL_HOOKS: Self =
        Self::new("git", &["config", "core.hooksPath", ".githooks"]);

    const fn new(program: &'static str, args: &'static [&'static str]) -> Self {
        Self {
            program,
            args,
            env: &[],
        }
    }

    const fn cargo(args: &'static [&'static str]) -> Self {
        Self::new("cargo", args)
    }

    const fn with_env(self, env: &'static [(&'static str, &'static str)]) -> Self {
        Self { env, ..self }
    }

    pub(crate) fn run(&self) -> io::Result<ExitStatus> {
        self.command().status()
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.program);
        command.args(self.args).envs(self.env.iter().copied());
        command
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, value) in self.env {
            write!(f, "{name}=\"{value}\" ")?;
        }
        f.write_str(self.program)?;
        for arg in self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::Step;

    #[test]
    fn fails_when_the_program_is_missing() {
        let step = Step::new("sibilant-missing-program", &[]);
        assert!(step.run().is_err());
    }

    #[test]
    fn passes_its_environment_to_the_command() {
        let command = Step::DOC.command();
        let env: Vec<_> = command.get_envs().collect();
        assert_eq!(
            env,
            [(OsStr::new("RUSTDOCFLAGS"), Some(OsStr::new("-D warnings")))]
        );
    }
}
