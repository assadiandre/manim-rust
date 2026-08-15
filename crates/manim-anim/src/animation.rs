//! Declarative animations: pure data, no closures (invariant #1).
//!
//! Every animation snapshots the state it interpolates *from* at construction
//! time, so evaluation at any t is stateless and the render loop can apply
//! the whole timeline to a fresh scene clone each frame.

use kurbo::{Affine, BezPath, Point, Vec2};
use manim_core::geometry;
use manim_core::peniko::Color;
use manim_core::style::lerp_color;
use manim_core::{DigitAtlas, NodeId, SceneGraph, Style};

use crate::easing::Easing;

/// Samples used when resampling paths for morphs/trims.
pub const MORPH_SAMPLES: usize = 512;

#[derive(Clone, Debug)]
pub enum AnimationKind {
    /// Stroke reveal along the path (Manim's `Create`/`ShowCreation`).
    Create { full: BezPath },
    /// Reverse of Create (Manim `Uncreate`).
    Uncreate { full: BezPath },
    /// Opacity 0 -> snapshot.
    FadeIn { to_opacity: f32 },
    /// Opacity snapshot -> 0.
    FadeOut { from_opacity: f32 },
    Shift { from: Affine, delta: Vec2 },
    Scale { from: Affine, about: Point, factor: f64 },
    Rotate { from: Affine, about: Point, angle: f64 },
    /// Scale 0 → 1 about a pivot (Manim `GrowFromCenter`).
    Grow { from: Affine, about: Point },
    /// Path morph via common resampling.
    Morph { from: BezPath, to: BezPath },
    /// Interpolate fill and/or stroke color (Manim `set_color` animate).
    Recolor {
        from_fill: Option<Color>,
        to_fill: Option<Color>,
        from_stroke: Option<Color>,
        to_stroke: Option<Color>,
    },
    /// Stroke reveal, then fade the fill in (Manim `DrawBorderThenFill`).
    DrawBorderThenFill {
        full: BezPath,
        fill: Option<Color>,
        fill_opacity: f32,
    },
    /// Oscillating rotate that returns to rest (Manim `Wiggle`).
    Wiggle {
        from: Affine,
        about: Point,
        angle: f64,
        wiggles: f64,
    },
    /// A sliding window along the path (Manim `ShowPassingFlash`).
    ShowPassingFlash { full: BezPath, time_width: f64 },
    /// Center follows a path (Manim `MoveAlongPath`).
    MoveAlongPath {
        from: Affine,
        about: Point,
        path: BezPath,
    },
    /// Scale 0→1 while spinning (Manim `SpinInFromNothing`).
    SpinIn {
        from: Affine,
        about: Point,
        angle: f64,
    },
    /// Rebuild a decimal from a baked glyph atlas (Manim `ChangingDecimal`).
    /// Typst runs at authoring time; the frame loop only concatenates outlines.
    ChangingDecimal {
        from: f64,
        to: f64,
        places: usize,
        atlas: DigitAtlas,
    },
    /// Translate and scale in one transform (Manim `FadeTransform` stretch).
    /// `Shift` and `Scale` cannot run together — they share `Prop::Transform`.
    Travel {
        from: Affine,
        delta: Vec2,
        about: Point,
        scale: f64,
    },
    /// Standing wave along the path that rests at the endpoints (Manim `ApplyWave`).
    ApplyWave {
        full: BezPath,
        amplitude: f64,
        ripples: f64,
    },
    /// Animate from the current snapshot to a prior `save_state` (Manim `Restore`).
    /// Writes transform, style, and path in one kind so they do not fight.
    Restore {
        from_path: BezPath,
        to_path: BezPath,
        from_transform: Affine,
        to_transform: Affine,
        from_style: Style,
        to_style: Style,
    },
}

