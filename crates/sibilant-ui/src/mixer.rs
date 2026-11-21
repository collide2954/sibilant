mod meter;
mod scale;
mod slider;
mod strip;

use sibilant_core::project::TrackId;

pub use meter::meter;
pub use scale::{fader_db, fader_position, gain_text, pan_text};
pub use slider::{fader, pan};
pub use strip::view;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Meter {
    pub level: f32,
    pub clipped: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meters {
    pub tracks: Vec<Meter>,
    pub master: Meter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Channel {
    Track(TrackId),
    Master,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mixing {
    Gain(Channel, f64),
    Pan(TrackId, f64),
    Done,
    ResetGain(Channel),
    ResetPan(TrackId),
}
