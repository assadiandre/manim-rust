//! Compound constructors inspired by ManimCE geometry and graphing.
//!
//! These build scene-graph groups (arrow = shaft + tip, number line = line +
//! ticks) so Create/Write can animate the parts independently.

use kurbo::{Affine, Point};

use crate::constants::{DEFAULT_ARROW_TIP_LENGTH, DEFAULT_DOT_RADIUS};
use crate::geometry;
use crate::mobject::Mobject;
use crate::scene::{NodeId, SceneGraph};
use crate::style::{palette, Style};

/// Filled dot (Manim `Dot`).
pub fn add_dot(graph: &mut SceneGraph, center: Point, radius: f64, style: Style) -> NodeId {
    let r = if radius <= 0.0 { DEFAULT_DOT_RADIUS } else { radius };
    graph.add(Mobject::new(geometry::circle(center, r)).with_style(style))
}

/// Arrow as a group: stroked shaft + filled triangular tip.
pub fn add_arrow(
    graph: &mut SceneGraph,
    start: Point,
    end: Point,
    buff: f64,
    tip_length: f64,
    style: Style,
) -> NodeId {
    let (start, end) = geometry::shorten(start, end, buff);
    let length = (end - start).hypot();
    let tip_len = if tip_length <= 0.0 {
        geometry::default_tip_length(length)
    } else {
        tip_length.min(length * 0.45)
    };
    let tip_width = tip_len * 0.7;
    let shaft = geometry::arrow_shaft(start, end, tip_len);
    let tip = geometry::arrow_tip(start, end, tip_len, tip_width);
    let tip_color = style.stroke.or(style.fill).unwrap_or_else(palette::white);

    let group = graph.add(Mobject::group().named("arrow"));
    graph.add_child(
        group,
        Mobject::new(shaft).with_style(style.clone().no_fill()),
    );
    graph.add_child(
        group,
        Mobject::new(tip).with_style(
            Style::filled(tip_color)
                .no_stroke()
                .with_opacity(style.opacity),
        ),
    );
    group
}

#[derive(Clone, Debug)]
pub struct NumberLineOpts {
    pub x_min: f64,
    pub x_max: f64,
    pub x_step: f64,
    pub unit_size: f64,
    pub tick_size: f64,
    pub include_ticks: bool,
    pub include_tip: bool,
}

impl Default for NumberLineOpts {
    fn default() -> Self {
        Self {
            x_min: -4.0,
            x_max: 4.0,
            x_step: 1.0,
            unit_size: 1.0,
            tick_size: 0.1,
            include_ticks: true,
            include_tip: false,
        }
    }
}

/// Horizontal number line at y=0, value 0 at the origin.
pub fn add_number_line(graph: &mut SceneGraph, opts: &NumberLineOpts, style: Style) -> NodeId {
    let group = graph.add(Mobject::group().named("number_line"));
    add_number_line_into(graph, group, opts, style);
    group
}

fn add_number_line_into(
    graph: &mut SceneGraph,
    group: NodeId,
    opts: &NumberLineOpts,
    style: Style,
) {
    let start = Point::new(opts.x_min * opts.unit_size, 0.0);
    let end = Point::new(opts.x_max * opts.unit_size, 0.0);
    if opts.include_tip {
        let tip = DEFAULT_ARROW_TIP_LENGTH.min(0.25 * (end - start).hypot());
        let shaft = geometry::arrow_shaft(start, end, tip);
        let tip_path = geometry::arrow_tip(start, end, tip, tip * 0.7);
        let tip_color = style.stroke.or(style.fill).unwrap_or_else(palette::white);
        graph.add_child(
            group,
            Mobject::new(shaft).with_style(style.clone().no_fill()),
        );
        graph.add_child(
            group,
            Mobject::new(tip_path).with_style(Style::filled(tip_color).no_stroke()),
        );
    } else {
        graph.add_child(
            group,
            Mobject::new(geometry::line(start, end)).with_style(style.clone().no_fill()),
        );
    }
    if opts.include_ticks && opts.x_step > 0.0 {
        let n = ((opts.x_max - opts.x_min) / opts.x_step).round() as i32;
        for i in 0..=n {
            let x = opts.x_min + i as f64 * opts.x_step;
            // A tick on the positive end sits under the arrow tip.
            if opts.include_tip && (x - opts.x_max).abs() < 1e-9 {
                continue;
            }
            let px = x * opts.unit_size;
            let tick = geometry::line(
                Point::new(px, -opts.tick_size),
                Point::new(px, opts.tick_size),
            );
            graph.add_child(group, Mobject::new(tick).with_style(style.clone().no_fill()));
        }
    }
}

