use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, mouse_area, row, scrollable, space, text};
use iced::{Element, Length, mouse};
use sibilant_media::{Entry, EntryKind, Place};

use crate::icons::{self, Icon};
use crate::style::quiet;
use crate::theme::{Color, Palette};

const TEXT_SIZE: f32 = 13.0;
const ICON_SIZE: f32 = 14.0;
const INDENT: f32 = 16.0;
const SPACING: f32 = 4.0;
const ROW_PADDING: [f32; 2] = [2.0, 4.0];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Choose(usize),
    Toggle(PathBuf),
    Grab(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Listing {
    Loading,
    Listed(Vec<Entry>),
    Failed(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Browser {
    places: Vec<Place>,
    place: Option<usize>,
    listings: BTreeMap<PathBuf, Listing>,
    open: BTreeSet<PathBuf>,
    notice: Option<String>,
}

impl Browser {
    pub fn new(places: Vec<Place>) -> Self {
        Self {
            places,
            ..Self::default()
        }
    }

    pub fn update(&mut self, event: &Event) -> Option<PathBuf> {
        match event {
            Event::Choose(index) => {
                let folder = self.places.get(*index)?.path.clone();
                self.place = Some(*index);
                self.request(folder)
            }
            Event::Toggle(folder) => {
                if self.open.remove(folder) {
                    None
                } else {
                    self.open.insert(folder.clone());
                    self.request(folder.clone())
                }
            }
            Event::Grab(_) => None,
        }
    }

    pub fn listed(&mut self, folder: PathBuf, result: Result<Vec<Entry>, String>) {
        let listing = match result {
            Ok(entries) => Listing::Listed(entries),
            Err(message) => Listing::Failed(message),
        };
        self.listings.insert(folder, listing);
    }

    pub fn set_notice(&mut self, notice: Option<String>) {
        self.notice = notice;
    }

    fn request(&mut self, folder: PathBuf) -> Option<PathBuf> {
        if self.listings.contains_key(&folder) {
            return None;
        }
        self.listings.insert(folder.clone(), Listing::Loading);
        Some(folder)
    }
}

pub fn view(browser: &Browser, palette: Palette) -> Element<'_, Event> {
    let places = row(browser.places.iter().enumerate().map(|(index, place)| {
        let chosen = browser.place == Some(index);
        button(text(&place.name).size(TEXT_SIZE))
            .on_press(Event::Choose(index))
            .padding(ROW_PADDING)
            .style(move |_, status| {
                if chosen {
                    button::Style {
                        text_color: palette.text.into(),
                        ..quiet(palette, button::Status::Hovered)
                    }
                } else {
                    quiet(palette, status)
                }
            })
            .into()
    }))
    .spacing(SPACING)
    .wrap();
    let mut rows = Vec::new();
    if let Some(place) = browser.place.and_then(|index| browser.places.get(index)) {
        tree(browser, &place.path, 0.0, palette, &mut rows);
    }
    let mut content = column![
        places,
        scrollable(column(rows).width(Length::Fill)).height(Length::Fill)
    ]
    .spacing(SPACING * 2.0);
    if let Some(notice) = &browser.notice {
        content = content.push(text(notice).size(TEXT_SIZE).color(palette.record));
    }
    content.into()
}

fn tree<'a>(
    browser: &'a Browser,
    folder: &Path,
    indent: f32,
    palette: Palette,
    rows: &mut Vec<Element<'a, Event>>,
) {
    match browser.listings.get(folder) {
        None | Some(Listing::Loading) => rows.push(note("Loading", indent, palette)),
        Some(Listing::Failed(message)) => rows.push(note(message, indent, palette)),
        Some(Listing::Listed(entries)) if entries.is_empty() => {
            rows.push(note("No audio files", indent, palette));
        }
        Some(Listing::Listed(entries)) => {
            for entry in entries {
                match entry.kind {
                    EntryKind::Folder => {
                        let open = browser.open.contains(&entry.path);
                        rows.push(folder_row(entry, open, indent, palette));
                        if open {
                            tree(browser, &entry.path, indent + INDENT, palette, rows);
                        }
                    }
                    EntryKind::Audio => rows.push(file_row(entry, indent, palette)),
                }
            }
        }
    }
}

fn folder_row(entry: &Entry, open: bool, indent: f32, palette: Palette) -> Element<'_, Event> {
    let (chevron, folder) = if open {
        (Icon::ChevronDown, Icon::FolderOpen)
    } else {
        (Icon::ChevronRight, Icon::Folder)
    };
    let content = row![
        space().width(indent),
        icon(chevron, palette.text_muted),
        icon(folder, palette.text_muted),
        text(&entry.name).size(TEXT_SIZE).color(palette.text)
    ]
    .spacing(SPACING);
    button(content)
        .on_press(Event::Toggle(entry.path.clone()))
        .width(Length::Fill)
        .padding(ROW_PADDING)
        .style(move |_, status| quiet(palette, status))
        .into()
}

fn file_row(entry: &Entry, indent: f32, palette: Palette) -> Element<'_, Event> {
    let content = row![
        space().width(indent + ICON_SIZE + SPACING),
        icon(Icon::AudioFile, palette.accent),
        text(&entry.name).size(TEXT_SIZE).color(palette.text)
    ]
    .spacing(SPACING);
    mouse_area(container(content).width(Length::Fill).padding(ROW_PADDING))
        .on_press(Event::Grab(entry.path.clone()))
        .interaction(mouse::Interaction::Grab)
        .into()
}

