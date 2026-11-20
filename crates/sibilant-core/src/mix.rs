mod gain;
mod pan;

pub use gain::{Gain, GainOutOfRange};
pub use pan::{ChannelGains, Pan, PanOutOfRange};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Mix {
    pub gain: Gain,
    pub pan: Pan,
    pub mute: bool,
    pub solo: bool,
}

#[cfg(test)]
mod tests {
    use crate::mix::{Gain, Mix, Pan};

    #[test]
    fn a_new_mix_plays_the_audio_unchanged() {
        let unchanged = Mix {
            gain: Gain::UNITY,
            pan: Pan::CENTER,
            mute: false,
            solo: false,
        };
        assert_eq!(Mix::default(), unchanged);
    }
}
