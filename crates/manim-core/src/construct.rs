//! Compound constructors inspired by ManimCE geometry and graphing.
//!
//! These build scene-graph groups (arrow = shaft + tip, number line = line +
//! ticks) so Create/Write can animate the parts independently.

use kurbo::{Affine, Point, Vec2};

use crate::constants::{DEFAULT_ARROW_TIP_LENGTH, DEFAULT_DOT_RADIUS, PI, TAU};
use crate::geometry;
use crate::mobject::Mobject;
use crate::scene::{NodeId, SceneGraph};
use crate::style::{lerp_color, palette, Style};

/// Filled dot (Manim `Dot`).
pub fn add_dot(graph: &mut SceneGraph, center: Point, radius: f64, style: Style) -> NodeId {
    let r = if radius <= 0.0 {
        DEFAULT_DOT_RADIUS
    } else {
        radius
    };
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
    if opts.include_ticks {
        for x in number_line_tick_values(opts) {
            let px = x * opts.unit_size;
            let tick = geometry::line(
                Point::new(px, -opts.tick_size),
                Point::new(px, opts.tick_size),
            );
            graph.add_child(
                group,
                Mobject::new(tick).with_style(style.clone().no_fill()),
            );
        }
    }
}

/// Number-line value → point on the line (y = 0).
pub fn number_line_n2p(opts: &NumberLineOpts, value: f64) -> Point {
    Point::new(value * opts.unit_size, 0.0)
}

