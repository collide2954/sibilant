use sibilant_core::time::{
    BarBeat, Clock, Duration, Musical, Position, SUPERCLOCKS_PER_SECOND, TempoMap, TimeSignatureMap,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Timing {
    sample_rate: u32,
    tempo_map: TempoMap,
    time_signatures: TimeSignatureMap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Click {
    pub frame: i64,
    pub position: Position<Musical>,
    pub accent: bool,
}

impl Timing {
    pub fn new(sample_rate: u32, tempo_map: TempoMap, time_signatures: TimeSignatureMap) -> Self {
        Self {
            sample_rate,
            tempo_map,
            time_signatures,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn clock_of_frame(&self, frame: i64) -> Position<Clock> {
        clock_of_frame(self.sample_rate, frame)
    }

    pub fn first_frame_at_or_after(&self, clock: Position<Clock>) -> i64 {
        let scaled = i128::from(clock.superclocks()) * i128::from(self.sample_rate);
        narrow(-(-scaled).div_euclid(i128::from(SUPERCLOCKS_PER_SECOND)))
    }

    pub fn clock_of(&self, position: Position<Musical>) -> Position<Clock> {
        self.tempo_map.to_clock(position)
    }

    pub fn frame_of(&self, position: Position<Musical>) -> i64 {
        self.first_frame_at_or_after(self.clock_of(position))
    }

    pub fn position_at_frame(&self, frame: i64) -> Position<Musical> {
        self.tempo_map.to_musical(self.clock_of_frame(frame))
    }

    pub fn clicks(&self, start: i64, end: i64) -> Clicks<'_> {
        let after = self.clock_of_frame(start - 1);
        let bar = self
            .time_signatures
            .to_bar_beat(self.tempo_map.to_musical(after))
            .bar;
        Clicks {
            timing: self,
            next: BarBeat {
                bar: bar - 1,
                beat: 1,
                offset: Duration::ZERO,
            },
            after,
            until: self.clock_of_frame(end - 1),
        }
    }

    fn beat_after(&self, beat: BarBeat) -> BarBeat {
        let bar_start = self.time_signatures.bar_start(beat.bar);
        let beats = i64::from(
            self.time_signatures
                .time_signature_at(bar_start)
                .numerator(),
        );
        if beat.beat < beats {
            BarBeat {
                beat: beat.beat + 1,
                ..beat
            }
        } else {
            BarBeat {
                bar: beat.bar + 1,
                beat: 1,
                ..beat
            }
        }
    }
}

#[derive(Debug)]
pub struct Clicks<'a> {
    timing: &'a Timing,
    next: BarBeat,
    after: Position<Clock>,
    until: Position<Clock>,
}

impl Iterator for Clicks<'_> {
    type Item = Click;

    fn next(&mut self) -> Option<Click> {
        loop {
            let beat = self.next;
            let position = self.timing.time_signatures.to_position(beat);
            let clock = self.timing.clock_of(position);
            if clock > self.until {
                return None;
            }
            self.next = self.timing.beat_after(beat);
            if clock > self.after {
                return Some(Click {
                    frame: self.timing.first_frame_at_or_after(clock),
                    position,
                    accent: beat.beat == 1,
                });
            }
        }
    }
}

pub fn clock_of_frame(sample_rate: u32, frame: i64) -> Position<Clock> {
    let scaled = i128::from(frame) * i128::from(SUPERCLOCKS_PER_SECOND);
    Position::from_superclocks(narrow(scaled.div_euclid(i128::from(sample_rate))))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "frame and superclock counts of real sessions stay inside the range of i64"
)]
fn narrow(value: i128) -> i64 {
    value as i64
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use sibilant_core::time::{
        Clock, Musical, Position, Tempo, TempoMap, TimeSignature, TimeSignatureMap,
    };

    use crate::timing::{Click, Timing};

    fn tempo(bpm: f64) -> TempoMap {
        TempoMap::new(Tempo::from_bpm(bpm).unwrap())
    }

    fn timing(sample_rate: u32, bpm: f64) -> Timing {
        Timing::new(sample_rate, tempo(bpm), TimeSignatureMap::default())
    }

    fn frames_and_accents(clicks: impl Iterator<Item = Click>) -> Vec<(i64, bool)> {
        clicks.map(|click| (click.frame, click.accent)).collect()
    }

    #[test]
    fn a_second_has_as_many_frames_as_the_sample_rate() {
        let timing = timing(48_000, 120.0);
        assert_eq!(
            timing.clock_of_frame(48_000),
            Position::<Clock>::from_seconds(1)
        );
    }

    #[test]
    fn a_musical_position_belongs_to_the_first_frame_at_or_after_it() {
        let quarter_note = Position::<Musical>::from_quarter_notes(1);
        assert_eq!(timing(48_000, 120.0).frame_of(quarter_note), 24_000);
        assert_eq!(timing(44_100, 128.0).frame_of(quarter_note), 20_672);
    }

    #[test]
    fn the_metronome_clicks_on_each_beat_and_accents_the_first_beat_of_a_bar() {
        let timing = timing(48_000, 120.0);
        assert_eq!(
            frames_and_accents(timing.clicks(0, 96_001)),
            [
                (0, true),
                (24_000, false),
                (48_000, false),
                (72_000, false),
                (96_000, true)
            ]
        );
    }

    #[test]
    fn a_click_belongs_to_one_block_only() {
        let timing = timing(48_000, 120.0);
        assert_eq!(frames_and_accents(timing.clicks(0, 24_000)), [(0, true)]);
        assert_eq!(
            frames_and_accents(timing.clicks(24_000, 48_000)),
            [(24_000, false)]
        );
    }

    #[test]
    fn the_clicks_follow_the_time_signature() {
        let waltz = TimeSignatureMap::new(TimeSignature::new(3, 4).unwrap());
        let timing = Timing::new(48_000, tempo(120.0), waltz);
        let accents: Vec<_> = timing.clicks(0, 96_001).map(|click| click.accent).collect();
        assert_eq!(accents, [true, false, false, true, false]);
    }

    fn timings() -> impl Strategy<Value = Timing> {
        (
            prop::sample::select(vec![22_050_u32, 44_100, 48_000, 96_000, 37_800]),
            30.0..=300.0,
            prop::collection::vec((1..1_i64 << 26, 30.0..=300.0), 0..3),
            prop::collection::vec((1..64_i64, 1..13_u8), 0..3),
        )
            .prop_map(|(rate, first, tempos, signatures)| {
                let tempo_map = tempos.into_iter().fold(tempo(first), |map, (ticks, bpm)| {
                    map.with_tempo(Position::from_ticks(ticks), Tempo::from_bpm(bpm).unwrap())
                });
                let signature_map = signatures.into_iter().fold(
                    TimeSignatureMap::default(),
                    |map, (bar, numerator)| {
                        map.with_time_signature(bar, TimeSignature::new(numerator, 8).unwrap())
                    },
                );
                Timing::new(rate, tempo_map, signature_map)
            })
    }

    proptest! {
        #[test]
        fn blocks_of_any_size_find_the_same_clicks_as_one_long_block(
            timing in timings(),
            start in 0..1_i64 << 20,
            blocks in prop::collection::vec(1..4_096_i64, 1..40),
        ) {
            let end = start + blocks.iter().sum::<i64>();
            let whole: Vec<_> = timing.clicks(start, end).collect();
            let mut pieces = Vec::new();
            let mut block_start = start;
            for block in blocks {
                pieces.extend(timing.clicks(block_start, block_start + block));
                block_start += block;
            }
            prop_assert_eq!(pieces, whole);
        }

        #[test]
        fn each_click_is_on_the_first_frame_at_or_after_its_beat(
            timing in timings(),
            start in 0..1_i64 << 20,
            length in 1..1_i64 << 18,
        ) {
            for click in timing.clicks(start, start + length) {
                let beat = timing.clock_of(click.position);
                prop_assert!(timing.clock_of_frame(click.frame) >= beat);
                prop_assert!(timing.clock_of_frame(click.frame - 1) < beat);
                prop_assert!((start..start + length).contains(&click.frame));
            }
        }
    }
}