#[derive(Clone, Debug)]
pub struct AxesOpts {
    pub x_min: f64,
    pub x_max: f64,
    pub x_step: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub y_step: f64,
    pub unit_size: f64,
    pub tick_size: f64,
    pub include_tip: bool,
}

impl Default for AxesOpts {
    fn default() -> Self {
        Self {
            x_min: -4.0,
            x_max: 4.0,
            x_step: 1.0,
            y_min: -3.0,
            y_max: 3.0,
            y_step: 1.0,
            unit_size: 1.0,
            tick_size: 0.1,
            include_tip: true,
        }
    }
}

/// Origin-centered axes: x-axis plus a y-axis (x-axis rotated 90°).
pub fn add_axes(graph: &mut SceneGraph, opts: &AxesOpts, style: Style) -> NodeId {
    let x_opts = NumberLineOpts {
        x_min: opts.x_min,
        x_max: opts.x_max,
        x_step: opts.x_step,
        unit_size: opts.unit_size,
        tick_size: opts.tick_size,
        include_ticks: true,
        include_tip: opts.include_tip,
    };
    let y_opts = NumberLineOpts {
        x_min: opts.y_min,
        x_max: opts.y_max,
        x_step: opts.y_step,
        unit_size: opts.unit_size,
        tick_size: opts.tick_size,
        include_ticks: true,
        include_tip: opts.include_tip,
    };
    let x = add_number_line(graph, &x_opts, style.clone());
    let y = add_number_line(graph, &y_opts, style);
    graph.get_mut(y).transform = Affine::rotate(std::f64::consts::FRAC_PI_2);
    graph.group_nodes(&[x, y])
}

/// Arrow from the origin to `end` (Manim `Vector`).
pub fn add_vector(graph: &mut SceneGraph, end: Point, style: Style) -> NodeId {
    add_arrow(graph, Point::ORIGIN, end, 0.0, 0.0, style)
}

/// Arrow with tips at both ends.
pub fn add_double_arrow(
    graph: &mut SceneGraph,
    start: Point,
    end: Point,
    buff: f64,
    style: Style,
) -> NodeId {
    let (start, end) = geometry::shorten(start, end, buff);
    let length = (end - start).hypot();
    let tip = geometry::default_tip_length(length);
    let group = graph.add(Mobject::group().named("double_arrow"));
    let shaft = geometry::arrow_shaft(start, end, tip);
    // Reverse shaft so the start tip has a matching inset: reuse arrow_shaft
    // from end→start for the other cap, but only take the tip.
    let tip_end = geometry::arrow_tip(start, end, tip, tip * 0.7);
    let tip_start = geometry::arrow_tip(end, start, tip, tip * 0.7);
    let color = style.stroke.or(style.fill).unwrap_or_else(palette::white);
    graph.add_child(group, Mobject::new(shaft).with_style(style.clone().no_fill()));
    graph.add_child(
        group,
        Mobject::new(tip_end).with_style(Style::filled(color).no_stroke()),
    );
    graph.add_child(
        group,
        Mobject::new(tip_start).with_style(Style::filled(color).no_stroke()),
    );
    group
}

/// Rectangle (optionally rounded) around `target`'s bbox (Manim `SurroundingRectangle`).
pub fn add_surrounding_rect(
    graph: &mut SceneGraph,
    target: NodeId,
    buff: f64,
    corner_radius: f64,
    style: Style,
) -> NodeId {
    let bb = graph.bounding_box(target);
    let w = bb.width() + 2.0 * buff;
    let h = bb.height() + 2.0 * buff;
    let path = if corner_radius > 1e-6 {
        geometry::rounded_rect(bb.center(), w, h, corner_radius)
    } else {
        geometry::rect(bb.center(), w, h)
    };
    graph.add(Mobject::new(path).with_style(style).named("surrounding_rect"))
}