fn note(message: &str, indent: f32, palette: Palette) -> Element<'_, Event> {
    row![
        space().width(indent + ICON_SIZE + SPACING),
        text(message).size(TEXT_SIZE).color(palette.text_muted)
    ]
    .padding(ROW_PADDING)
    .into()
}

fn icon<'a>(icon: Icon, color: Color) -> Element<'a, Event> {
    text(icon.codepoint())
        .font(icons::FONT)
        .size(ICON_SIZE)
        .color(color)
        .into()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use iced_test::simulator;
    use sibilant_media::{Entry, EntryKind, Place};

    use crate::browser::{self, Browser, Event};
    use crate::theme::Palette;

    fn music() -> PathBuf {
        Path::new("home").join("Music")
    }

    fn places() -> Vec<Place> {
        vec![
            Place {
                name: "Home".into(),
                path: PathBuf::from("home"),
            },
            Place {
                name: "Music".into(),
                path: music(),
            },
        ]
    }

    fn entry(folder: &Path, name: &str, kind: EntryKind) -> Entry {
        Entry {
            name: name.into(),
            path: folder.join(name),
            kind,
        }
    }

    fn music_listed() -> Browser {
        let mut browser = Browser::new(places());
        browser.update(&Event::Choose(1));
        browser.listed(
            music(),
            Ok(vec![
                entry(&music(), "Drums", EntryKind::Folder),
                entry(&music(), "pad.wav", EntryKind::Audio),
            ]),
        );
        browser
    }

    fn clicks(browser: &Browser, label: &str) -> Vec<Event> {
        let mut ui = simulator(browser::view(browser, Palette::GRAPHITE));
        ui.click(label).unwrap();
        ui.into_messages().collect()
    }

    #[test]
    fn choosing_a_place_asks_once_for_its_folder() {
        let mut browser = Browser::new(places());
        assert_eq!(browser.update(&Event::Choose(1)), Some(music()));
        assert_eq!(browser.update(&Event::Choose(1)), None);
    }

    #[test]
    fn opening_a_folder_asks_once_for_its_entries() {
        let mut browser = music_listed();
        let drums = music().join("Drums");
        assert_eq!(
            browser.update(&Event::Toggle(drums.clone())),
            Some(drums.clone())
        );
        assert_eq!(browser.update(&Event::Toggle(drums.clone())), None);
        assert_eq!(browser.update(&Event::Toggle(drums)), None);
    }

    #[test]
    fn grabbing_a_file_asks_for_no_folder() {
        let mut browser = music_listed();
        assert_eq!(browser.update(&Event::Grab(music().join("pad.wav"))), None);
    }

    #[test]
    fn the_browser_shows_the_places_and_chooses_one_on_a_click() {
        assert_eq!(clicks(&Browser::new(places()), "Music"), [Event::Choose(1)]);
    }

    #[test]
    fn a_click_on_a_folder_opens_it() {
        assert_eq!(
            clicks(&music_listed(), "Drums"),
            [Event::Toggle(music().join("Drums"))]
        );
    }

    #[test]
    fn a_press_on_a_file_grabs_it() {
        assert_eq!(
            clicks(&music_listed(), "pad.wav"),
            [Event::Grab(music().join("pad.wav"))]
        );
    }

    #[test]
    fn an_open_folder_shows_its_entries_below_it_and_further_in() {
        let mut browser = music_listed();
        let drums = music().join("Drums");
        browser.update(&Event::Toggle(drums.clone()));
        browser.listed(
            drums.clone(),
            Ok(vec![entry(&drums, "kick.wav", EntryKind::Audio)]),
        );
        let mut ui = simulator(browser::view(&browser, Palette::GRAPHITE));
        let folder = ui.find("Drums").unwrap().bounds();
        let kick = ui.find("kick.wav").unwrap().bounds();
        let pad = ui.find("pad.wav").unwrap().bounds();
        assert!(folder.y < kick.y && kick.y < pad.y);
        assert!(kick.x > pad.x);
    }

    #[test]
    fn a_folder_that_cannot_be_read_tells_why() {
        let mut browser = Browser::new(places());
        browser.update(&Event::Choose(0));
        browser.listed(PathBuf::from("home"), Err("access is denied".into()));
        let mut ui = simulator(browser::view(&browser, Palette::GRAPHITE));
        assert!(ui.find("access is denied").is_ok());
    }

    #[test]
    fn a_folder_without_audio_files_says_so() {
        let mut browser = Browser::new(places());
        browser.update(&Event::Choose(0));
        browser.listed(PathBuf::from("home"), Ok(Vec::new()));
        let mut ui = simulator(browser::view(&browser, Palette::GRAPHITE));
        assert!(ui.find("No audio files").is_ok());
    }

    #[test]
    fn the_browser_shows_its_notice() {
        let mut browser = music_listed();
        browser.set_notice(Some("kick.wav: not a WAV file".into()));
        let mut ui = simulator(browser::view(&browser, Palette::GRAPHITE));
        assert!(ui.find("kick.wav: not a WAV file").is_ok());
    }
}
