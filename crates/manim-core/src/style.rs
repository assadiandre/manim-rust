use peniko::Color;

/// Common colors as functions (avoids relying on const-fn stability in `color`).
pub mod palette {
    use peniko::Color;

    pub fn white() -> Color {
        Color::from_rgba8(255, 255, 255, 255)
    }
    pub fn black() -> Color {
        Color::from_rgba8(0, 0, 0, 255)
    }
    pub fn blue() -> Color {
        Color::from_rgba8(88, 196, 221, 255) // Manim BLUE_C
    }
    pub fn green() -> Color {
        Color::from_rgba8(131, 193, 103, 255)
    }
    pub fn yellow() -> Color {
        Color::from_rgba8(255, 255, 0, 255)
    }
    pub fn red() -> Color {
        Color::from_rgba8(252, 98, 85, 255)
    }
    pub fn gray() -> Color {
        Color::from_rgba8(136, 136, 136, 255)
    }
    pub fn teal() -> Color {
        Color::from_rgba8(92, 208, 179, 255) // TEAL_C
    }
    pub fn orange() -> Color {
        Color::from_rgba8(255, 134, 47, 255)
    }
    pub fn purple() -> Color {
        Color::from_rgba8(154, 114, 172, 255) // PURPLE_C
    }
    pub fn pink() -> Color {
        Color::from_rgba8(209, 71, 189, 255)
    }
    pub fn gold() -> Color {
        Color::from_rgba8(240, 172, 95, 255) // GOLD_C
    }
    pub fn maroon() -> Color {
        Color::from_rgba8(197, 95, 115, 255) // MAROON_C
    }
    pub fn blue_d() -> Color {
        Color::from_rgba8(41, 171, 202, 255) // BLUE_D
    }
    pub fn grey() -> Color {
        Color::from_rgba8(136, 136, 136, 255)
    }
    pub fn pure_yellow() -> Color {
        Color::from_rgba8(255, 255, 0, 255)
    }

    fn hex(rgb: u32) -> Color {
        Color::from_rgba8(
            ((rgb >> 16) & 0xff) as u8,
            ((rgb >> 8) & 0xff) as u8,
            (rgb & 0xff) as u8,
            255,
        )
    }

    /// ManimCE color names (case-insensitive). `yellow` stays `#FFFF00` so
    /// existing scenes/goldens do not shift; CE's `YELLOW` is `yellow_c`.
    pub fn named(s: &str) -> Option<Color> {
        let k = s.trim().to_ascii_lowercase().replace('-', "_");
        Some(match k.as_str() {
            "white" => white(),
            "black" => black(),
            "blue" | "blue_c" => blue(),
            "blue_a" => hex(0xC7E9F1),
            "blue_b" => hex(0x9CDCEB),
            "blue_d" => blue_d(),
            "blue_e" | "dark_blue" => hex(0x236B8E),
            "teal" | "teal_c" => teal(),
            "teal_a" => hex(0xACEAD7),
            "teal_b" => hex(0x76DDC0),
            "teal_d" => hex(0x55C1A7),
            "teal_e" => hex(0x49A88F),
            "green" | "green_c" => green(),
            "green_a" => hex(0xC9E2AE),
            "green_b" => hex(0xA6CF8C),
            "green_d" => hex(0x77B05D),
            "green_e" => hex(0x699C52),
            "yellow" | "pure_yellow" => yellow(),
            "yellow_a" => hex(0xFFF1B6),
            "yellow_b" => hex(0xFFEA94),
            "yellow_c" => hex(0xF7D96F),
            "yellow_d" => hex(0xF4D345),
            "yellow_e" => hex(0xE8C11C),
            "gold" | "gold_c" => gold(),
            "gold_a" => hex(0xF7C797),
            "gold_b" => hex(0xF9B775),
            "gold_d" => hex(0xE1A158),
            "gold_e" => hex(0xC78D46),
            "red" | "red_c" => red(),
            "red_a" => hex(0xF7A1A3),
            "red_b" => hex(0xFF8080),
            "red_d" => hex(0xE65A4C),
            "red_e" => hex(0xCF5044),
            "maroon" | "maroon_c" => maroon(),
            "maroon_a" => hex(0xECABC1),
            "maroon_b" => hex(0xEC92AB),
            "maroon_d" => hex(0xA24D61),
            "maroon_e" => hex(0x94424F),
            "purple" | "purple_c" => purple(),
            "purple_a" => hex(0xCAA3E8),
            "purple_b" => hex(0xB189C6),
            "purple_d" => hex(0x715582),
            "purple_e" => hex(0x644172),
            "pink" => pink(),
            "light_pink" => hex(0xDC75CD),
            "orange" => orange(),
            "light_brown" => hex(0xCD853F),
            "dark_brown" => hex(0x8B4513),
            "gray_brown" | "grey_brown" => hex(0x736357),
            "gray" | "grey" | "gray_c" | "grey_c" => gray(),
            "gray_a" | "grey_a" | "lighter_gray" | "lighter_grey" => hex(0xDDDDDD),
            "gray_b" | "grey_b" | "light_gray" | "light_grey" => hex(0xBBBBBB),
            "gray_d" | "grey_d" | "dark_gray" | "dark_grey" => hex(0x444444),
            "gray_e" | "grey_e" | "darker_gray" | "darker_grey" => hex(0x222222),
            "pure_red" => hex(0xFF0000),
            "pure_green" => hex(0x00FF00),
            "pure_blue" => hex(0x0000FF),
            "pure_cyan" => hex(0x00FFFF),
            "pure_magenta" => hex(0xFF00FF),
            "logo_white" => hex(0xECE7E2),
            "logo_green" => hex(0x87C2A5),
            "logo_blue" => hex(0x525893),
            "logo_red" => hex(0xE07A5F),
            "logo_black" => hex(0x343434),
            _ => return None,
        })
    }
}