/// Horizontal line just below `target` (Manim `Underline`).
pub fn add_underline(graph: &mut SceneGraph, target: NodeId, buff: f64, style: Style) -> NodeId {
    let bb = graph.bounding_box(target);
    let y = bb.y0 - buff;
    graph.add(
        Mobject::new(geometry::line(Point::new(bb.x0, y), Point::new(bb.x1, y)))
            .with_style(style)
            .named("underline"),
    )
}

/// X across `target`'s bbox (Manim `Cross`).
pub fn add_cross(graph: &mut SceneGraph, target: NodeId, style: Style) -> NodeId {
    let bb = graph.bounding_box(target);
    let group = graph.add(Mobject::group().named("cross"));
    graph.add_child(
        group,
        Mobject::new(geometry::line(
            Point::new(bb.x0, bb.y0),
            Point::new(bb.x1, bb.y1),
        ))
        .with_style(style.clone()),
    );
    graph.add_child(
        group,
        Mobject::new(geometry::line(
            Point::new(bb.x0, bb.y1),
            Point::new(bb.x1, bb.y0),
        ))
        .with_style(style),
    );
    group
}

/// Brace along `target`'s `direction` edge (Manim `Brace`). `DOWN` puts it below.
pub fn add_brace(
    graph: &mut SceneGraph,
    target: NodeId,
    direction: kurbo::Vec2,
    buff: f64,
    style: Style,
) -> NodeId {
    let bb = graph.bounding_box(target);
    let along_x = direction.x.abs() >= direction.y.abs();
    let width = if along_x { bb.height() } else { bb.width() };
    let stroke = style.stroke.unwrap_or_else(palette::white);
    let sw = style.stroke_width;
    let id = graph.add(
        Mobject::new(geometry::brace(width))
            .with_style(style.no_fill().with_stroke(stroke, sw))
            .named("brace"),
    );
    // Brace is built tip-up. Rotate so the tip faces the target.
    let angle = (-direction.x).atan2(-direction.y); // DOWN → 0
    graph.get_mut(id).transform = Affine::rotate(angle);
    graph.next_to(id, target, direction, buff);
    id
}

#[derive(Clone, Debug)]
pub struct NumberPlaneOpts {
    pub x_min: f64,
    pub x_max: f64,
    pub x_step: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub y_step: f64,
    pub unit_size: f64,
    pub faded_line_ratio: u32,
}

impl Default for NumberPlaneOpts {
    fn default() -> Self {
        Self {
            x_min: -crate::constants::FRAME_X_RADIUS,
            x_max: crate::constants::FRAME_X_RADIUS,
            x_step: 1.0,
            y_min: -crate::constants::FRAME_Y_RADIUS,
            y_max: crate::constants::FRAME_Y_RADIUS,
            y_step: 1.0,
            unit_size: 1.0,
            faded_line_ratio: 1,
        }
    }
}