/// The scene property an animation drives. Animations are grouped by
/// (target, property) during evaluation: at any time t, the last started
/// animation in the group wins; before the first one starts, the first one
/// is applied at alpha 0 (restoring the pre-animation state).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Prop {
    Path,
    Opacity,
    Transform,
    Color,
}

impl AnimationKind {
    pub fn prop(&self) -> Prop {
        match self {
            AnimationKind::Create { .. }
            | AnimationKind::Uncreate { .. }
            | AnimationKind::Morph { .. }
            | AnimationKind::DrawBorderThenFill { .. }
            | AnimationKind::ShowPassingFlash { .. }
            | AnimationKind::ChangingDecimal { .. }
            | AnimationKind::ApplyWave { .. } => Prop::Path,
            AnimationKind::FadeIn { .. } | AnimationKind::FadeOut { .. } => Prop::Opacity,
            AnimationKind::Shift { .. }
            | AnimationKind::Scale { .. }
            | AnimationKind::Rotate { .. }
            | AnimationKind::Grow { .. }
            | AnimationKind::Wiggle { .. }
            | AnimationKind::MoveAlongPath { .. }
            | AnimationKind::SpinIn { .. }
            | AnimationKind::Travel { .. }
            | AnimationKind::Restore { .. } => Prop::Transform,
            AnimationKind::Recolor { .. } => Prop::Color,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub target: NodeId,
    pub kind: AnimationKind,
    pub start: f64,
    pub duration: f64,
    pub easing: Easing,
}

impl Animation {
    fn new(target: NodeId, kind: AnimationKind, start: f64, duration: f64) -> Self {
        Self {
            target,
            kind,
            start,
            duration: duration.max(1e-9),
            easing: Easing::default(),
        }
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn create(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let full = scene.get(target).path.clone();
        Self::new(target, AnimationKind::Create { full }, 0.0, duration)
    }

    pub fn fade_in(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let to_opacity = scene.get(target).style.opacity;
        Self::new(target, AnimationKind::FadeIn { to_opacity }, 0.0, duration)
    }

    pub fn fade_out(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let from_opacity = scene.get(target).style.opacity;
        Self::new(target, AnimationKind::FadeOut { from_opacity }, 0.0, duration)
    }

    pub fn shift(scene: &SceneGraph, target: NodeId, delta: Vec2, duration: f64) -> Self {
        let from = scene.get(target).transform;
        Self::new(target, AnimationKind::Shift { from, delta }, 0.0, duration)
    }

    pub fn scale(scene: &SceneGraph, target: NodeId, factor: f64, duration: f64) -> Self {
        let from = scene.get(target).transform;
        let about = scene.local_pivot(target);
        Self::new(target, AnimationKind::Scale { from, about, factor }, 0.0, duration)
    }

    pub fn uncreate(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let full = scene.get(target).path.clone();
        Self::new(target, AnimationKind::Uncreate { full }, 0.0, duration)
    }

    pub fn rotate(scene: &SceneGraph, target: NodeId, angle: f64, duration: f64) -> Self {
        let from = scene.get(target).transform;
        let about = scene.local_pivot(target);
        Self::new(target, AnimationKind::Rotate { from, about, angle }, 0.0, duration)
    }

    pub fn grow_from_center(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let from = scene.get(target).transform;
        let about = scene.local_pivot(target);
        Self::new(target, AnimationKind::Grow { from, about }, 0.0, duration)
    }

    /// Pulse scale (Manim `Indicate`): grows then returns via `ThereAndBack`.
    pub fn indicate(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        Self::scale(scene, target, 1.2, duration).with_easing(Easing::ThereAndBack)
    }

    pub fn morph(scene: &SceneGraph, target: NodeId, to: BezPath, duration: f64) -> Self {
        let from = scene.get(target).path.clone();
        Self::new(target, AnimationKind::Morph { from, to }, 0.0, duration)
    }

    pub fn recolor(scene: &SceneGraph, target: NodeId, to: Color, duration: f64) -> Self {
        let s = &scene.get(target).style;
        Self::new(
            target,
            AnimationKind::Recolor {
                from_fill: s.fill,
                to_fill: s.fill.map(|_| to),
                from_stroke: s.stroke,
                to_stroke: s.stroke.map(|_| to),
            },
            0.0,
            duration,
        )
    }

    pub fn draw_border_then_fill(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let m = scene.get(target);
        Self::new(
            target,
            AnimationKind::DrawBorderThenFill {
                full: m.path.clone(),
                fill: m.style.fill,
                fill_opacity: m.style.fill_opacity,
            },
            0.0,
            duration,
        )
    }

    pub fn show_passing_flash(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let full = scene.get(target).path.clone();
        Self::new(
            target,
            AnimationKind::ShowPassingFlash {
                full,
                time_width: 0.1,
            },
            0.0,
            duration,
        )
    }

    pub fn move_along_path(
        scene: &SceneGraph,
        target: NodeId,
        path: NodeId,
        duration: f64,
    ) -> Self {
        let from = scene.get(target).transform;
        let about = scene.local_pivot(target);
        let world = scene.world_transform(path);
        let baked = world * scene.get(path).path.clone();
        Self::new(
            target,
            AnimationKind::MoveAlongPath {
                from,
                about,
                path: baked,
            },
            0.0,
            duration,
        )
    }

    pub fn grow_from_point(
        scene: &SceneGraph,
        target: NodeId,
        world_point: Point,
        duration: f64,
    ) -> Self {
        let from = scene.get(target).transform;
        let about = scene.world_transform(target).inverse() * world_point;
        Self::new(target, AnimationKind::Grow { from, about }, 0.0, duration)
    }

    pub fn grow_from_edge(scene: &SceneGraph, target: NodeId, edge: Vec2, duration: f64) -> Self {
        let bb = scene.local_family_bbox(target);
        let about = Point::new(
            if edge.x > 0.0 {
                bb.x1
            } else if edge.x < 0.0 {
                bb.x0
            } else {
                bb.center().x
            },
            if edge.y > 0.0 {
                bb.y1
            } else if edge.y < 0.0 {
                bb.y0
            } else {
                bb.center().y
            },
        );
        let from = scene.get(target).transform;
        Self::new(target, AnimationKind::Grow { from, about }, 0.0, duration)
    }

    pub fn shrink_to_center(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        Self::scale(scene, target, 0.0, duration)
    }

    pub fn spin_in(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let from = scene.get(target).transform;
        let about = scene.local_pivot(target);
        Self::new(
            target,
            AnimationKind::SpinIn {
                from,
                about,
                angle: std::f64::consts::PI,
            },
            0.0,
            duration,
        )
    }

    pub fn changing_decimal(
        target: NodeId,
        from: f64,
        to: f64,
        places: usize,
        atlas: DigitAtlas,
        duration: f64,
    ) -> Self {
        Self::new(
            target,
            AnimationKind::ChangingDecimal {
                from,
                to,
                places,
                atlas,
            },
            0.0,
            duration,
        )
    }

    pub fn travel(
        scene: &SceneGraph,
        target: NodeId,
        delta: Vec2,
        scale: f64,
        duration: f64,
    ) -> Self {
        let from = scene.get(target).transform;
        let about = scene.local_pivot(target);
        Self::new(
            target,
            AnimationKind::Travel {
                from,
                delta,
                about,
                scale,
            },
            0.0,
            duration,
        )
    }

    /// Interpolate from the *current* path/transform/style to the last
    /// `save_state` snapshot. If none was saved, `from` and `to` match.
    /// CE `TransformFromCopy`: animate `target` from looking like `source`
    /// to its own current path/transform/style. Source is not mutated.
    pub fn transform_from_copy(
        scene: &SceneGraph,
        source: NodeId,
        target: NodeId,
        duration: f64,
    ) -> Self {
        let src = scene.get(source);
        let dst = scene.get(target);
        Self::new(
            target,
            AnimationKind::Restore {
                from_path: src.path.clone(),
                to_path: dst.path.clone(),
                from_transform: src.transform,
                to_transform: dst.transform,
                from_style: src.style.clone(),
                to_style: dst.style.clone(),
            },
            0.0,
            duration,
        )
    }

    pub fn restore(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let (from_path, from_transform, from_style) = {
            let m = scene.get(target);
            (m.path.clone(), m.transform, m.style.clone())
        };
        let (to_path, to_transform, to_style) = match scene.saved_state(target) {
            Some(saved) => (saved.path.clone(), saved.transform, saved.style.clone()),
            None => (from_path.clone(), from_transform, from_style.clone()),
        };
        Self::new(
            target,
            AnimationKind::Restore {
                from_path,
                to_path,
                from_transform,
                to_transform,
                from_style,
                to_style,
            },
            0.0,
            duration,
        )
    }

    pub fn apply_wave(
        scene: &SceneGraph,
        target: NodeId,
        amplitude: f64,
        ripples: f64,
        duration: f64,
    ) -> Self {
        let full = scene.get(target).path.clone();
        Self::new(
            target,
            AnimationKind::ApplyWave {
                full,
                amplitude,
                ripples,
            },
            0.0,
            duration,
        )
    }

    pub fn wiggle(scene: &SceneGraph, target: NodeId, duration: f64) -> Self {
        let from = scene.get(target).transform;
        let about = scene.local_pivot(target);
        Self::new(
            target,
            AnimationKind::Wiggle {
                from,
                about,
                angle: 0.14,
                wiggles: 2.0,
            },
            0.0,
            duration,
        )
    }

    pub fn end(&self) -> f64 {
        self.start + self.duration
    }

    /// Absolute state of this animation at time t.
    pub(crate) fn alpha_at(&self, t: f64) -> f64 {
        self.easing.eval((t - self.start) / self.duration)
    }

    /// Set the target's absolute state for progress `alpha`.
    fn apply(&self, scene: &mut SceneGraph, alpha: f64) {
        match &self.kind {
            AnimationKind::Create { full } => {
                scene.get_mut(self.target).path = geometry::trim(full, 0.0, alpha);
            }
            AnimationKind::Uncreate { full } => {
                scene.get_mut(self.target).path = geometry::trim(full, 0.0, 1.0 - alpha);
            }
            AnimationKind::FadeIn { to_opacity } => {
                scene.get_mut(self.target).style.opacity = to_opacity * alpha as f32;
            }
            AnimationKind::FadeOut { from_opacity } => {
                scene.get_mut(self.target).style.opacity =
                    from_opacity * (1.0 - alpha) as f32;
            }
            AnimationKind::Shift { from, delta } => {
                scene.get_mut(self.target).transform =
                    *from * Affine::translate(*delta * alpha);
            }
            AnimationKind::Scale {
                from,
                about,
                factor,
            } => {
                let f = 1.0 + (factor - 1.0) * alpha;
                let scale_about = Affine::translate(about.to_vec2())
                    * Affine::scale(f)
                    * Affine::translate(-about.to_vec2());
                scene.get_mut(self.target).transform = *from * scale_about;
            }
            AnimationKind::Rotate { from, about, angle } => {
                scene.get_mut(self.target).transform =
                    *from * Affine::rotate_about(angle * alpha, *about);
            }
            AnimationKind::Grow { from, about } => {
                let scale_about = Affine::translate(about.to_vec2())
                    * Affine::scale(alpha)
                    * Affine::translate(-about.to_vec2());
                scene.get_mut(self.target).transform = *from * scale_about;
            }
            AnimationKind::Morph { from, to } => {
                scene.get_mut(self.target).path =
                    geometry::lerp_paths(from, to, MORPH_SAMPLES, alpha);
            }
            AnimationKind::Recolor {
                from_fill,
                to_fill,
                from_stroke,
                to_stroke,
            } => {
                let m = scene.get_mut(self.target);
                if let (Some(a), Some(b)) = (from_fill, to_fill) {
                    m.style.fill = Some(lerp_color(*a, *b, alpha as f32));
                }
                if let (Some(a), Some(b)) = (from_stroke, to_stroke) {
                    m.style.stroke = Some(lerp_color(*a, *b, alpha as f32));
                }
            }
            AnimationKind::DrawBorderThenFill {
                full,
                fill,
                fill_opacity,
            } => {
                let border_end = 0.6;
                let m = scene.get_mut(self.target);
                if alpha < border_end {
                    let t = alpha / border_end;
                    m.path = geometry::trim(full, 0.0, t);
                    m.style.fill = None;
                } else {
                    m.path = full.clone();
                    m.style.fill = *fill;
                    let t = (alpha - border_end) / (1.0 - border_end);
                    m.style.fill_opacity = fill_opacity * t as f32;
                }
            }
            AnimationKind::Wiggle {
                from,
                about,
                angle,
                wiggles,
            } => {
                let w = (wiggles * std::f64::consts::PI * alpha).sin() * (1.0 - alpha);
                scene.get_mut(self.target).transform =
                    *from * Affine::rotate_about(angle * w, *about);
            }
            AnimationKind::ShowPassingFlash { full, time_width } => {
                let tw = time_width.max(1e-6);
                let upper = (alpha * (1.0 + tw)).min(1.0);
                let lower = (alpha * (1.0 + tw) - tw).clamp(0.0, 1.0);
                scene.get_mut(self.target).path = geometry::trim(full, lower, upper);
            }
            AnimationKind::MoveAlongPath { from, about, path } => {
                let target_pt = geometry::point_along(path, alpha);
                let current = *from * *about;
                scene.get_mut(self.target).transform =
                    Affine::translate(target_pt - current) * *from;
            }
            AnimationKind::SpinIn { from, about, angle } => {
                let rot = Affine::rotate_about(angle * (1.0 - alpha), *about);
                let scale = Affine::translate(about.to_vec2())
                    * Affine::scale(alpha)
                    * Affine::translate(-about.to_vec2());
                scene.get_mut(self.target).transform = *from * rot * scale;
            }
            AnimationKind::ChangingDecimal {
                from,
                to,
                places,
                atlas,
            } => {
                let v = from + (to - from) * alpha;
                scene.get_mut(self.target).path = atlas.compose(v, *places);
            }
            AnimationKind::Travel {
                from,
                delta,
                about,
                scale,
            } => {
                // World-space shift so a pre-scaled `from` (FadeTransform
                // target) still travels the full center-to-center delta.
                let f = 1.0 + (scale - 1.0) * alpha;
                let scale_about = Affine::translate(about.to_vec2())
                    * Affine::scale(f)
                    * Affine::translate(-about.to_vec2());
                scene.get_mut(self.target).transform =
                    Affine::translate(*delta * alpha) * *from * scale_about;
            }
            AnimationKind::ApplyWave {
                full,
                amplitude,
                ripples,
            } => {
                scene.get_mut(self.target).path =
                    wave_path(full, *amplitude, *ripples, alpha);
            }
            AnimationKind::Restore {
                from_path,
                to_path,
                from_transform,
                to_transform,
                from_style,
                to_style,
            } => {
                let path = geometry::lerp_paths(from_path, to_path, MORPH_SAMPLES, alpha);
                let transform = lerp_affine(*from_transform, *to_transform, alpha);
                let t = alpha as f32;
                let m = scene.get_mut(self.target);
                m.path = path;
                m.transform = transform;
                m.style.opacity =
                    from_style.opacity + (to_style.opacity - from_style.opacity) * t;
                m.style.fill = lerp_opt_color(from_style.fill, to_style.fill, alpha);
                m.style.stroke = lerp_opt_color(from_style.stroke, to_style.stroke, alpha);
            }
        }
    }

    /// End state — applied eagerly at `play()` time so subsequently built
    /// animations snapshot correct `from` states (Manim semantics).
    /// Uses the eased alpha at t=1 so ThereAndBack (Indicate) restores the
    /// pre-animation transform.
    pub fn apply_final(&self, scene: &mut SceneGraph) {
        self.apply(scene, self.easing.eval(1.0));
    }

    pub(crate) fn apply_at_alpha(&self, scene: &mut SceneGraph, alpha: f64) {
        self.apply(scene, alpha);
    }
}

fn lerp_affine(from: Affine, to: Affine, alpha: f64) -> Affine {
    let a = from.as_coeffs();
    let b = to.as_coeffs();
    Affine::new([
        a[0] + (b[0] - a[0]) * alpha,
        a[1] + (b[1] - a[1]) * alpha,
        a[2] + (b[2] - a[2]) * alpha,
        a[3] + (b[3] - a[3]) * alpha,
        a[4] + (b[4] - a[4]) * alpha,
        a[5] + (b[5] - a[5]) * alpha,
    ])
}

fn lerp_opt_color(from: Option<Color>, to: Option<Color>, alpha: f64) -> Option<Color> {
    match (from, to) {
        (Some(a), Some(b)) => Some(lerp_color(a, b, alpha as f32)),
        (_, t) if alpha >= 1.0 => t,
        (f, _) => f,
    }
}

fn wave_path(full: &BezPath, amplitude: f64, ripples: f64, alpha: f64) -> BezPath {
    let (pts, closed) = geometry::resample(full, 96);
    if pts.len() < 2 {
        return full.clone();
    }
    // CE ApplyWave homotopy: a window of width `time_width` travels in x.
    let time_width = 1.0;
    let upper = alpha * (1.0 + time_width);
    let lower = upper - time_width;
    let span = (upper - lower).max(1e-9);
    let x_min = pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let x_max = pts.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let x_span = (x_max - x_min).max(1e-9);
    let waved: Vec<Point> = pts
        .iter()
        .map(|p| {
            let relative_x = (p.x - x_min) / x_span;
            let phase = (relative_x - lower) / span;
            let o = amplitude * ce_wave(phase, ripples);
            Point::new(p.x, p.y + o)
        })
        .collect();
    geometry::points_to_path(&waved, closed)
}

/// ManimCE `rate_functions.smooth` (sigmoid), used as ApplyWave's flank.
fn ce_smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let inflection = 10.0;
    let sigmoid = |x: f64| 1.0 / (1.0 + (-x).exp());
    let error = sigmoid(-inflection / 2.0);
    ((sigmoid(inflection * (t - 0.5)) - error) / (1.0 - 2.0 * error)).clamp(0.0, 1.0)
}

/// ManimCE `ApplyWave.wave`: ripples of alternating sign, zero outside [0, 1].
fn ce_wave(phase: f64, ripples: f64) -> f64 {
    let t = 1.0 - phase;
    if !(0.0..=1.0).contains(&t) || t == 0.0 || t == 1.0 {
        return 0.0;
    }
    let ripples = ripples.max(1.0);
    let phases = ripples * 2.0;
    let phase_i = (t * phases).floor() as i32;
    if phase_i == 0 {
        return ce_smooth(t * phases);
    }
    if phase_i as f64 >= phases - 1.0 {
        let t = t - phase_i as f64 / phases;
        return (1.0 - ce_smooth(t * phases)) * (2.0 * (ripples as i32 % 2) as f64 - 1.0);
    }
    let p = (phase_i - 1) / 2;
    let t = t - (2 * p + 1) as f64 / phases;
    (1.0 - 2.0 * ce_smooth(t * ripples)) * (1.0 - 2.0 * (p % 2) as f64)
}
