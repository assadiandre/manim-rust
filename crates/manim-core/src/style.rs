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