/// Cartesian plane with background grid + axes (Manim `NumberPlane`).
pub fn add_number_plane(
    graph: &mut SceneGraph,
    opts: &NumberPlaneOpts,
    grid_style: Style,
    axis_style: Style,
) -> NodeId {
    let group = graph.add(Mobject::group().named("number_plane"));
    let u = opts.unit_size;
    let faded = opts.faded_line_ratio.max(1);
    let mut faded_style = grid_style.clone();
    faded_style.stroke_opacity *= 0.35;
    faded_style.stroke_width *= 0.6;

    let add_v = |graph: &mut SceneGraph, x: f64, style: &Style| {
        graph.add_child(
            group,
            Mobject::new(geometry::line(
                Point::new(x * u, opts.y_min * u),
                Point::new(x * u, opts.y_max * u),
            ))
            .with_style(style.clone().no_fill()),
        );
    };
    let add_h = |graph: &mut SceneGraph, y: f64, style: &Style| {
        graph.add_child(
            group,
            Mobject::new(geometry::line(
                Point::new(opts.x_min * u, y * u),
                Point::new(opts.x_max * u, y * u),
            ))
            .with_style(style.clone().no_fill()),
        );
    };

    let on_step = |v: f64, step: f64| (v / step - (v / step).round()).abs() < 1e-6;
    if faded > 1 && opts.x_step > 0.0 && opts.y_step > 0.0 {
        let dx = opts.x_step / faded as f64;
        let dy = opts.y_step / faded as f64;
        let nx = ((opts.x_max - opts.x_min) / dx).round() as i32;
        let ny = ((opts.y_max - opts.y_min) / dy).round() as i32;
        for i in 0..=nx {
            let x = opts.x_min + i as f64 * dx;
            if !on_step(x, opts.x_step) {
                add_v(graph, x, &faded_style);
            }
        }
        for i in 0..=ny {
            let y = opts.y_min + i as f64 * dy;
            if !on_step(y, opts.y_step) {
                add_h(graph, y, &faded_style);
            }
        }
    }

    if opts.x_step > 0.0 {
        let n = ((opts.x_max - opts.x_min) / opts.x_step).round() as i32;
        for i in 0..=n {
            let x = opts.x_min + i as f64 * opts.x_step;
            if x.abs() > 1e-9 {
                add_v(graph, x, &grid_style);
            }
        }
    }
    if opts.y_step > 0.0 {
        let n = ((opts.y_max - opts.y_min) / opts.y_step).round() as i32;
        for i in 0..=n {
            let y = opts.y_min + i as f64 * opts.y_step;
            if y.abs() > 1e-9 {
                add_h(graph, y, &grid_style);
            }
        }
    }

    // Axes last so they sit on top of the grid (same z, later in tree).
    graph.add_child(
        group,
        Mobject::new(geometry::line(
            Point::new(opts.x_min * u, 0.0),
            Point::new(opts.x_max * u, 0.0),
        ))
        .with_style(axis_style.clone().no_fill()),
    );
    graph.add_child(
        group,
        Mobject::new(geometry::line(
            Point::new(0.0, opts.y_min * u),
            Point::new(0.0, opts.y_max * u),
        ))
        .with_style(axis_style.no_fill()),
    );
    group
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DOWN;
    use crate::geometry;
    use crate::scene::SceneGraph;

    #[test]
    fn surrounding_rect_is_larger_than_target() {
        let mut g = SceneGraph::new();
        let sq = g.add(Mobject::new(geometry::square(Point::ORIGIN, 2.0)));
        let r = add_surrounding_rect(
            &mut g,
            sq,
            0.25,
            0.0,
            Style::default().no_fill().with_stroke(palette::yellow(), 4.0),
        );
        let a = g.bounding_box(sq);
        let b = g.bounding_box(r);
        assert!((b.width() - a.width() - 0.5).abs() < 1e-6);
        assert!((b.height() - a.height() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn brace_sits_below_target() {
        let mut g = SceneGraph::new();
        let sq = g.add(Mobject::new(geometry::square(Point::ORIGIN, 2.0)));
        let b = add_brace(&mut g, sq, DOWN, 0.2, Style::default());
        let sq_bb = g.bounding_box(sq);
        let br_bb = g.bounding_box(b);
        assert!(br_bb.y1 <= sq_bb.y0 + 1e-6, "brace should be below square");
    }

    #[test]
    fn number_plane_has_grid_and_axes() {
        let mut g = SceneGraph::new();
        let p = add_number_plane(
            &mut g,
            &NumberPlaneOpts {
                x_min: -2.0,
                x_max: 2.0,
                y_min: -1.0,
                y_max: 1.0,
                faded_line_ratio: 2,
                ..NumberPlaneOpts::default()
            },
            Style::default().with_stroke(palette::blue_d(), 2.0),
            Style::default().with_stroke(palette::white(), 3.0),
        );
        assert!(g.children_of(p).len() > 6);
    }
}

