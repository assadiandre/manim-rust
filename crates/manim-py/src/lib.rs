//! manim-py: Python bindings (PyO3).
//!
//! Invariant #1 holds here by construction: Python only *builds* the scene
//! and timeline (plain data). The per-frame evaluation and rendering never
//! call back into Python.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use manim_anim::{Animation, Easing, Scene};
use manim_core::constants::{
    DL, DOWN, DR, LEFT, ORIGIN, RIGHT, UL, UP, UR, DEFAULT_ARROW_TIP_LENGTH, DEFAULT_DOT_RADIUS,
    DEFAULT_MOBJECT_TO_EDGE_BUFFER, DEFAULT_MOBJECT_TO_MOBJECT_BUFFER,
};
use manim_core::kurbo::{Point, Vec2};
use manim_core::peniko::Color;
use manim_core::{
    add_angle as rust_add_angle, add_arrow as rust_add_arrow, add_axes as rust_add_axes,
    add_background_rect as rust_add_background_rect, add_brace as rust_add_brace,
    add_cross as rust_add_cross, add_curved_arrow as rust_add_curved_arrow,
    add_double_arrow as rust_add_double_arrow, add_number_line as rust_add_number_line,
    add_number_plane as rust_add_number_plane, add_polar_plane as rust_add_polar_plane,
    add_right_angle as rust_add_right_angle, add_surrounding_rect as rust_add_surrounding_rect,
    add_underline as rust_add_underline, add_vector as rust_add_vector, geometry, palette,
    AxesOpts, Mobject, NodeId, NumberLineOpts, NumberPlaneOpts, PolarPlaneOpts, Style,
};
use manim_render::{render_video, Renderer};
use manim_typst::{
    add_brace_label as rust_add_brace_label, add_decimal as rust_add_decimal, add_math,
    add_tex as add_latex, add_text as rust_add_text, add_title as rust_add_title, MathOptions,
};

fn parse_color(s: &str) -> PyResult<Color> {
    if let Some(c) = palette::named(s) {
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
        "there_and_back" => Ok(Easing::ThereAndBack),
        _ => Err(PyValueError::new_err(format!("unknown easing {s:?}"))),
    }
}

