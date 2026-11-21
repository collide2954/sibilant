use std::path::PathBuf;

use iced::{Task, window};
use rfd::AsyncFileDialog;
use sibilant_project::EXTENSION;

const FILTER: &str = "Sibilant project";

pub(super) fn open_file() -> Task<Option<PathBuf>> {
    prepare("Open Project", None).then(|dialog| {
        Task::future(async move {
            dialog
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf())
        })
    })
}

pub(super) fn save_file(name: &str) -> Task<Option<PathBuf>> {
    let file_name = format!("{name}.{EXTENSION}");
    prepare("Save Project", Some(file_name)).then(|dialog| {
        Task::future(async move {
            dialog
                .save_file()
                .await
                .map(|file| file.path().to_path_buf())
        })
    })
}

fn prepare(title: &'static str, file_name: Option<String>) -> Task<AsyncFileDialog> {
    let make = move || {
        let dialog = AsyncFileDialog::new()
            .set_title(title)
            .add_filter(FILTER, &[EXTENSION]);
        match &file_name {
            Some(name) => dialog.set_file_name(name),
            None => dialog,
        }
    };
    window::oldest().then(move |window| {
        let make = make.clone();
        match window {
            Some(id) => window::run(id, move |window| make().set_parent(window)),
            None => Task::done(make()),
        }
    })
}