/// Tick locations used by `add_number_line` (skips the tip-end when `include_tip`).
pub fn number_line_tick_values(opts: &NumberLineOpts) -> Vec<f64> {
    if opts.x_step <= 0.0 {
        return Vec::new();
    }
    let n = ((opts.x_max - opts.x_min) / opts.x_step).round() as i32;
    let mut vals = Vec::new();
    for i in 0..=n {
        let x = opts.x_min + i as f64 * opts.x_step;
        // A tick on the positive end sits under the arrow tip.
        if opts.include_tip && (x - opts.x_max).abs() < 1e-9 {
            continue;
        }
        vals.push(x);
    }
    vals
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

/// Axes coordinates → scene point.
pub fn axes_c2p(opts: &AxesOpts, x: f64, y: f64) -> Point {
    Point::new(x * opts.unit_size, y * opts.unit_size)
}

/// World-space point on a baked plot at path-local `x` (parent transforms apply).
pub fn plot_point_at_x(graph: &SceneGraph, plot_id: NodeId, x: f64) -> Point {
    let local = geometry::point_at_x(&graph.get(plot_id).path, x);
    graph.world_transform(plot_id) * local
}

/// Arrow from the origin to `end` (Manim `Vector`).
pub fn add_vector(graph: &mut SceneGraph, end: Point, style: Style) -> NodeId {
    add_arrow(graph, Point::ORIGIN, end, 0.0, 0.0, style)
}

/// Grid of arrows from a vector function (Manim `ArrowVectorField`, static).
///
/// Each sample `(x, y)` gets an arrow of `(vx, vy)`, scaled so its length is
/// at most `max_len` (treated as 0.45 when `max_len <= 0`). Near-zero vectors
/// (`len < 1e-6`) are skipped.
pub fn add_arrow_field(
    graph: &mut SceneGraph,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_step: f64,
    y_step: f64,
    vx: impl Fn(f64, f64) -> f64,
    vy: impl Fn(f64, f64) -> f64,
    max_len: f64,
    style: Style,
) -> NodeId {
    let group = graph.add(Mobject::group().named("arrow_field"));
    let max_len = if max_len > 0.0 { max_len } else { 0.45 };
    if x_step <= 0.0 || y_step <= 0.0 {
        return group;
    }
    let nx = ((x_max - x_min) / x_step).round() as i32;
    let ny = ((y_max - y_min) / y_step).round() as i32;
    for j in 0..=ny {
        let y = y_min + j as f64 * y_step;
        for i in 0..=nx {
            let x = x_min + i as f64 * x_step;
            let dx = vx(x, y);
            let dy = vy(x, y);
            let len = dx.hypot(dy);
            if len < 1e-6 {
                continue;
            }
            let scale = (max_len / len).min(1.0);
            let start = Point::new(x, y);
            let end = Point::new(x + dx * scale, y + dy * scale);
            let arrow = add_arrow(graph, start, end, 0.0, 0.0, style.clone());
            graph.reparent(arrow, Some(group));
        }
    }
    group
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
    graph.add_child(
        group,
        Mobject::new(shaft).with_style(style.clone().no_fill()),
    );
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
    graph.add(
        Mobject::new(path)
            .with_style(style)
            .named("surrounding_rect"),
    )
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

/// Complex value (re, im) → scene point on a number plane.
pub fn plane_n2p(opts: &NumberPlaneOpts, re: f64, im: f64) -> Point {
    Point::new(re * opts.unit_size, im * opts.unit_size)
}

/// Number plane labeled as a complex plane (Manim `ComplexPlane`).
pub fn add_complex_plane(
    graph: &mut SceneGraph,
    opts: &NumberPlaneOpts,
    grid_style: Style,
    axis_style: Style,
) -> NodeId {
    let id = add_number_plane(graph, opts, grid_style, axis_style);
    graph.get_mut(id).name = Some("complex_plane".into());
    id
}

/// Marching-squares isoline `f(x,y) = 0` (Manim `ImplicitFunction`).
pub fn add_implicit_curve(
    graph: &mut SceneGraph,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    nx: usize,
    ny: usize,
    f: impl Fn(f64, f64) -> f64,
    style: Style,
) -> NodeId {
    graph.add(
        Mobject::new(geometry::implicit_curve(
            x_min, x_max, y_min, y_max, nx, ny, f,
        ))
        .with_style(style)
        .named("implicit"),
    )
}

/// Filled region under `f` on `[x_min, x_max]`.
pub fn add_area_under(
    graph: &mut SceneGraph,
    x_min: f64,
    x_max: f64,
    samples: usize,
    unit_x: f64,
    unit_y: f64,
    f: impl Fn(f64) -> f64,
    style: Style,
) -> NodeId {
    graph.add(
        Mobject::new(geometry::area_under(
            x_min, x_max, samples, unit_x, unit_y, f,
        ))
        .with_style(style)
        .named("area"),
    )
}

/// Filled region between `f` and `g` on `[x_min, x_max]`.
pub fn add_area_between(
    graph: &mut SceneGraph,
    x_min: f64,
    x_max: f64,
    samples: usize,
    unit_x: f64,
    unit_y: f64,
    f: impl Fn(f64) -> f64,
    g: impl Fn(f64) -> f64,
    style: Style,
) -> NodeId {
    graph.add(
        Mobject::new(geometry::area_between(
            x_min, x_max, samples, unit_x, unit_y, f, g,
        ))
        .with_style(style)
        .named("area"),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiemannSample {
    Left,
    Right,
    Center,
}

/// Equal-width Riemann rectangles under `f` on `[x_min, x_max]`.
pub fn add_riemann_rects(
    graph: &mut SceneGraph,
    x_min: f64,
    x_max: f64,
    n: usize,
    unit_x: f64,
    unit_y: f64,
    f: impl Fn(f64) -> f64,
    sample: RiemannSample,
    color_a: crate::peniko::Color,
    color_b: crate::peniko::Color,
    style_opacity: f32,
) -> NodeId {
    let n = n.max(1);
    let dx = (x_max - x_min) / n as f64;
    let group = graph.add(Mobject::group().named("riemann"));
    for i in 0..n {
        let x0 = x_min + i as f64 * dx;
        let x1 = x0 + dx;
        let xs = match sample {
            RiemannSample::Left => x0,
            RiemannSample::Right => x1,
            RiemannSample::Center => 0.5 * (x0 + x1),
        };
        let y = f(xs);
        let cx = 0.5 * (x0 + x1) * unit_x;
        let cy = 0.5 * y * unit_y;
        let path = geometry::rect(Point::new(cx, cy), (x1 - x0) * unit_x, (y * unit_y).abs());
        let t = if n <= 1 {
            0.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let style = Style::filled(lerp_color(color_a, color_b, t))
            .with_stroke(palette::black(), 1.0)
            .with_opacity(style_opacity);
        graph.add_child(group, Mobject::new(path).with_style(style));
    }
    group
}

fn dash_world_path(
    graph: &SceneGraph,
    id: NodeId,
    num_dashes: usize,
    dashed_ratio: f64,
) -> (kurbo::BezPath, Style) {
    let world = graph.world_transform(id);
    let (path, style) = {
        let m = graph.get(id);
        (m.path.clone(), m.style.clone())
    };
    let dashed = geometry::dashed_path(&(world * path), num_dashes, dashed_ratio);
    (dashed, style)
}

/// Dashed copy of `target`'s world-space path (Manim `DashedVMobject`).
///
/// A single path (or none) stays one node named `"dashed"`. Multiple path
/// leaves are each dashed and grouped under `"dashed"`.
pub fn add_dashed_copy(
    graph: &mut SceneGraph,
    target: NodeId,
    num_dashes: usize,
    dashed_ratio: f64,
) -> NodeId {
    let leaves = graph.path_leaves(target);
    if leaves.len() <= 1 {
        let id = leaves.first().copied().unwrap_or(target);
        let (dashed, style) = dash_world_path(graph, id, num_dashes, dashed_ratio);
        return graph.add(Mobject::new(dashed).with_style(style).named("dashed"));
    }
    let group = graph.add(Mobject::group().named("dashed"));
    for leaf in leaves {
        let (dashed, style) = dash_world_path(graph, leaf, num_dashes, dashed_ratio);
        graph.add_child(group, Mobject::new(dashed).with_style(style));
    }
    group
}

/// Arc plus a tip (Manim `CurvedArrow`).
pub fn add_curved_arrow(
    graph: &mut SceneGraph,
    start: Point,
    end: Point,
    sweep: f64,
    style: Style,
) -> NodeId {
    let full = geometry::arc_between_points(start, end, sweep);
    let len = geometry::path_length(&full);
    let tip_len = geometry::default_tip_length(len);
    let t_end = (1.0 - (tip_len / len.max(1e-9))).clamp(0.55, 0.95);
    let shaft = geometry::trim(&full, 0.0, t_end);
    let tip_base = geometry::point_along(&full, t_end);
    let tip = geometry::arrow_tip(tip_base, end, tip_len, tip_len * 0.7);
    let color = style.stroke.or(style.fill).unwrap_or_else(palette::white);
    let group = graph.add(Mobject::group().named("curved_arrow"));
    graph.add_child(
        group,
        Mobject::new(shaft).with_style(style.clone().no_fill()),
    );
    graph.add_child(
        group,
        Mobject::new(tip).with_style(Style::filled(color).no_stroke()),
    );
    group
}

/// Arc with tips at both ends (Manim `CurvedDoubleArrow`).
pub fn add_curved_double_arrow(
    graph: &mut SceneGraph,
    start: Point,
    end: Point,
    sweep: f64,
    style: Style,
) -> NodeId {
    let full = geometry::arc_between_points(start, end, sweep);
    let len = geometry::path_length(&full);
    let tip_len = geometry::default_tip_length(len);
    let t0 = (tip_len / len.max(1e-9)).clamp(0.05, 0.45);
    let t1 = (1.0 - t0).clamp(0.55, 0.95);
    let shaft = geometry::trim(&full, t0, t1);
    let tip_end = geometry::arrow_tip(
        geometry::point_along(&full, t1),
        end,
        tip_len,
        tip_len * 0.7,
    );
    let tip_start = geometry::arrow_tip(
        geometry::point_along(&full, t0),
        start,
        tip_len,
        tip_len * 0.7,
    );
    let color = style.stroke.or(style.fill).unwrap_or_else(palette::white);
    let group = graph.add(Mobject::group().named("curved_double_arrow"));
    graph.add_child(
        group,
        Mobject::new(shaft).with_style(style.clone().no_fill()),
    );
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

/// Angle mark between `p1`–`vertex`–`p2` (Manim `Angle`).
pub fn add_angle(
    graph: &mut SceneGraph,
    vertex: Point,
    p1: Point,
    p2: Point,
    radius: f64,
    style: Style,
) -> NodeId {
    graph.add(
        Mobject::new(geometry::angle_arc(vertex, p1, p2, radius))
            .with_style(style)
            .named("angle"),
    )
}

pub fn add_right_angle(
    graph: &mut SceneGraph,
    vertex: Point,
    p1: Point,
    p2: Point,
    size: f64,
    style: Style,
) -> NodeId {
    graph.add(
        Mobject::new(geometry::right_angle(vertex, p1, p2, size))
            .with_style(style)
            .named("right_angle"),
    )
}

#[derive(Clone, Debug)]
pub struct PolarPlaneOpts {
    pub radius: f64,
    pub radius_step: f64,
    pub azimuth_divisions: u32,
    pub unit_size: f64,
    pub faded_line_ratio: u32,
}

impl Default for PolarPlaneOpts {
    fn default() -> Self {
        Self {
            radius: crate::constants::FRAME_Y_RADIUS,
            radius_step: 1.0,
            azimuth_divisions: 12,
            unit_size: 1.0,
            faded_line_ratio: 1,
        }
    }
}

/// Concentric circles + radials (Manim `PolarPlane`, unlabeled).
pub fn add_polar_plane(
    graph: &mut SceneGraph,
    opts: &PolarPlaneOpts,
    grid_style: Style,
    axis_style: Style,
) -> NodeId {
    let group = graph.add(Mobject::group().named("polar_plane"));
    let u = opts.unit_size;
    let r_max = opts.radius.max(0.1);
    let faded = opts.faded_line_ratio.max(1);
    let mut faded_style = grid_style.clone();
    faded_style.stroke_opacity *= 0.35;
    faded_style.stroke_width *= 0.6;

    if faded > 1 && opts.radius_step > 0.0 {
        let dr = opts.radius_step / faded as f64;
        let n = (r_max / dr).round() as i32;
        for i in 1..=n {
            let r = i as f64 * dr;
            if (r / opts.radius_step - (r / opts.radius_step).round()).abs() > 1e-6 {
                graph.add_child(
                    group,
                    Mobject::new(geometry::circle(Point::ORIGIN, r * u))
                        .with_style(faded_style.clone().no_fill()),
                );
            }
        }
    }
    if opts.radius_step > 0.0 {
        let n = (r_max / opts.radius_step).round() as i32;
        for i in 1..=n {
            let r = i as f64 * opts.radius_step;
            graph.add_child(
                group,
                Mobject::new(geometry::circle(Point::ORIGIN, r * u))
                    .with_style(grid_style.clone().no_fill()),
            );
        }
    }
    let n_az = opts.azimuth_divisions.max(2);
    for i in 0..n_az {
        let a = i as f64 / n_az as f64 * std::f64::consts::TAU;
        let end = Point::new(a.cos() * r_max * u, a.sin() * r_max * u);
        graph.add_child(
            group,
            Mobject::new(geometry::line(Point::ORIGIN, end))
                .with_style(grid_style.clone().no_fill()),
        );
    }
    graph.add_child(
        group,
        Mobject::new(geometry::line(
            Point::new(-r_max * u, 0.0),
            Point::new(r_max * u, 0.0),
        ))
        .with_style(axis_style.clone().no_fill()),
    );
    graph.add_child(
        group,
        Mobject::new(geometry::line(
            Point::new(0.0, -r_max * u),
            Point::new(0.0, r_max * u),
        ))
        .with_style(axis_style.no_fill()),
    );
    group
}

/// Filled rect behind `target` (Manim `BackgroundRectangle`).
pub fn add_background_rect(
    graph: &mut SceneGraph,
    target: NodeId,
    buff: f64,
    fill: crate::peniko::Color,
    opacity: f32,
) -> NodeId {
    let id = add_surrounding_rect(
        graph,
        target,
        buff,
        0.0,
        Style::filled(fill).no_stroke().with_opacity(opacity),
    );
    graph.set_z_index(id, -1);
    id
}

/// Baked bar chart (Manim `BarChart` geometry).
///
/// The returned group has two children: a `bars` group (one rect per value,
/// in order) and an `axes` group. Labels are added separately so Typst stays
/// out of the geometry crate.
pub fn add_bar_chart(
    graph: &mut SceneGraph,
    values: &[f64],
    y_min: f64,
    y_max: f64,
    x_length: f64,
    y_length: f64,
    bar_width: f64,
    colors: &[crate::peniko::Color],
    fill_opacity: f32,
    stroke_width: f64,
) -> NodeId {
    let group = graph.add(Mobject::group().named("barchart"));
    if values.is_empty() {
        return group;
    }
    let span = (y_max - y_min).abs().max(1e-9);
    let y0_base = y_min.min(y_max);
    let map_y = |v: f64| -0.5 * y_length + (v - y0_base) / span * y_length;
    let n = values.len() as f64;
    let slot = x_length / n;
    let width = slot * bar_width.clamp(0.05, 1.0);
    let fallback = [
        palette::blue(),
        palette::teal(),
        palette::purple(),
        palette::red(),
        palette::gold(),
    ];
    let palette = if colors.is_empty() {
        &fallback[..]
    } else {
        colors
    };
    let zero_y = map_y(0.0f64.clamp(y_min.min(y_max), y_min.max(y_max)));

    let bars = graph.add(Mobject::group().named("bars"));
    for (i, &v) in values.iter().enumerate() {
        let cx = -0.5 * x_length + (i as f64 + 0.5) * slot;
        let y1 = map_y(v);
        let h = (y1 - zero_y).abs().max(1e-4);
        let cy = 0.5 * (zero_y + y1);
        let mut style =
            Style::filled(palette[i % palette.len()]).with_stroke(palette::white(), stroke_width);
        style.fill_opacity = fill_opacity;
        graph.add_child(
            bars,
            Mobject::new(geometry::rect(Point::new(cx, cy), width, h)).with_style(style),
        );
    }
    graph.reparent(bars, Some(group));

    let axis_style = Style::default()
        .with_stroke(palette::white(), 3.0)
        .no_fill();
    let axes = graph.add(Mobject::group().named("axes"));
    graph.add_child(
        axes,
        Mobject::new(geometry::line(
            Point::new(-0.5 * x_length, zero_y),
            Point::new(0.5 * x_length, zero_y),
        ))
        .with_style(axis_style.clone()),
    );
    graph.add_child(
        axes,
        Mobject::new(geometry::line(
            Point::new(-0.5 * x_length, map_y(y_min.min(y_max))),
            Point::new(-0.5 * x_length, map_y(y_min.max(y_max))),
        ))
        .with_style(axis_style),
    );
    graph.reparent(axes, Some(group));
    group
}

/// Compute vertex positions. `layout` is "circular" | "spring" | "tree" | anything else → circular.
/// `n` is vertex count. `edges` are undirected index pairs (0..n).
/// `scale` is the radius / half-extent (default callers pass 2.5).
pub fn layout_graph(
    n: usize,
    edges: &[(usize, usize)],
    layout: &str,
    scale: f64,
) -> Vec<kurbo::Point> {
    match layout {
        "spring" => layout_spring(n, edges, scale),
        "tree" => layout_tree(n, edges, scale),
        _ => layout_circular(n, scale),
    }
}

fn layout_circular(n: usize, scale: f64) -> Vec<Point> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![Point::ORIGIN];
    }
    (0..n)
        .map(|i| {
            let theta = TAU * i as f64 / n as f64 + PI / 2.0;
            Point::new(scale * theta.cos(), scale * theta.sin())
        })
        .collect()
}

fn layout_spring(n: usize, edges: &[(usize, usize)], scale: f64) -> Vec<Point> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![Point::ORIGIN];
    }
    let mut pos = layout_circular(n, 1.0);
    let k = 1.0;
    let iters = 40;
    for iter in 0..iters {
        let mut disp = vec![Vec2::ZERO; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let d = pos[i] - pos[j];
                let dist = d.hypot().max(1e-6);
                let force = (k * k) / dist;
                let dir = d / dist;
                disp[i] += dir * force;
                disp[j] -= dir * force;
            }
        }
        for &(a, b) in edges {
            if a >= n || b >= n || a == b {
                continue;
            }
            let d = pos[a] - pos[b];
            let dist = d.hypot().max(1e-6);
            let force = (dist * dist) / k;
            let dir = d / dist;
            disp[a] -= dir * force;
            disp[b] += dir * force;
        }
        let temp = 0.5 * (1.0 - iter as f64 / iters as f64);
        for i in 0..n {
            let len = disp[i].hypot();
            if len > 1e-12 {
                let step = temp.min(len);
                pos[i] = pos[i] + disp[i] * (step / len);
            }
        }
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    for p in &pos {
        cx += p.x;
        cy += p.y;
    }
    let inv = 1.0 / n as f64;
    cx *= inv;
    cy *= inv;
    let mut max_d: f64 = 0.0;
    for p in &mut pos {
        *p = Point::new(p.x - cx, p.y - cy);
        max_d = max_d.max(p.x.hypot(p.y));
    }
    if max_d > 1e-12 {
        let s = scale / max_d;
        for p in &mut pos {
            *p = Point::new(p.x * s, p.y * s);
        }
    }
    pos
}

fn layout_tree(n: usize, edges: &[(usize, usize)], scale: f64) -> Vec<Point> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![Point::ORIGIN];
    }
    let mut adj = vec![Vec::new(); n];
    for &(a, b) in edges {
        if a < n && b < n && a != b {
            adj[a].push(b);
            adj[b].push(a);
        }
    }
    let mut layer_of = vec![None; n];
    let mut next_layer = 0usize;
    for seed in 0..n {
        if layer_of[seed].is_some() {
            continue;
        }
        let mut queue = std::collections::VecDeque::new();
        layer_of[seed] = Some(next_layer);
        queue.push_back(seed);
        let mut max_l = next_layer;
        while let Some(u) = queue.pop_front() {
            let lu = layer_of[u].unwrap();
            for &v in &adj[u] {
                if layer_of[v].is_none() {
                    let lv = lu + 1;
                    layer_of[v] = Some(lv);
                    max_l = max_l.max(lv);
                    queue.push_back(v);
                }
            }
        }
        next_layer = max_l + 1;
    }
    let depth = layer_of.iter().filter_map(|l| *l).max().unwrap_or(0);
    let mut layers = vec![Vec::new(); depth + 1];
    for i in 0..n {
        layers[layer_of[i].unwrap()].push(i);
    }
    let dy = 2.0 * scale / (depth.max(1) as f64);
    let mut pos = vec![Point::ORIGIN; n];
    for (k, layer) in layers.iter().enumerate() {
        if layer.is_empty() {
            continue;
        }
        let y = scale - k as f64 * dy;
        let m = layer.len();
        for (j, &v) in layer.iter().enumerate() {
            let x = if m == 1 {
                0.0
            } else {
                -scale + j as f64 * (2.0 * scale / (m - 1) as f64)
            };
            pos[v] = Point::new(x, y);
        }
    }
    pos
}

