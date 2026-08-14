//! manim-py: Python bindings (PyO3).
//!
//! Invariant #1 holds here by construction: Python only *builds* the scene
//! and timeline (plain data). The per-frame evaluation and rendering never
//! call back into Python.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use manim_anim::{Animation, Easing, Scene};
use manim_core::kurbo::{Point, Vec2};
use manim_core::peniko::Color;
use manim_core::{geometry, palette, Mobject, NodeId, Style};
use manim_render::{render_video, Renderer};
use manim_typst::{add_math, add_tex as add_latex, MathOptions};

fn parse_color(s: &str) -> PyResult<Color> {
    let named = |name: &str| match name {
        "white" => Some(palette::white()),
        "black" => Some(palette::black()),
        "blue" => Some(palette::blue()),
        "green" => Some(palette::green()),
        "yellow" => Some(palette::yellow()),
        "red" => Some(palette::red()),
        "gray" | "grey" => Some(palette::gray()),
        _ => None,
    };
    if let Some(c) = named(&s.to_lowercase()) {
        return Ok(c);
    }
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16);
        let g = u8::from_str_radix(&hex[2..4], 16);
        let b = u8::from_str_radix(&hex[4..6], 16);
        if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
            return Ok(Color::from_rgba8(r, g, b, 255));
        }
    }
    Err(PyValueError::new_err(format!(
        "unknown color {s:?} (use a name like \"blue\" or \"#58c4dd\")"
    )))
}

fn parse_easing(s: &str) -> PyResult<Easing> {
    match s.to_lowercase().as_str() {
        "linear" => Ok(Easing::Linear),
        "smooth" => Ok(Easing::Smooth),
        "ease_in" | "ease_in_cubic" => Ok(Easing::EaseInCubic),
        "ease_out" | "ease_out_cubic" => Ok(Easing::EaseOutCubic),
        "ease_in_out" | "ease_in_out_cubic" => Ok(Easing::EaseInOutCubic),
        _ => Err(PyValueError::new_err(format!("unknown easing {s:?}"))),
    }
}

fn build_style(fill: Option<&str>, stroke: Option<&str>, stroke_width: f64) -> PyResult<Style> {
    let mut style = Style::default().no_fill().no_stroke();
    if let Some(f) = fill {
        style = style.with_fill(parse_color(f)?);
    }
    if let Some(s) = stroke {
        style = style.with_stroke(parse_color(s)?, stroke_width);
    }
    Ok(style)
}

/// A scene being authored. Mirrors the Rust `Scene` semantics: `play_*`
/// calls run sequentially, each starting when the previous ended.
#[pyclass(name = "Scene")]
struct PyScene {
    scene: Scene,
    width: u32,
    height: u32,
    background: Color,
}

#[pymethods]
impl PyScene {
    #[new]
    #[pyo3(signature = (width = 1920, height = 1080, background = "black"))]
    fn new(width: u32, height: u32, background: &str) -> PyResult<Self> {
        Ok(Self {
            scene: Scene::new(),
            width,
            height,
            background: parse_color(background)?,
        })
    }

