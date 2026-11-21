const TAPER: [(f64, f64); 7] = [
    (0.0, -100.0),
    (0.05, -60.0),
    (0.15, -40.0),
    (0.3, -24.0),
    (0.5, -12.0),
    (0.75, 0.0),
    (1.0, 6.0),
];

pub fn fader_db(position: f32) -> f64 {
    let position = f64::from(position);
    if position <= 0.0 {
        return f64::NEG_INFINITY;
    }
    TAPER
        .windows(2)
        .find(|pair| position <= pair[1].0)
        .map_or(TAPER[6].1, |pair| {
            let [(from, low), (to, high)] = [pair[0], pair[1]];
            low + (position - from) / (to - from) * (high - low)
        })
}

pub fn fader_position(db: f64) -> f32 {
    let position = TAPER
        .windows(2)
        .find(|pair| db <= pair[1].1)
        .map_or(TAPER[6].0, |pair| {
            let [(from, low), (to, high)] = [pair[0], pair[1]];
            from + (db.max(low) - low) / (high - low) * (to - from)
        });
    to_f32(position)
}

pub fn gain_text(db: f64) -> String {
    if db == f64::NEG_INFINITY {
        return "-inf dB".to_owned();
    }
    let tenths = (db * 10.0).round() / 10.0;
    if tenths > 0.0 {
        format!("+{tenths:.1} dB")
    } else {
        format!("{:.1} dB", tenths + 0.0)
    }
}

pub fn pan_text(position: f64) -> String {
    let percent = (position * 100.0).round();
    match percent {
        0.0 => "C".to_owned(),
        _ if percent < 0.0 => format!("L{}", -percent),
        _ => format!("R{percent}"),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "fader positions are between 0 and 1"
)]
fn to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use crate::mixer::{fader_db, fader_position, gain_text, pan_text};

    fn is_close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn zero_db_sits_at_three_quarters_of_the_travel() {
        assert!(is_close(fader_db(0.75), 0.0));
        assert!(is_close(f64::from(fader_position(0.0)), 0.75));
    }

    #[test]
    fn the_bottom_is_silence_and_the_top_is_plus_6_db() {
        assert_eq!(fader_db(0.0), f64::NEG_INFINITY);
        assert!(is_close(fader_db(1.0), 6.0));
        assert!(is_close(f64::from(fader_position(f64::NEG_INFINITY)), 0.0));
        assert!(is_close(f64::from(fader_position(6.0)), 1.0));
    }

    #[test]
    fn positions_outside_the_travel_stay_at_its_ends() {
        assert_eq!(fader_db(-0.5), f64::NEG_INFINITY);
        assert!(is_close(fader_db(1.5), 6.0));
    }

    #[test]
    fn the_steps_are_finer_near_0_db_than_near_the_bottom() {
        let step = |from: f32, to: f32| fader_db(to) - fader_db(from);
        assert!(step(0.74, 0.76) < step(0.10, 0.12));
    }

    #[test]
    fn a_position_and_its_gain_map_back_and_forth() {
        for hundredth in 1..=100_u8 {
            let position = f32::from(hundredth) / 100.0;
            let back = fader_position(fader_db(position));
            assert!(
                (back - position).abs() < 1e-4,
                "{position} came back as {back}"
            );
        }
    }

    #[test]
    fn the_gain_reads_in_db_with_one_decimal() {
        let texts = [
            (f64::NEG_INFINITY, "-inf dB"),
            (-6.0, "-6.0 dB"),
            (2.54, "+2.5 dB"),
            (-0.04, "0.0 dB"),
            (0.0, "0.0 dB"),
        ];
        for (db, text) in texts {
            assert_eq!(gain_text(db), text, "{db}");
        }
    }

    #[test]
    fn the_pan_reads_as_left_center_or_right() {
        let texts = [(-0.5, "L50"), (0.0, "C"), (0.004, "C"), (1.0, "R100")];
        for (position, text) in texts {
            assert_eq!(pan_text(position), text, "{position}");
        }
    }
}
