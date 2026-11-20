use std::fmt;
use std::io;
use std::process::{Command, ExitStatus};

#[derive(Debug)]
pub(crate) struct Step {
    program: &'static str,
    args: &'static [&'static str],
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
    pub(crate) const INSTALL_HOOKS: Self = Self {
        program: "git",
        args: &["config", "core.hooksPath", ".githooks"],
    };

    const fn cargo(args: &'static [&'static str]) -> Self {
        Self {
            program: "cargo",
            args,
        }
    }

    pub(crate) fn run(&self) -> io::Result<ExitStatus> {
        Command::new(self.program).args(self.args).status()
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.program)?;
        for arg in self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Step;

    #[test]
    fn fails_when_the_program_is_missing() {
        let step = Step {
            program: "sibilant-missing-program",
            args: &[],
        };
        assert!(step.run().is_err());
    }
}
