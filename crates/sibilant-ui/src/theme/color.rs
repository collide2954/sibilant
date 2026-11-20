#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl Color {
    pub const fn from_hex(hex: u32) -> Self {
        let [_, red, green, blue] = hex.to_be_bytes();
        Self { red, green, blue }
    }

    pub const fn red(self) -> u8 {
        self.red
    }

    pub const fn green(self) -> u8 {
        self.green
    }

    pub const fn blue(self) -> u8 {
        self.blue
    }

    pub fn relative_luminance(self) -> f64 {
        0.2126 * linear(self.red) + 0.7152 * linear(self.green) + 0.0722 * linear(self.blue)
    }

    pub fn contrast_ratio(self, other: Self) -> f64 {
        let (a, b) = (self.relative_luminance(), other.relative_luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

impl From<Color> for iced::Color {
    fn from(color: Color) -> Self {
        Self::from_rgb8(color.red, color.green, color.blue)
    }
}

fn linear(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use crate::theme::Color;

    const BLACK: Color = Color::from_hex(0x00_00_00);
    const WHITE: Color = Color::from_hex(0xFF_FF_FF);

    fn is_close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn reads_red_green_and_blue_from_a_hex_value() {
        let violet = Color::from_hex(0x8B_7C_F6);
        assert_eq!(
            (violet.red(), violet.green(), violet.blue()),
            (0x8B, 0x7C, 0xF6)
        );
    }

    #[test]
    fn black_and_white_have_a_contrast_of_21() {
        assert!(is_close(BLACK.contrast_ratio(WHITE), 21.0));
    }

    #[test]
    fn contrast_does_not_depend_on_the_order_of_the_colors() {
        let violet = Color::from_hex(0x8B_7C_F6);
        assert!(is_close(
            violet.contrast_ratio(BLACK),
            BLACK.contrast_ratio(violet)
        ));
    }

    #[test]
    fn converts_to_an_iced_color() {
        assert_eq!(
            iced::Color::from(Color::from_hex(0x8B_7C_F6)),
            iced::Color::from_rgb8(0x8B, 0x7C, 0xF6)
        );
    }

    #[test]
    fn a_color_has_a_contrast_of_1_with_itself() {
        let grey = Color::from_hex(0x80_80_80);
        assert!(is_close(grey.contrast_ratio(grey), 1.0));
    }
}
