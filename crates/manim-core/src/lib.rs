//! manim-core: geometry kernel, styles, and the dirty-tracked scene graph.
//!
//! Invariants enforced here (see DESIGN.md):
//! - paths are contiguous `kurbo::BezPath` buffers, transforms are affines
//!   applied at encode time (never baked into points unless requested),
//! - all mutation goes through `SceneGraph::get_mut` / `mark_dirty` so the
//!   renderer can skip unchanged frames.

pub mod camera;
pub mod constants;
pub mod construct;
pub mod geometry;
pub mod layout;
pub mod mobject;
pub mod scene;
pub mod style;

pub use camera::{Camera, OrthoCamera2D};
pub use constants::{
    DEGREES, DEFAULT_ARROW_TIP_LENGTH, DEFAULT_DOT_RADIUS, DEFAULT_MOBJECT_TO_EDGE_BUFFER,
    DEFAULT_MOBJECT_TO_MOBJECT_BUFFER, DOWN, DL, DR, FRAME_HEIGHT, FRAME_WIDTH, FRAME_X_RADIUS,
    FRAME_Y_RADIUS, LEFT, MED_SMALL_BUFF, ORIGIN, PI, RIGHT, TAU, UL, UP, UR,
};
pub use construct::{
    add_angle, add_arrow, add_axes, add_background_rect, add_brace, add_cross, add_curved_arrow,
    add_dot, add_double_arrow, add_number_line, add_number_plane, add_polar_plane,
    add_right_angle, add_surrounding_rect, add_underline, add_vector, AxesOpts, NumberLineOpts,
    NumberPlaneOpts, PolarPlaneOpts,
};
pub use mobject::Mobject;
pub use scene::{NodeId, SceneGraph};
pub use style::{lerp_color, palette, Style};

// Re-export the data model so downstream crates stay version-consistent.
pub use kurbo;
pub use peniko;