/// Baked network graph (Manim `Graph` geometry).
///
/// Group named `"graph"` with two children:
/// - `"edges"` group: one line (or arrow if `directed`) per edge
/// - `"vertices"` group: one filled circle per vertex, in vertex order
///
/// `positions.len()` is the vertex count. Ignore edges whose indices are out of range.
/// `vertex_radius` <= 0 → use DEFAULT_DOT_RADIUS.
pub fn add_graph(
    graph: &mut SceneGraph,
    positions: &[kurbo::Point],
    edges: &[(usize, usize)],
    directed: bool,
    vertex_radius: f64,
    vertex_style: Style,
    edge_style: Style,
) -> NodeId {
    let group = graph.add(Mobject::group().named("graph"));
    let r = if vertex_radius <= 0.0 {
        DEFAULT_DOT_RADIUS
    } else {
        vertex_radius
    };
    let n = positions.len();

    let edges_g = graph.add(Mobject::group().named("edges"));
    for &(i, j) in edges {
        if i >= n || j >= n {
            continue;
        }
        let a = positions[i];
        let b = positions[j];
        if directed {
            let arrow = add_arrow(graph, a, b, r + 0.02, 0.0, edge_style.clone());
            graph.reparent(arrow, Some(edges_g));
        } else {
            graph.add_child(
                edges_g,
                Mobject::new(geometry::line(a, b)).with_style(edge_style.clone()),
            );
        }
    }
    graph.reparent(edges_g, Some(group));

    let vertices = graph.add(Mobject::group().named("vertices"));
    for (i, &pos) in positions.iter().enumerate() {
        graph.add_child(
            vertices,
            Mobject::new(geometry::circle(pos, r))
                .with_style(vertex_style.clone())
                .named(format!("vertex:{i}")),
        );
    }
    graph.reparent(vertices, Some(group));
    group
}