/// Multiply a color's alpha by `opacity` (0..=1).
pub fn with_opacity(c: Color, opacity: f32) -> Color {
    let rgba = c.to_rgba8();
    let a = (rgba.a as f32 * opacity.clamp(0.0, 1.0)).round() as u8;
    Color::from_rgba8(rgba.r, rgba.g, rgba.b, a)
}

/// Paint + opacity for a vector mobject. Manim-flavored defaults: white
/// stroke, no fill.
#[derive(Clone, Debug)]
pub struct Style {
    pub fill: Option<Color>,
    pub fill_opacity: f32,
    pub stroke: Option<Color>,
    /// Stroke width in *device pixels*; does not scale with transforms
    /// (Manim semantics — 4.0 matches Manim's DEFAULT_STROKE_WIDTH).
    pub stroke_width: f64,
    pub stroke_opacity: f32,
    /// Global multiplier applied to both fill and stroke.
    pub opacity: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fill: None,
            fill_opacity: 1.0,
            stroke: Some(palette::white()),
            stroke_width: 4.0,
            stroke_opacity: 1.0,
            opacity: 1.0,
        }
    }
}

impl Style {
    pub fn filled(fill: Color) -> Self {
        Self {
            fill: Some(fill),
            ..Default::default()
        }
    }

    pub fn with_fill(mut self, fill: Color) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn with_stroke(mut self, stroke: Color, width: f64) -> Self {
        self.stroke = Some(stroke);
        self.stroke_width = width;
        self
    }

    pub fn no_stroke(mut self) -> Self {
        self.stroke = None;
        self
    }

    pub fn no_fill(mut self) -> Self {
        self.fill = None;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Fill color with all opacities folded in; `None` means "don't paint".
    pub fn effective_fill(&self) -> Option<Color> {
        self.fill
            .map(|c| with_opacity(c, self.fill_opacity * self.opacity))
    }

    /// (stroke color, width) with opacities folded in.
    pub fn effective_stroke(&self) -> Option<(Color, f64)> {
        self.stroke
            .map(|c| (with_opacity(c, self.stroke_opacity * self.opacity), self.stroke_width))
    }

    /// True when a frame with this style is guaranteed to paint nothing.
    pub fn is_invisible(&self) -> bool {
        self.opacity <= 0.0 || (self.effective_fill().is_none() && self.effective_stroke().is_none())
    }
}

/// Linear interpolation in sRGB bytes (Manim's `interpolate_color` flavor).
pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let a = a.to_rgba8();
    let b = b.to_rgba8();
    let mix = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    Color::from_rgba8(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b), mix(a.a, b.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_yellow_stays_pure_for_compat() {
        let y = palette::named("yellow").unwrap().to_rgba8();
        assert_eq!((y.r, y.g, y.b), (255, 255, 0));
        let yc = palette::named("YELLOW_C").unwrap().to_rgba8();
        assert_eq!((yc.r, yc.g, yc.b), (0xF7, 0xD9, 0x6F));
    }
}
