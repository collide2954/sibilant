use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs, process};

pub(super) struct Folder(PathBuf);

impl Folder {
    pub(super) fn new() -> Self {
        static FOLDERS: AtomicUsize = AtomicUsize::new(0);
        let path = env::temp_dir().join(format!(
            "sibilant-app-{}-{}",
            process::id(),
            FOLDERS.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }

    pub(super) fn recovery(&self) -> PathBuf {
        self.0.join("Recovery")
    }

    pub(super) fn sessions(&self) -> usize {
        fs::read_dir(self.recovery()).map_or(0, Iterator::count)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
