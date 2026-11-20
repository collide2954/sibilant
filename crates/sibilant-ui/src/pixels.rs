use std::fs;
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, process};

use iced::{Element, Settings, Size, Theme};
use iced_test::Simulator;

use crate::theme::{Color, Palette};

const SCALE: usize = 2;

pub(crate) struct Pixels {
    width: usize,
    rgba: Vec<u8>,
}

impl Pixels {
    pub(crate) fn render<'a, Message>(
        element: impl Into<Element<'a, Message>>,
        size: Size,
    ) -> Self {
        static RENDERS: AtomicUsize = AtomicUsize::new(0);
        let folder = env::temp_dir().join(format!(
            "sibilant-pixels-{}-{}",
            process::id(),
            RENDERS.fetch_add(1, Ordering::Relaxed)
        ));
        let snapshot = Simulator::with_size(Settings::default(), size, element)
            .snapshot(&Theme::from(Palette::GRAPHITE))
            .unwrap();
        snapshot.matches_image(folder.join("render")).unwrap();
        let path = fs::read_dir(&folder)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let png = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&folder).unwrap();
        let mut reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
        let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
        let width = reader.next_frame(&mut rgba).unwrap().width as usize;
        Self { width, rgba }
    }

    pub(crate) fn color_at(&self, x: usize, y: usize) -> Color {
        let start = (y * self.width + x) * SCALE * 4;
        let [red, green, blue] = [0, 1, 2].map(|channel| self.rgba[start + channel]);
        Color::from_hex(u32::from_be_bytes([0, red, green, blue]))
    }
}
