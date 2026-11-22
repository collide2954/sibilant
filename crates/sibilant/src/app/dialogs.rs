use std::path::PathBuf;

use iced::{Task, window};
use rfd::AsyncFileDialog;
use sibilant_project::EXTENSION;

type Filter = (&'static str, &'static str);

const PROJECT: Filter = ("Sibilant project", EXTENSION);
const WAV: Filter = ("WAV audio", "wav");

pub(super) fn open_file() -> Task<Option<PathBuf>> {
    prepare("Open Project", PROJECT, None).then(|dialog| {
        Task::future(async move {
            dialog
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf())
        })
    })
}

pub(super) fn save_file(name: &str) -> Task<Option<PathBuf>> {
    save("Save Project", PROJECT, name)
}

pub(super) fn export_file(name: &str) -> Task<Option<PathBuf>> {
    save("Export Audio", WAV, name)
}

fn save(title: &'static str, filter: Filter, name: &str) -> Task<Option<PathBuf>> {
    let file_name = format!("{name}.{}", filter.1);
    prepare(title, filter, Some(file_name)).then(|dialog| {
        Task::future(async move {
            dialog
                .save_file()
                .await
                .map(|file| file.path().to_path_buf())
        })
    })
}

fn prepare(
    title: &'static str,
    (kind, extension): Filter,
    file_name: Option<String>,
) -> Task<AsyncFileDialog> {
    let make = move || {
        let dialog = AsyncFileDialog::new()
            .set_title(title)
            .add_filter(kind, &[extension]);
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
