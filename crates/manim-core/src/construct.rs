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

