//! manim-anim: declarative animations and the stateless timeline evaluator.
//!
//! Invariant #1 (see DESIGN.md): animations are *data*. The render loop never
//! calls user closures — it evaluates this crate's `AnimationKind` directly,
//! which is what keeps the per-frame path free of the Python FFI.

pub mod animation;
pub mod easing;
pub mod timeline;

pub use animation::{Animation, AnimationKind, Prop};
pub use easing::Easing;
pub use timeline::{path_targets, CameraAnim, Scene, Timeline};

pub use kurbo;
pub use manim_core;
