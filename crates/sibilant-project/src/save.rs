use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    NameTaken(PathBuf),
    NoName,
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameTaken(folder) => write!(
                f,
                "{} already contains a project with this name",
                folder.display()
            ),
            Self::NoName => write!(f, "the project needs a name"),
        }
    }
}

impl Error for SaveError {}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::SaveError;

    #[test]
    fn the_errors_tell_why_the_project_cannot_be_saved() {
        let taken = SaveError::NameTaken(PathBuf::from("MySong"));
        assert_eq!(
            taken.to_string(),
            "MySong already contains a project with this name"
        );
        assert_eq!(SaveError::NoName.to_string(), "the project needs a name");
    }
}
