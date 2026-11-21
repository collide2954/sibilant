use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, mouse_area, row, scrollable, slider, space, text};
use iced::{Alignment, Element, Length, mouse};
use sibilant_media::{Entry, EntryKind, Place};

use crate::fonts;
use crate::icons::{self, Icon};
use crate::style::quiet;
use crate::theme::{Color, Palette};

const TEXT_SIZE: f32 = 13.0;
const ICON_SIZE: f32 = 14.0;
const INDENT: f32 = 16.0;
const SPACING: f32 = 4.0;
const ROW_PADDING: [f32; 2] = [2.0, 4.0];
const VOLUME_RANGE: RangeInclusive<f32> = -60.0..=0.0;
const VOLUME_STEP: f32 = 0.5;
const START_VOLUME: f32 = -6.0;
const VOLUME_LABEL_WIDTH: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Volume(f32);

impl Default for Volume {
    fn default() -> Self {
        Self(START_VOLUME)
    }
}

impl Volume {
    fn new(db: f32) -> Self {
        Self(db.clamp(*VOLUME_RANGE.start(), *VOLUME_RANGE.end()))
    }

    fn is_silent(self) -> bool {
        self.0 <= *VOLUME_RANGE.start()
    }

    fn gain(self) -> f32 {
        if self.is_silent() {
            0.0
        } else {
            10_f32.powf(self.0 / 20.0)
        }
    }

