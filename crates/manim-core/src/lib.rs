//! manim-core: geometry kernel, styles, and the dirty-tracked scene graph.
//!
//! Invariants enforced here (see DESIGN.md):
//! - paths are contiguous `kurbo::BezPath` buffers, transforms are affines
//!   applied at encode time (never baked into points unless requested),
//! - all mutation goes through `SceneGraph::get_mut` / `mark_dirty` so the
//!   renderer can skip unchanged frames.

pub mod camera;
pub mod geometry;
pub mod mobject;
pub mod scene;
pub mod style;

pub use camera::{Camera, OrthoCamera2D};
pub use mobject::Mobject;
pub use scene::{NodeId, SceneGraph};
pub use style::{palette, Style};

// Re-export the data model so downstream crates stay version-consistent.
pub use kurbo;
pub use peniko;
