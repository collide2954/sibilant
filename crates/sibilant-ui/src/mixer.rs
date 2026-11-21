mod meter;
mod scale;
mod slider;

use sibilant_core::project::TrackId;

pub use meter::meter;
pub use scale::{fader_db, fader_position, gain_text, pan_text};
pub use slider::{fader, pan};

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