fn parse_direction(s: &str) -> PyResult<Vec2> {
    match s.to_lowercase().as_str() {
        "up" => Ok(UP),
        "down" => Ok(DOWN),
        "left" => Ok(LEFT),
        "right" => Ok(RIGHT),
        "ul" | "up_left" => Ok(UL),
        "ur" | "up_right" => Ok(UR),
        "dl" | "down_left" => Ok(DL),
        "dr" | "down_right" => Ok(DR),
        "origin" | "center" => Ok(ORIGIN),
        _ => Err(PyValueError::new_err(format!(
            "unknown direction {s:?} (use up/down/left/right/ul/ur/dl/dr)"
        ))),
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

    #[pyo3(signature = (x = 0.0, y = 0.0, width = 3.0, height = 2.0, fill = None, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(fill.as_deref(), stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::rect(Point::new(x, y), width, height)).with_style(style),
        ))
    }

    #[pyo3(signature = (x = 0.0, y = 0.0, rx = 1.5, ry = 0.8, fill = None, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_ellipse(
        &mut self,
        x: f64,
        y: f64,
        rx: f64,
        ry: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(fill.as_deref(), stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::ellipse(Point::new(x, y), rx, ry)).with_style(style),
        ))
    }

    #[pyo3(signature = (x = 0.0, y = 0.0, radius = 1.0, start_angle = 0.0, sweep = 3.141592653589793, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_arc(
        &mut self,
        x: f64,
        y: f64,
        radius: f64,
        start_angle: f64,
        sweep: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::arc(Point::new(x, y), radius, start_angle, sweep))
                .with_style(style),
        ))
    }

    #[pyo3(signature = (x = 0.0, y = 0.0, radius = 0.08, fill = Some("white".to_string()), stroke = None, stroke_width = 0.0))]
    fn add_dot(
        &mut self,
        x: f64,
        y: f64,
        radius: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let r = if radius <= 0.0 { DEFAULT_DOT_RADIUS } else { radius };
        let style = build_style(fill.as_deref(), stroke.as_deref(), stroke_width)?;
        Ok(self
            .scene
            .add(Mobject::new(geometry::circle(Point::new(x, y), r)).with_style(style)))
    }

    #[pyo3(signature = (x1, y1, x2, y2, buff = 0.25, tip_length = 0.0, stroke = Some("white".to_string()), stroke_width = 6.0))]
    fn add_arrow(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        buff: f64,
        tip_length: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        let tip = if tip_length <= 0.0 {
            DEFAULT_ARROW_TIP_LENGTH
        } else {
            tip_length
        };
        Ok(rust_add_arrow(
            &mut self.scene.graph,
            Point::new(x1, y1),
            Point::new(x2, y2),
            buff,
            tip,
            style,
        ))
    }

    #[pyo3(signature = (x1, y1, x2, y2, dash = 0.15, gap = 0.1, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_dashed_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        dash: f64,
        gap: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::dashed_line(
                Point::new(x1, y1),
                Point::new(x2, y2),
                dash,
                gap,
            ))
            .with_style(style),
        ))
    }

    #[pyo3(signature = (x_min = -4.0, x_max = 4.0, x_step = 1.0, unit_size = 1.0, include_tip = false, stroke = Some("white".to_string()), stroke_width = 3.0))]
    fn add_number_line(
        &mut self,
        x_min: f64,
        x_max: f64,
        x_step: f64,
        unit_size: f64,
        include_tip: bool,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_number_line(
            &mut self.scene.graph,
            &NumberLineOpts {
                x_min,
                x_max,
                x_step,
                unit_size,
                include_tip,
                ..NumberLineOpts::default()
            },
            style,
        ))
    }

    #[pyo3(signature = (x_min = -4.0, x_max = 4.0, y_min = -3.0, y_max = 3.0, unit_size = 1.0, include_tip = true, stroke = Some("gray".to_string()), stroke_width = 3.0))]
    fn add_axes(
        &mut self,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        unit_size: f64,
        include_tip: bool,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_axes(
            &mut self.scene.graph,
            &AxesOpts {
                x_min,
                x_max,
                y_min,
                y_max,
                unit_size,
                include_tip,
                ..AxesOpts::default()
            },
            style,
        ))
    }

    /// Bake a Python callable `f(x) -> y` into a polyline (authoring-time only).
    #[pyo3(signature = (f, x_min, x_max, samples = 200, unit_size = 1.0, stroke = Some("yellow".to_string()), stroke_width = 4.0))]
    fn add_function(
        &mut self,
        f: Bound<'_, PyAny>,
        x_min: f64,
        x_max: f64,
        samples: usize,
        unit_size: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let n = samples.max(2);
        let mut points = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            let x = x_min + (x_max - x_min) * t;
            let y: f64 = f.call1((x,))?.extract()?;
            points.push(Point::new(x * unit_size, y * unit_size));
        }
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(self
            .scene
            .add(Mobject::new(geometry::polyline(&points)).with_style(style)))
    }

    fn add_group(&mut self, ids: Vec<NodeId>) -> usize {
        self.scene.graph.group_nodes(&ids)
    }

    fn move_to(&mut self, target: NodeId, x: f64, y: f64) {
        self.scene.graph.move_to(target, Point::new(x, y));
    }

    #[pyo3(signature = (target, other, direction = "right", buff = 0.25))]
    fn next_to(
        &mut self,
        target: NodeId,
        other: NodeId,
        direction: &str,
        buff: f64,
    ) -> PyResult<()> {
        let dir = parse_direction(direction)?;
        self.scene.graph.next_to(target, other, dir, buff);
        Ok(())
    }

    fn align_to(&mut self, target: NodeId, other: NodeId, direction: &str) -> PyResult<()> {
        self.scene
            .graph
            .align_to(target, other, parse_direction(direction)?);
        Ok(())
    }

    #[pyo3(signature = (target, direction = "left", buff = 0.5))]
    fn to_edge(&mut self, target: NodeId, direction: &str, buff: f64) -> PyResult<()> {
        self.scene
            .graph
            .to_edge(target, parse_direction(direction)?, buff);
        Ok(())
    }

    fn shift(&mut self, target: NodeId, dx: f64, dy: f64) {
        self.scene.graph.shift(target, Vec2::new(dx, dy));
    }

    #[pyo3(signature = (group, direction = "right", buff = 0.25, center = true))]
    fn arrange(
        &mut self,
        group: NodeId,
        direction: &str,
        buff: f64,
        center: bool,
    ) -> PyResult<()> {
        self.scene
            .graph
            .arrange(group, parse_direction(direction)?, buff, center);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0, easing = "smooth"))]
    fn play_create(&mut self, target: NodeId, duration: f64, easing: &str) -> PyResult<()> {
        let e = parse_easing(easing)?;
        let anims: Vec<_> = manim_anim::path_targets(&self.scene.graph, target)
            .into_iter()
            .map(|id| Animation::create(&self.scene.graph, id, duration).with_easing(e))
            .collect();
        self.scene.play(anims);
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
    fn play_uncreate(&mut self, target: NodeId, duration: f64, easing: &str) -> PyResult<()> {
        let e = parse_easing(easing)?;
        let anims: Vec<_> = manim_anim::path_targets(&self.scene.graph, target)
            .into_iter()
            .map(|id| Animation::uncreate(&self.scene.graph, id, duration).with_easing(e))
            .collect();
        self.scene.play(anims);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0))]
    fn play_write(&mut self, target: NodeId, duration: f64) {
        self.scene.play_write(target, duration);
    }

    #[pyo3(signature = (target, angle, duration = 1.0, easing = "smooth"))]
    fn play_rotate(
        &mut self,
        target: NodeId,
        angle: f64,
        duration: f64,
        easing: &str,
    ) -> PyResult<()> {
        let a = Animation::rotate(&self.scene.graph, target, angle, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0, easing = "smooth"))]
    fn play_grow(&mut self, target: NodeId, duration: f64, easing: &str) -> PyResult<()> {
        let a = Animation::grow_from_center(&self.scene.graph, target, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0))]
    fn play_indicate(&mut self, target: NodeId, duration: f64) {
        self.scene.play([Animation::indicate(&self.scene.graph, target, duration)]);
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

    #[pyo3(signature = (source, x = 0.0, y = 0.0, color = None, font_size_pt = 48.0))]
    fn add_text(
        &mut self,
        source: &str,
        x: f64,
        y: f64,
        color: Option<String>,
        font_size_pt: f64,
    ) -> PyResult<usize> {
        let options = MathOptions {
            font_size_pt,
            color: color.as_deref().map(parse_color).transpose()?,
        };
        let id = rust_add_text(&mut self.scene.graph, source, &options)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.scene.graph.get_mut(id).transform =
            manim_core::kurbo::Affine::translate((x, y));
        Ok(id)
    }

    #[pyo3(signature = (x = 0.0, y = 0.0, stroke = Some("white".to_string()), stroke_width = 6.0))]
    fn add_vector(
        &mut self,
        x: f64,
        y: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_vector(
            &mut self.scene.graph,
            Point::new(x, y),
            style,
        ))
    }

    #[pyo3(signature = (x1, y1, x2, y2, buff = 0.25, stroke = Some("white".to_string()), stroke_width = 6.0))]
    fn add_double_arrow(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        buff: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_double_arrow(
            &mut self.scene.graph,
            Point::new(x1, y1),
            Point::new(x2, y2),
            buff,
            style,
        ))
    }

    #[pyo3(signature = (target, buff = 0.15, corner_radius = 0.0, stroke = Some("yellow".to_string()), stroke_width = 4.0))]
    fn add_surrounding_rect(
        &mut self,
        target: NodeId,
        buff: f64,
        corner_radius: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_surrounding_rect(
            &mut self.scene.graph,
            target,
            buff,
            corner_radius,
            style,
        ))
    }

    #[pyo3(signature = (target, buff = 0.1, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_underline(
        &mut self,
        target: NodeId,
        buff: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_underline(
            &mut self.scene.graph,
            target,
            buff,
            style,
        ))
    }

    #[pyo3(signature = (target, direction = "down", buff = 0.2, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_brace(
        &mut self,
        target: NodeId,
        direction: &str,
        buff: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_brace(
            &mut self.scene.graph,
            target,
            parse_direction(direction)?,
            buff,
            style,
        ))
    }

    #[pyo3(signature = (target, stroke = Some("red".to_string()), stroke_width = 6.0))]
    fn add_cross(&mut self, target: NodeId, stroke: Option<String>, stroke_width: f64) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_cross(&mut self.scene.graph, target, style))
    }

    #[pyo3(signature = (x_min = -7.0, x_max = 7.0, y_min = -4.0, y_max = 4.0, faded_line_ratio = 1, grid = Some("blue_d".to_string()), axis = Some("white".to_string())))]
    fn add_number_plane(
        &mut self,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        faded_line_ratio: u32,
        grid: Option<String>,
        axis: Option<String>,
    ) -> PyResult<usize> {
        let grid_style = build_style(None, grid.as_deref(), 2.0)?;
        let axis_style = build_style(None, axis.as_deref(), 3.0)?;
        Ok(rust_add_number_plane(
            &mut self.scene.graph,
            &NumberPlaneOpts {
                x_min,
                x_max,
                y_min,
                y_max,
                faded_line_ratio,
                ..NumberPlaneOpts::default()
            },
            grid_style,
            axis_style,
        ))
    }

    #[pyo3(signature = (target, direction = "ul", buff = 0.5))]
    fn to_corner(&mut self, target: NodeId, direction: &str, buff: f64) -> PyResult<()> {
        self.scene
            .graph
            .to_corner(target, parse_direction(direction)?, buff);
        Ok(())
    }

    fn set_x(&mut self, target: NodeId, x: f64) {
        self.scene.graph.set_x(target, x);
    }

    fn set_y(&mut self, target: NodeId, y: f64) {
        self.scene.graph.set_y(target, y);
    }

    fn flip(&mut self, target: NodeId, axis: &str) -> PyResult<()> {
        self.scene.graph.flip(target, parse_direction(axis)?);
        Ok(())
    }

    #[pyo3(signature = (group, rows = None, cols = None, buff_x = 0.25, buff_y = 0.25, center = true))]
    fn arrange_in_grid(
        &mut self,
        group: NodeId,
        rows: Option<usize>,
        cols: Option<usize>,
        buff_x: f64,
        buff_y: f64,
        center: bool,
    ) {
        self.scene
            .graph
            .arrange_in_grid(group, rows, cols, buff_x, buff_y, center);
    }

    fn set_z_index(&mut self, target: NodeId, z: i32) {
        self.scene.graph.set_z_index(target, z);
    }

    fn set_width(&mut self, target: NodeId, width: f64) {
        self.scene.graph.set_width(target, width);
    }

    fn set_height(&mut self, target: NodeId, height: f64) {
        self.scene.graph.set_height(target, height);
    }

    fn rotate(&mut self, target: NodeId, angle: f64) {
        self.scene.graph.rotate_about_center(target, angle);
    }

    fn set_color(&mut self, target: NodeId, color: &str) -> PyResult<()> {
        self.scene.graph.set_color(target, parse_color(color)?);
        Ok(())
    }

    #[pyo3(signature = (points, fill = None, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_polygon(
        &mut self,
        points: Vec<(f64, f64)>,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let pts: Vec<Point> = points.into_iter().map(|(x, y)| Point::new(x, y)).collect();
        let style = build_style(fill.as_deref(), stroke.as_deref(), stroke_width)?;
        Ok(self
            .scene
            .add(Mobject::new(geometry::polygon(&pts)).with_style(style)))
    }

    #[pyo3(signature = (sides = 6, radius = 1.0, x = 0.0, y = 0.0, rotation = 0.0, fill = None, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_regular_polygon(
        &mut self,
        sides: usize,
        radius: f64,
        x: f64,
        y: f64,
        rotation: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(fill.as_deref(), stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::regular_polygon(
                Point::new(x, y),
                sides,
                radius,
                rotation,
            ))
            .with_style(style),
        ))
    }

    #[pyo3(signature = (n = 5, outer = 1.0, inner = None, x = 0.0, y = 0.0, rotation = 1.5707963267948966, fill = None, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_star(
        &mut self,
        n: usize,
        outer: f64,
        inner: Option<f64>,
        x: f64,
        y: f64,
        rotation: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(fill.as_deref(), stroke.as_deref(), stroke_width)?;
        Ok(self.scene.add(
            Mobject::new(geometry::star(Point::new(x, y), n, outer, inner, rotation))
                .with_style(style),
        ))
    }

    #[pyo3(signature = (x1, y1, x2, y2, sweep = 1.5707963267948966, stroke = Some("white".to_string()), stroke_width = 6.0))]
    fn add_curved_arrow(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        sweep: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_curved_arrow(
            &mut self.scene.graph,
            Point::new(x1, y1),
            Point::new(x2, y2),
            sweep,
            style,
        ))
    }

    #[pyo3(signature = (vx, vy, x1, y1, x2, y2, radius = 0.4, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_angle(
        &mut self,
        vx: f64,
        vy: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        radius: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_angle(
            &mut self.scene.graph,
            Point::new(vx, vy),
            Point::new(x1, y1),
            Point::new(x2, y2),
            radius,
            style,
        ))
    }

    #[pyo3(signature = (vx, vy, x1, y1, x2, y2, size = 0.3, stroke = Some("white".to_string()), stroke_width = 4.0))]
    fn add_right_angle(
        &mut self,
        vx: f64,
        vy: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        size: f64,
        stroke: Option<String>,
        stroke_width: f64,
    ) -> PyResult<usize> {
        let style = build_style(None, stroke.as_deref(), stroke_width)?;
        Ok(rust_add_right_angle(
            &mut self.scene.graph,
            Point::new(vx, vy),
            Point::new(x1, y1),
            Point::new(x2, y2),
            size,
            style,
        ))
    }

    #[pyo3(signature = (radius = 4.0, radius_step = 1.0, azimuth_divisions = 12, faded_line_ratio = 1, grid = Some("blue_d".to_string()), axis = Some("white".to_string())))]
    fn add_polar_plane(
        &mut self,
        radius: f64,
        radius_step: f64,
        azimuth_divisions: u32,
        faded_line_ratio: u32,
        grid: Option<String>,
        axis: Option<String>,
    ) -> PyResult<usize> {
        let grid_style = build_style(None, grid.as_deref(), 2.0)?;
        let axis_style = build_style(None, axis.as_deref(), 3.0)?;
        Ok(rust_add_polar_plane(
            &mut self.scene.graph,
            &PolarPlaneOpts {
                radius,
                radius_step,
                azimuth_divisions,
                faded_line_ratio,
                ..PolarPlaneOpts::default()
            },
            grid_style,
            axis_style,
        ))
    }

    #[pyo3(signature = (source, color = None, font_size_pt = 48.0))]
    fn add_title(
        &mut self,
        source: &str,
        color: Option<String>,
        font_size_pt: f64,
    ) -> PyResult<usize> {
        let options = MathOptions {
            font_size_pt,
            color: color.as_deref().map(parse_color).transpose()?,
        };
        rust_add_title(&mut self.scene.graph, source, &options)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[pyo3(signature = (value, places = 2, x = 0.0, y = 0.0, color = None, font_size_pt = 48.0))]
    fn add_decimal(
        &mut self,
        value: f64,
        places: usize,
        x: f64,
        y: f64,
        color: Option<String>,
        font_size_pt: f64,
    ) -> PyResult<usize> {
        let options = MathOptions {
            font_size_pt,
            color: color.as_deref().map(parse_color).transpose()?,
        };
        let id = rust_add_decimal(&mut self.scene.graph, value, places, &options)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.scene.graph.get_mut(id).transform =
            manim_core::kurbo::Affine::translate((x, y));
        Ok(id)
    }

    #[pyo3(signature = (target, label, direction = "down", font_size_pt = 36.0))]
    fn add_brace_label(
        &mut self,
        target: NodeId,
        label: &str,
        direction: &str,
        font_size_pt: f64,
    ) -> PyResult<usize> {
        let options = MathOptions {
            font_size_pt,
            color: None,
        };
        rust_add_brace_label(
            &mut self.scene.graph,
            target,
            parse_direction(direction)?,
            label,
            &options,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[pyo3(signature = (target, buff = 0.15, fill = "black", opacity = 0.75))]
    fn add_background_rect(
        &mut self,
        target: NodeId,
        buff: f64,
        fill: &str,
        opacity: f32,
    ) -> PyResult<usize> {
        Ok(rust_add_background_rect(
            &mut self.scene.graph,
            target,
            buff,
            parse_color(fill)?,
            opacity,
        ))
    }

    #[pyo3(signature = (target, path, duration = 1.0, easing = "smooth"))]
    fn play_move_along_path(
        &mut self,
        target: NodeId,
        path: NodeId,
        duration: f64,
        easing: &str,
    ) -> PyResult<()> {
        let a = Animation::move_along_path(&self.scene.graph, target, path, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (x, y, duration = 1.0, color = "yellow"))]
    fn play_flash(&mut self, x: f64, y: f64, duration: f64, color: &str) -> PyResult<()> {
        self.scene
            .play_flash(Point::new(x, y), duration, parse_color(color)?);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0))]
    fn play_show_passing_flash(&mut self, target: NodeId, duration: f64) {
        self.scene.play_show_passing_flash(target, duration);
    }

    #[pyo3(signature = (target, x, y, duration = 1.0, easing = "smooth"))]
    fn play_grow_from_point(
        &mut self,
        target: NodeId,
        x: f64,
        y: f64,
        duration: f64,
        easing: &str,
    ) -> PyResult<()> {
        let a = Animation::grow_from_point(&self.scene.graph, target, Point::new(x, y), duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0))]
    fn play_spin_in(&mut self, target: NodeId, duration: f64) {
        self.scene.play_spin_in(target, duration);
    }

    #[pyo3(signature = (target, duration = 1.0, easing = "smooth"))]
    fn play_shrink(&mut self, target: NodeId, duration: f64, easing: &str) -> PyResult<()> {
        let a = Animation::shrink_to_center(&self.scene.graph, target, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (target, color, duration = 1.0, easing = "smooth"))]
    fn play_recolor(
        &mut self,
        target: NodeId,
        color: &str,
        duration: f64,
        easing: &str,
    ) -> PyResult<()> {
        let a = Animation::recolor(&self.scene.graph, target, parse_color(color)?, duration)
            .with_easing(parse_easing(easing)?);
        self.scene.play([a]);
        Ok(())
    }

    #[pyo3(signature = (target, duration = 1.0))]
    fn play_wiggle(&mut self, target: NodeId, duration: f64) {
        self.scene
            .play([Animation::wiggle(&self.scene.graph, target, duration)]);
    }

    #[pyo3(signature = (target, duration = 1.0))]
    fn play_draw_border_then_fill(&mut self, target: NodeId, duration: f64) {
        self.scene.play_draw_border_then_fill(target, duration);
    }

    #[pyo3(signature = (target, duration = 1.2, color = "yellow"))]
    fn play_circumscribe(&mut self, target: NodeId, duration: f64, color: &str) -> PyResult<()> {
        self.scene
            .play_circumscribe(target, duration, parse_color(color)?);
        Ok(())
    }

    #[pyo3(signature = (dx, dy, duration = 1.0))]
    fn play_camera_shift(&mut self, dx: f64, dy: f64, duration: f64) {
        self.scene.play_camera_shift(Vec2::new(dx, dy), duration);
    }

    #[pyo3(signature = (factor, duration = 1.0))]
    fn play_camera_zoom(&mut self, factor: f64, duration: f64) {
        self.scene.play_camera_zoom(factor, duration);
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
        renderer.camera = self.scene.timeline.camera_at(time);
        renderer
            .save_png(&mut sim, std::path::Path::new(path))
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

#[pymodule]
fn manim_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyScene>()?;
    m.add("ORIGIN", (ORIGIN.x, ORIGIN.y))?;
    m.add("UP", (UP.x, UP.y))?;
    m.add("DOWN", (DOWN.x, DOWN.y))?;
    m.add("LEFT", (LEFT.x, LEFT.y))?;
    m.add("RIGHT", (RIGHT.x, RIGHT.y))?;
    m.add("UL", (UL.x, UL.y))?;
    m.add("UR", (UR.x, UR.y))?;
    m.add("DL", (DL.x, DL.y))?;
    m.add("DR", (DR.x, DR.y))?;
    m.add("DEFAULT_MOBJECT_TO_MOBJECT_BUFFER", DEFAULT_MOBJECT_TO_MOBJECT_BUFFER)?;
    m.add("DEFAULT_MOBJECT_TO_EDGE_BUFFER", DEFAULT_MOBJECT_TO_EDGE_BUFFER)?;
    Ok(())
}