    fn label(self) -> String {
        if self.is_silent() {
            "Off".into()
        } else {
            format!("{:.1} dB", self.0).replace('-', "\u{2212}")
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Choose(usize),
    Toggle(PathBuf),
    Press(PathBuf),
    Leave(PathBuf),
    Release(PathBuf),
    Volume(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    List(PathBuf),
    Drag(PathBuf),
    Preview(PathBuf),
    StopPreview,
    Volume(f32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Listing {
    Loading,
    Listed(Vec<Entry>),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Notice {
    Problem(String),
    Info(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Browser {
    places: Vec<Place>,
    place: Option<usize>,
    listings: BTreeMap<PathBuf, Listing>,
    open: BTreeSet<PathBuf>,
    notice: Option<Notice>,
    pressed: Option<PathBuf>,
    dragged: bool,
    previewing: Option<PathBuf>,
    volume: Volume,
}

impl Browser {
    pub fn new(places: Vec<Place>) -> Self {
        let mut browser = Self::default();
        browser.set_places(places);
        browser
    }

    pub fn set_places(&mut self, places: Vec<Place>) {
        self.places = places;
    }

    pub fn preview_gain(&self) -> f32 {
        self.volume.gain()
    }

    pub fn update(&mut self, event: &Event) -> Option<Request> {
        match event {
            Event::Choose(index) => {
                let folder = self.places.get(*index)?.path.clone();
                self.place = Some(*index);
                self.list(folder)
            }
            Event::Toggle(folder) => {
                if self.open.remove(folder) {
                    None
                } else {
                    self.open.insert(folder.clone());
                    self.list(folder.clone())
                }
            }
            Event::Press(file) => {
                self.pressed = Some(file.clone());
                self.dragged = false;
                None
            }
            Event::Leave(file) => {
                let drag = self.pressed.as_ref() == Some(file) && !self.dragged;
                self.dragged |= drag;
                drag.then(|| Request::Drag(file.clone()))
            }
            Event::Release(file) => {
                let click = self.pressed.take().as_ref() == Some(file) && !self.dragged;
                self.dragged = false;
                click.then(|| {
                    if self.previewing.as_ref() == Some(file) {
                        Request::StopPreview
                    } else {
                        Request::Preview(file.clone())
                    }
                })
            }
            Event::Volume(db) => {
                self.volume = Volume::new(*db);
                Some(Request::Volume(self.volume.gain()))
            }
        }
    }

    pub fn set_previewing(&mut self, file: Option<PathBuf>) {
        self.previewing = file;
    }

    pub fn listed(&mut self, folder: PathBuf, result: Result<Vec<Entry>, String>) {
        let listing = match result {
            Ok(entries) => Listing::Listed(entries),
            Err(message) => Listing::Failed(message),
        };
        self.listings.insert(folder, listing);
    }

    pub fn show_problem(&mut self, problem: String) {
        self.notice = Some(Notice::Problem(problem));
    }

    pub fn show_info(&mut self, info: String) {
        self.notice = Some(Notice::Info(info));
    }

    pub fn clear_notice(&mut self) {
        self.notice = None;
    }

    fn list(&mut self, folder: PathBuf) -> Option<Request> {
        if self.listings.contains_key(&folder) {
            return None;
        }
        self.listings.insert(folder.clone(), Listing::Loading);
        Some(Request::List(folder))
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
        scrollable(column(rows).width(Length::Fill)).height(Length::Fill),
        volume(browser.volume, palette)
    ]
    .spacing(SPACING * 2.0);
    if let Some(notice) = &browser.notice {
        let (message, color) = match notice {
            Notice::Problem(message) => (message, palette.record),
            Notice::Info(message) => (message, palette.text),
        };
        content = content.push(text(message).size(TEXT_SIZE).color(color));
    }
    content.into()
}

fn volume<'a>(volume: Volume, palette: Palette) -> Element<'a, Event> {
    row![
        text("Preview").size(TEXT_SIZE).color(palette.text_muted),
        slider(VOLUME_RANGE, volume.0, Event::Volume).step(VOLUME_STEP),
        text(volume.label())
            .size(TEXT_SIZE)
            .font(fonts::NUMBERS)
            .color(palette.text_muted)
            .width(VOLUME_LABEL_WIDTH)
    ]
    .spacing(SPACING * 2.0)
    .align_y(Alignment::Center)
    .into()
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
                    EntryKind::Audio => {
                        let playing = browser.previewing.as_ref() == Some(&entry.path);
                        rows.push(file_row(entry, playing, indent, palette));
                    }
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

fn file_row(entry: &Entry, playing: bool, indent: f32, palette: Palette) -> Element<'_, Event> {
    let (glyph, name) = if playing {
        (Icon::Speaker, palette.accent)
    } else {
        (Icon::AudioFile, palette.text)
    };
    let content = row![
        space().width(indent + ICON_SIZE + SPACING),
        icon(glyph, palette.accent),
        text(&entry.name).size(TEXT_SIZE).color(name)
    ]
    .spacing(SPACING);
    mouse_area(container(content).width(Length::Fill).padding(ROW_PADDING))
        .on_press(Event::Press(entry.path.clone()))
        .on_release(Event::Release(entry.path.clone()))
        .on_exit(Event::Leave(entry.path.clone()))
        .interaction(mouse::Interaction::Pointer)
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

    use iced::Size;
    use iced_test::simulator;
    use sibilant_media::{Entry, EntryKind, Place};

    use crate::browser::{self, Browser, Event, Request};
    use crate::icons::Icon;
    use crate::pixels::Pixels;
    use crate::theme::{Color, Palette};

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

    fn pad() -> PathBuf {
        music().join("pad.wav")
    }

    #[test]
    fn choosing_a_place_asks_once_for_its_folder() {
        let mut browser = Browser::new(places());
        assert_eq!(
            browser.update(&Event::Choose(1)),
            Some(Request::List(music()))
        );
        assert_eq!(browser.update(&Event::Choose(1)), None);
    }

    #[test]
    fn opening_a_folder_asks_once_for_its_entries() {
        let mut browser = music_listed();
        let drums = music().join("Drums");
        assert_eq!(
            browser.update(&Event::Toggle(drums.clone())),
            Some(Request::List(drums.clone()))
        );
        assert_eq!(browser.update(&Event::Toggle(drums.clone())), None);
        assert_eq!(browser.update(&Event::Toggle(drums)), None);
    }

    #[test]
    fn a_click_on_a_file_asks_to_preview_it() {
        let mut browser = music_listed();
        assert_eq!(browser.update(&Event::Press(pad())), None);
        assert_eq!(
            browser.update(&Event::Release(pad())),
            Some(Request::Preview(pad()))
        );
    }

    #[test]
    fn a_click_on_the_file_that_plays_asks_to_stop_it() {
        let mut browser = music_listed();
        browser.set_previewing(Some(pad()));
        browser.update(&Event::Press(pad()));
        assert_eq!(
            browser.update(&Event::Release(pad())),
            Some(Request::StopPreview)
        );
    }

    #[test]
    fn leaving_the_row_with_the_button_down_starts_a_drag_and_no_preview() {
        let mut browser = music_listed();
        browser.update(&Event::Press(pad()));
        assert_eq!(
            browser.update(&Event::Leave(pad())),
            Some(Request::Drag(pad()))
        );
        assert_eq!(browser.update(&Event::Leave(pad())), None);
        assert_eq!(browser.update(&Event::Release(pad())), None);
    }

    #[test]
    fn moving_across_a_row_without_a_press_does_nothing() {
        let mut browser = music_listed();
        assert_eq!(browser.update(&Event::Leave(pad())), None);
    }

    #[test]
    fn a_release_on_a_different_file_is_not_a_click() {
        let mut browser = music_listed();
        browser.update(&Event::Press(pad()));
        let other = music().join("bass.wav");
        assert_eq!(browser.update(&Event::Release(other)), None);
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

    fn is_close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn the_preview_volume_starts_at_minus_6_db() {
        let browser = Browser::new(places());
        assert!(is_close(browser.preview_gain(), 0.501_19));
        let mut ui = simulator(browser::view(&browser, Palette::GRAPHITE));
        assert!(ui.find("Preview").is_ok());
        assert!(ui.find("\u{2212}6.0 dB").is_ok());
    }

    #[test]
    fn a_new_preview_volume_asks_for_its_gain() {
        let mut browser = Browser::new(places());
        let Some(Request::Volume(gain)) = browser.update(&Event::Volume(-12.0)) else {
            panic!("no volume request");
        };
        assert!(is_close(gain, 0.251_19));
        assert!(is_close(browser.preview_gain(), gain));
    }

    #[test]
    fn the_lowest_preview_volume_is_silent() {
        let mut browser = Browser::new(places());
        assert_eq!(
            browser.update(&Event::Volume(-60.0)),
            Some(Request::Volume(0.0))
        );
        let mut ui = simulator(browser::view(&browser, Palette::GRAPHITE));
        assert!(ui.find("Off").is_ok());
    }

    #[test]
    fn new_places_keep_the_preview_volume() {
        let mut browser = Browser::default();
        browser.update(&Event::Volume(-12.0));
        browser.set_places(places());
        assert!(is_close(browser.preview_gain(), 0.251_19));
        assert_eq!(
            browser.update(&Event::Choose(1)),
            Some(Request::List(music()))
        );
    }

    #[test]
    fn a_click_on_a_file_presses_and_releases_it() {
        assert_eq!(
            clicks(&music_listed(), "pad.wav"),
            [Event::Press(pad()), Event::Release(pad())]
        );
    }

    #[test]
    fn the_file_that_plays_shows_a_speaker() {
        let mut browser = music_listed();
        let speaker = Icon::Speaker.codepoint().to_string();
        assert!(
            simulator(browser::view(&browser, Palette::GRAPHITE))
                .find(speaker.as_str())
                .is_err()
        );
        browser.set_previewing(Some(pad()));
        assert!(
            simulator(browser::view(&browser, Palette::GRAPHITE))
                .find(speaker.as_str())
                .is_ok()
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

    fn shows(browser: &Browser, notice: &str) -> bool {
        simulator(browser::view(browser, Palette::GRAPHITE))
            .find(notice)
            .is_ok()
    }

    #[test]
    fn the_browser_shows_its_newest_notice_until_it_is_cleared() {
        let mut browser = music_listed();
        browser.show_problem("kick.wav: not a WAV file".into());
        assert!(shows(&browser, "kick.wav: not a WAV file"));
        browser.show_info("Exported MySong.wav".into());
        assert!(shows(&browser, "Exported MySong.wav"));
        assert!(!shows(&browser, "kick.wav: not a WAV file"));
        browser.clear_notice();
        assert!(!shows(&browser, "Exported MySong.wav"));
    }

    fn notice_line_colors(browser: &Browser) -> Vec<Color> {
        let pixels = &Pixels::render(
            browser::view(browser, Palette::GRAPHITE),
            Size::new(240.0, 300.0),
        );
        (286..298)
            .flat_map(|y| (0..240).map(move |x| pixels.color_at(x, y)))
            .collect()
    }

    fn is_red(color: Color) -> bool {
        color.red() > color.green().saturating_add(0x60)
    }

    #[test]
    fn a_problem_shows_in_the_record_color() {
        let mut browser = music_listed();
        browser.show_problem("kick.wav: not a WAV file".into());
        assert!(notice_line_colors(&browser).into_iter().any(is_red));
    }

    #[test]
    fn a_notice_that_is_not_a_problem_shows_in_the_text_color() {
        let mut browser = music_listed();
        browser.show_info("Exported MySong.wav".into());
        let colors = notice_line_colors(&browser);
        assert!(!colors.iter().copied().any(is_red));
        assert!(colors.iter().any(|color| color.green() > 0xD0));
    }
}
