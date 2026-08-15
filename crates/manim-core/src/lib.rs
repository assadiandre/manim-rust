//! manim-core: geometry kernel, styles, and the dirty-tracked scene graph.
//!
//! Invariants enforced here (see DESIGN.md):
//! - paths are contiguous `kurbo::BezPath` buffers, transforms are affines
//!   applied at encode time (never baked into points unless requested),
//! - all mutation goes through `SceneGraph::get_mut` / `mark_dirty` so the
//!   renderer can skip unchanged frames.

pub mod boolean;
pub mod camera;
pub mod constants;
pub mod construct;
pub mod decimal;
pub mod geometry;
pub mod layout;
pub mod mobject;
pub mod scene;
pub mod style;
mod svg;

pub use boolean::{add_boolean, boolean_path, BooleanOp};
pub use camera::{Camera, OrthoCamera2D};
pub use constants::{
    DEFAULT_ARROW_TIP_LENGTH, DEFAULT_DOT_RADIUS, DEFAULT_MOBJECT_TO_EDGE_BUFFER,
    DEFAULT_MOBJECT_TO_MOBJECT_BUFFER, DEGREES, DL, DOWN, DR, FRAME_HEIGHT, FRAME_WIDTH,
    FRAME_X_RADIUS, FRAME_Y_RADIUS, LEFT, MED_SMALL_BUFF, ORIGIN, PI, RIGHT, TAU, UL, UP, UR,
};
pub use construct::{
    add_angle, add_area_between, add_area_under, add_arrow, add_arrow_field, add_axes,
    add_background_rect, add_bar_chart, add_brace, add_complex_plane, add_cross, add_curved_arrow,
    add_curved_double_arrow, add_dashed_copy, add_dot, add_double_arrow, add_graph,
    add_implicit_curve, add_number_line, add_number_plane, add_polar_plane, add_riemann_rects,
    add_right_angle, add_surrounding_rect, add_tangent_line, add_underline, add_vector,
    add_vertical_line_to_graph, axes_c2p, layout_graph, number_line_n2p, number_line_tick_values,
    plane_n2p, plot_point_at_x, AxesOpts, NumberLineOpts, NumberPlaneOpts, PolarPlaneOpts,
    RiemannSample,
};
pub use decimal::DigitAtlas;
pub use mobject::Mobject;
pub use scene::{NodeId, SceneGraph};
pub use style::{lerp_color, palette, Style};
pub use svg::{add_svg, svg_mobjects, SvgError};

// Re-export the data model so downstream crates stay version-consistent.
pub use kurbo;
pub use peniko;