/// Line of length `length` tangent to a baked plot at path-local `x`.
/// Slope from points at x-0.02 and x+0.02 via `plot_point_at_x`.
/// Named `"tangent_line"`.
pub fn add_tangent_line(
    graph: &mut SceneGraph,
    plot_id: NodeId,
    x: f64,
    length: f64,
    style: Style,
) -> NodeId {
    let p = plot_point_at_x(graph, plot_id, x);
    let a = plot_point_at_x(graph, plot_id, x - 0.02);
    let b = plot_point_at_x(graph, plot_id, x + 0.02);
    let mut d = b - a;
    let len = d.hypot();
    if len < 1e-9 {
        d = Vec2::new(1.0, 0.0);
    } else {
        d = d / len;
    }
    let half = length.max(0.1) * 0.5;
    let path = geometry::line(p - d * half, p + d * half);
    graph.add(Mobject::new(path).with_style(style).named("tangent_line"))
}

/// Vertical segment from `(px, y0)` to the plot point at `x`.
/// `y0` is the axis baseline (usually 0.0). Named `"vline_to_graph"`.
pub fn add_vertical_line_to_graph(
    graph: &mut SceneGraph,
    plot_id: NodeId,
    x: f64,
    y0: f64,
    style: Style,
) -> NodeId {
    let p = plot_point_at_x(graph, plot_id, x);
    graph.add(
        Mobject::new(geometry::line(Point::new(p.x, y0), p))
            .with_style(style)
            .named("vline_to_graph"),
    )
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
            Style::default()
                .no_fill()
                .with_stroke(palette::yellow(), 4.0),
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

    #[test]
    fn polar_plane_has_circles_and_radials() {
        let mut g = SceneGraph::new();
        let p = add_polar_plane(
            &mut g,
            &PolarPlaneOpts {
                radius: 3.0,
                radius_step: 1.0,
                azimuth_divisions: 8,
                faded_line_ratio: 1,
                ..PolarPlaneOpts::default()
            },
            Style::default().with_stroke(palette::blue_d(), 2.0),
            Style::default().with_stroke(palette::white(), 3.0),
        );
        // 3 circles + 8 radials + 2 axes
        assert_eq!(g.children_of(p).len(), 13);
    }

    #[test]
    fn riemann_rects_have_n_children() {
        let mut g = SceneGraph::new();
        let id = add_riemann_rects(
            &mut g,
            0.0,
            1.0,
            6,
            1.0,
            1.0,
            |x| x * x,
            RiemannSample::Left,
            palette::blue(),
            palette::red(),
            0.7,
        );
        assert_eq!(g.children_of(id).len(), 6);
        assert_eq!(g.get(id).name.as_deref(), Some("riemann"));
    }

    #[test]
    fn complex_plane_has_children() {
        let mut g = SceneGraph::new();
        let id = add_complex_plane(
            &mut g,
            &NumberPlaneOpts {
                x_min: -2.0,
                x_max: 2.0,
                y_min: -1.0,
                y_max: 1.0,
                ..NumberPlaneOpts::default()
            },
            Style::default().with_stroke(palette::blue_d(), 2.0),
            Style::default().with_stroke(palette::white(), 3.0),
        );
        assert!(!g.children_of(id).is_empty());
        assert_eq!(g.get(id).name.as_deref(), Some("complex_plane"));
    }

    #[test]
    fn dashed_copy_of_circle_is_shorter() {
        let mut g = SceneGraph::new();
        let c = g.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        let d = add_dashed_copy(&mut g, c, 8, 0.5);
        let sl = geometry::path_length(&g.get(c).path);
        let dl = geometry::path_length(&g.get(d).path);
        assert!(dl < sl, "dashed={dl} solid={sl}");
        assert_eq!(g.get(d).name.as_deref(), Some("dashed"));
    }

    #[test]
    fn add_area_between_is_named_area() {
        let mut g = SceneGraph::new();
        let id = add_area_between(
            &mut g,
            -1.0,
            1.0,
            16,
            1.0,
            1.0,
            |_| 1.0,
            |_| 0.0,
            Style::default(),
        );
        assert_eq!(g.get(id).name.as_deref(), Some("area"));
    }

    #[test]
    fn dashed_copy_of_grouped_lines_has_two_children() {
        let mut g = SceneGraph::new();
        let a = g.add(Mobject::new(geometry::line(
            Point::new(-1.0, 0.0),
            Point::new(1.0, 0.0),
        )));
        let b = g.add(Mobject::new(geometry::line(
            Point::new(0.0, -1.0),
            Point::new(0.0, 1.0),
        )));
        let group = g.group_nodes(&[a, b]);
        let d = add_dashed_copy(&mut g, group, 8, 0.5);
        assert_eq!(g.children_of(d).len(), 2);
        assert_eq!(g.get(d).name.as_deref(), Some("dashed"));
    }

    #[test]
    fn default_number_line_tick_values_nonempty() {
        let vals = number_line_tick_values(&NumberLineOpts::default());
        assert!(!vals.is_empty());
    }

    #[test]
    fn constant_arrow_field_has_several_children() {
        let mut g = SceneGraph::new();
        let id = add_arrow_field(
            &mut g,
            -1.0,
            1.0,
            -1.0,
            1.0,
            1.0,
            1.0,
            |_, _| 1.0,
            |_, _| 0.0,
            0.45,
            Style::default(),
        );
        let n = g.children_of(id).len();
        assert!(n >= 9, "expected a 3x3 field of arrows, got {n}");
        assert_eq!(g.get(id).name.as_deref(), Some("arrow_field"));
    }

    #[test]
    fn curved_double_arrow_has_shaft_and_two_tips() {
        let mut g = SceneGraph::new();
        let id = add_curved_double_arrow(
            &mut g,
            Point::new(-1.0, 0.0),
            Point::new(1.0, 0.0),
            std::f64::consts::FRAC_PI_2,
            Style::default(),
        );
        assert_eq!(g.children_of(id).len(), 3);
        assert_eq!(g.get(id).name.as_deref(), Some("curved_double_arrow"));
    }

    #[test]
    fn bar_chart_has_one_rect_per_value() {
        let mut g = SceneGraph::new();
        let id = add_bar_chart(
            &mut g,
            &[1.0, 2.0, 3.0],
            0.0,
            3.0,
            4.0,
            3.0,
            0.6,
            &[],
            0.7,
            2.0,
        );
        let kids = g.children_of(id);
        assert_eq!(kids.len(), 2, "bars group + axes group");
        assert_eq!(g.children_of(kids[0]).len(), 3);
        assert_eq!(g.get(id).name.as_deref(), Some("barchart"));
    }

    #[test]
    fn plot_point_at_x_follows_parent_transform() {
        let mut g = SceneGraph::new();
        let axes = add_axes(&mut g, &AxesOpts::default(), Style::default());
        let plot = g.add_child(
            axes,
            Mobject::new(geometry::plot(-2.0, 2.0, 17, 1.0, 1.0, |x| x)),
        );
        g.shift(axes, kurbo::Vec2::new(2.0, 0.5));
        let p = plot_point_at_x(&g, plot, 1.0);
        assert!(
            (p.x - 3.0).abs() < 1e-9 && (p.y - 1.5).abs() < 1e-9,
            "{p:?}"
        );
    }

    #[test]
    fn circular_graph_has_n_vertices_and_m_edges() {
        let mut g = SceneGraph::new();
        let edges = [(0, 1), (1, 2), (2, 3), (3, 0)];
        let pos = layout_graph(4, &edges, "circular", 2.5);
        let id = add_graph(
            &mut g,
            &pos,
            &edges,
            false,
            0.1,
            Style::filled(palette::blue()),
            Style::default().with_stroke(palette::white(), 2.0),
        );
        assert_eq!(g.get(id).name.as_deref(), Some("graph"));
        let kids = g.children_of(id);
        assert_eq!(kids.len(), 2);
        assert_eq!(g.get(kids[0]).name.as_deref(), Some("edges"));
        assert_eq!(g.get(kids[1]).name.as_deref(), Some("vertices"));
        assert_eq!(g.children_of(kids[0]).len(), 4);
        assert_eq!(g.children_of(kids[1]).len(), 4);
    }

    #[test]
    fn directed_graph_edges_are_arrows() {
        let mut g = SceneGraph::new();
        let edges = [(0, 1)];
        let pos = layout_graph(2, &edges, "circular", 2.5);
        let id = add_graph(
            &mut g,
            &pos,
            &edges,
            true,
            0.1,
            Style::filled(palette::blue()),
            Style::default(),
        );
        let kids = g.children_of(id);
        let edges_g = kids
            .iter()
            .copied()
            .find(|&c| g.get(c).name.as_deref() == Some("edges"))
            .unwrap();
        assert_eq!(g.children_of(edges_g).len(), 1);
        let arrow = g.children_of(edges_g)[0];
        assert_eq!(g.children_of(arrow).len(), 2);
    }

    #[test]
    fn tree_layout_root_is_highest() {
        let edges = [(0, 1), (0, 2)];
        let pos = layout_graph(3, &edges, "tree", 2.5);
        assert!(pos[0].y > pos[1].y);
        assert!(pos[0].y > pos[2].y);
    }

    #[test]
    fn tangent_line_is_named() {
        let mut g = SceneGraph::new();
        let plot = g.add(Mobject::new(geometry::plot(-2.0, 2.0, 17, 1.0, 1.0, |x| x)));
        let id = add_tangent_line(&mut g, plot, 1.0, 2.0, Style::default());
        assert_eq!(g.get(id).name.as_deref(), Some("tangent_line"));
        assert!(g.bounding_box(id).width() > 1.0);
    }

    #[test]
    fn vertical_line_to_graph_reaches_curve() {
        let mut g = SceneGraph::new();
        let plot = g.add(Mobject::new(geometry::plot(-2.0, 2.0, 17, 1.0, 1.0, |x| x)));
        let id = add_vertical_line_to_graph(&mut g, plot, 1.0, 0.0, Style::default());
        assert_eq!(g.get(id).name.as_deref(), Some("vline_to_graph"));
        let bb = g.bounding_box(id);
        assert!((bb.y1 - 1.0).abs() < 1e-6, "{bb:?}");
    }

    #[test]
    fn add_implicit_curve_is_named() {
        let mut g = SceneGraph::new();
        let id = add_implicit_curve(
            &mut g,
            -1.5,
            1.5,
            -1.5,
            1.5,
            16,
            16,
            |x, y| x * x + y * y - 1.0,
            Style::default(),
        );
        assert_eq!(g.get(id).name.as_deref(), Some("implicit"));
    }
}