    #[pyo3(signature = (x = 0.0, y = 0.0, radius = 1.0, fill = None, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_circle(
        &mut self,
        x: f64,
        y: f64,
        radius: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(fill.as_deref(), stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::circle(Point::new(x, y), radius)).with_style(style),
        ))
    }

    #[pyo3(signature = (x = 0.0, y = 0.0, side = 2.0, fill = None, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_square(
        &mut self,
        x: f64,
        y: f64,
        side: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(fill.as_deref(), stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::square(Point::new(x, y), side)).with_style(style),
        ))
    }

    #[pyo3(signature = (x1, y1, x2, y2, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::line(Point::new(x1, y1), Point::new(x2, y2)))
                .with_style(style),
        ))
    }

    /// Typeset math and add it at (x, y). `syntax` is "latex" (default —
    /// LaTeX math via mitex, matching Manim's MathTex) or "typst" (native
    /// typst math syntax). Returns a group id.
    #[pyo3(signature = (source, x = 0.0, y = 0.0, color = None, font_size_pt = 48.0, syntax = "latex"))]
    fn add_tex(
        &mut self,
        source: &str,
        x: f64,
        y: f64,
        color: Option<String>,
        font_size_pt: f64,
        syntax: &str,
    ) -> PyResult<usize> {
        let options = MathOptions {
            font_size_pt,
            color: color.as_deref().map(parse_color).transpose()?,
        };
        let result = match syntax {
            "latex" => add_latex(&mut self.scene.graph, source, &options),
            "typst" => add_math(&mut self.scene.graph, source, &options),
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown syntax {other:?} (expected \"latex\" or \"typst\")"
                )))
            }
        };
        let id = result.map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.scene.graph.get_mut(id).transform =
            manim_core::kurbo::Affine::translate((x, y));
        Ok(id)
    }

    #[pyo3(signature = (target, duration = 1.0, easing = "smooth"))]
    fn play_create(&mut self, target: NodeId, duration: f64, easing: &str) -> PyResult<()> {
        let a = Animation::create(&self.scene.graph, target, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    /// Morph `target` into the shape of `other`. `other` is consumed as a
    /// reference and removed from the scene (Manim's `Transform(a, b)`
    /// semantics where `b` was never added).
    #[pyo3(signature = (target, other, duration = 1.0, easing = "smooth"))]
    fn play_morph(
        &mut self,
        target: NodeId,
        other: NodeId,
        duration: f64,
        easing: &str,
    ) -> PyResult<()> {
        let to = self.scene.graph.get(other).path.clone();
        let a = Animation::morph(&self.scene.graph, target, to, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        self.scene.graph.remove(other);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0, easing = "smooth"))]
    fn play_fade_in(&mut self, target: NodeId, duration: f64, easing: &str) -> PyResult<()> {
        let a = Animation::fade_in(&self.scene.graph, target, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0, easing = "smooth"))]
    fn play_fade_out(&mut self, target: NodeId, duration: f64, easing: &str) -> PyResult<()> {
        let a = Animation::fade_out(&self.scene.graph, target, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (target, dx, dy, duration = 1.0, easing = "smooth"))]
    fn play_shift(
        &mut self,
        target: NodeId,
        dx: f64,
        dy: f64,
        duration: f64,
        easing: &str,
    ) -> PyResult<()> {
        let a = Animation::shift(&self.scene.graph, target, Vec2::new(dx, dy), duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (target, factor, duration = 1.0, easing = "smooth"))]
    fn play_scale(
        &mut self,
        target: NodeId,
        factor: f64,
        duration: f64,
        easing: &str,
    ) -> PyResult<()> {
        let a = Animation::scale(&self.scene.graph, target, factor, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    fn wait(&mut self, duration: f64) {
        self.scene.wait(duration);
    }

    fn duration(&self) -> f64 {
        self.scene.duration()
    }

    #[pyo3(signature = (path, fps = 60))]
    fn render(&mut self, path: &str, fps: u32) -> PyResult<()> {
        let mut renderer = Renderer::new(self.width, self.height, self.background)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        render_video(
            &self.scene.graph,
            &self.scene.timeline,
            &mut renderer,
            fps,
            std::path::Path::new(path),
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Render a single frame at `time` to a PNG.
    #[pyo3(signature = (path, time = 0.0))]
    fn save_png(&mut self, path: &str, time: f64) -> PyResult<()> {
        let mut renderer = Renderer::new(self.width, self.height, self.background)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let mut sim = self.scene.graph.clone();
        self.scene.timeline.apply(&mut sim, time);
        renderer
            .save_png(&mut sim, std::path::Path::new(path))
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

#[pymodule]
fn manim_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyScene>()?;
    Ok(())
}
