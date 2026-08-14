//! ManimCE-compatible directions, frame size, and default buffers.
//!
//! Values match `manim/constants.py` so authored scenes land in the same
//! logical frame (height 8, 16:9).

use kurbo::Vec2;

/// Logical frame height (Manim default). Width follows 16:9.
pub const FRAME_HEIGHT: f64 = 8.0;
pub const FRAME_WIDTH: f64 = FRAME_HEIGHT * 16.0 / 9.0;
pub const FRAME_X_RADIUS: f64 = FRAME_WIDTH / 2.0;
pub const FRAME_Y_RADIUS: f64 = FRAME_HEIGHT / 2.0;

pub const ORIGIN: Vec2 = Vec2::new(0.0, 0.0);
pub const UP: Vec2 = Vec2::new(0.0, 1.0);
pub const DOWN: Vec2 = Vec2::new(0.0, -1.0);
pub const RIGHT: Vec2 = Vec2::new(1.0, 0.0);
pub const LEFT: Vec2 = Vec2::new(-1.0, 0.0);
pub const UL: Vec2 = Vec2::new(-1.0, 1.0);
pub const UR: Vec2 = Vec2::new(1.0, 1.0);
pub const DL: Vec2 = Vec2::new(-1.0, -1.0);
pub const DR: Vec2 = Vec2::new(1.0, -1.0);

pub const DEFAULT_DOT_RADIUS: f64 = 0.08;
pub const DEFAULT_SMALL_DOT_RADIUS: f64 = 0.04;
pub const DEFAULT_DASH_LENGTH: f64 = 0.05;
pub const DEFAULT_ARROW_TIP_LENGTH: f64 = 0.35;
pub const DEFAULT_STROKE_WIDTH: f64 = 4.0;

pub const SMALL_BUFF: f64 = 0.1;
pub const MED_SMALL_BUFF: f64 = 0.25;
pub const MED_LARGE_BUFF: f64 = 0.5;
pub const DEFAULT_MOBJECT_TO_EDGE_BUFFER: f64 = MED_LARGE_BUFF;
pub const DEFAULT_MOBJECT_TO_MOBJECT_BUFFER: f64 = MED_SMALL_BUFF;

/// Multiply by this to convert degrees to radians (`90.0 * DEGREES`).
pub const DEGREES: f64 = std::f64::consts::PI / 180.0;
pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = std::f64::consts::TAU;
