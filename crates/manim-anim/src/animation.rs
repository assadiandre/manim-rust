//! Declarative animations: pure data, no closures (invariant #1).
//!
//! Every animation snapshots the state it interpolates *from* at construction
//! time, so evaluation at any t is stateless and the render loop can apply
//! the whole timeline to a fresh scene clone each frame.

use kurbo::{Affine, BezPath, Point, Vec2};
use manim_core::geometry;
use manim_core::peniko::Color;
use manim_core::style::lerp_color;
use manim_core::{NodeId, SceneGraph};

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
            | AnimationKind::DrawBorderThenFill { .. } => Prop::Path,
            AnimationKind::FadeIn { .. } | AnimationKind::FadeOut { .. } => Prop::Opacity,
            AnimationKind::Shift { .. }
            | AnimationKind::Scale { .. }
            | AnimationKind::Rotate { .. }
            | AnimationKind::Grow { .. }
            | AnimationKind::Wiggle { .. } => Prop::Transform,
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
