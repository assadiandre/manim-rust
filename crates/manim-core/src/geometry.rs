//! Geometry kernel: shape constructors plus the path resampling/trimming
//! machinery that `Create` and morph animations are built on.
//!
//! Everything is a flat `BezPath`; resampling converts to polylines so two
//! arbitrary paths always have a common parameterization for interpolation.

use kurbo::{
    Arc, BezPath, Circle, Ellipse, ParamCurveArclen, PathEl, Point, Rect, RoundedRect, Shape, Vec2,
};

use crate::constants::{DEFAULT_ARROW_TIP_LENGTH, DEFAULT_DASH_LENGTH};

/// Tolerance for curve flattening, in logical units. At the default camera
/// (8 units = 1080 px) 1e-3 units is well under a pixel.
pub const FLATTEN_TOL: f64 = 1e-3;

pub fn circle(center: Point, radius: f64) -> BezPath {
    Circle::new(center, radius).to_path(FLATTEN_TOL)
}

pub fn rect(center: Point, width: f64, height: f64) -> BezPath {
    Rect::from_center_size(center, (width, height)).to_path(FLATTEN_TOL)
}

pub fn square(center: Point, side: f64) -> BezPath {
    rect(center, side, side)
}

pub fn line(a: Point, b: Point) -> BezPath {
    let mut p = BezPath::new();
    p.move_to(a);
    p.line_to(b);
    p
}

pub fn regular_polygon(center: Point, sides: usize, radius: f64, rotation: f64) -> BezPath {
    assert!(sides >= 3);
    let mut p = BezPath::new();
    for i in 0..sides {
        let angle = rotation + i as f64 / sides as f64 * std::f64::consts::TAU;
        let pt = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);
        if i == 0 {
            p.move_to(pt);
        } else {
            p.line_to(pt);
        }
    }
    p.close_path();
    p
}

pub fn triangle(center: Point, radius: f64) -> BezPath {
    regular_polygon(center, 3, radius, std::f64::consts::FRAC_PI_2)
}

pub fn ellipse(center: Point, rx: f64, ry: f64) -> BezPath {
    Ellipse::new(center, (rx, ry), 0.0).to_path(FLATTEN_TOL)
}

/// Circular arc. `start_angle` and `sweep` are radians; 0 is +x, CCW positive
/// (Manim / standard math).
pub fn arc(center: Point, radius: f64, start_angle: f64, sweep: f64) -> BezPath {
    Arc::new(center, (radius, radius), start_angle, sweep, 0.0).to_path(FLATTEN_TOL)
}

/// Pie slice: arc plus radii to the center.
pub fn sector(center: Point, radius: f64, start_angle: f64, sweep: f64) -> BezPath {
    let mut p = arc(center, radius, start_angle, sweep);
    p.line_to(center);
    p.close_path();
    p
}

/// Filled ring. Outer CCW, inner CW so NonZero fill punches a hole.
pub fn annulus(center: Point, inner: f64, outer: f64) -> BezPath {
    let mut p = circle(center, outer);
    let n = 64;
    for i in 0..=n {
        let a = -(i as f64) / n as f64 * std::f64::consts::TAU;
        let pt = center + Vec2::new(a.cos() * inner, a.sin() * inner);
        if i == 0 {
            p.move_to(pt);
        } else {
            p.line_to(pt);
        }
    }
    p.close_path();
    p
}

pub fn rounded_rect(center: Point, width: f64, height: f64, radius: f64) -> BezPath {
    let rect = Rect::from_center_size(center, (width, height));
    RoundedRect::from_rect(rect, radius).to_path(FLATTEN_TOL)
}

pub fn polygon(points: &[Point]) -> BezPath {
    points_to_path(points, true)
}

pub fn polyline(points: &[Point]) -> BezPath {
    points_to_path(points, false)
}

/// Shorten both ends of a segment by `buff` (Manim `Line(buff=...)`).
pub fn shorten(start: Point, end: Point, buff: f64) -> (Point, Point) {
    shorten_asymmetric(start, end, buff, buff)
}

pub fn shorten_asymmetric(start: Point, end: Point, head: f64, tail: f64) -> (Point, Point) {
    let v = end - start;
    let len = v.hypot();
    if len <= head + tail + 1e-9 {
        let mid = start.lerp(end, 0.5);
        return (mid, mid);
    }
    let dir = v / len;
    (start + dir * head, end - dir * tail)
}

pub fn dashed_line(a: Point, b: Point, dash: f64, gap: f64) -> BezPath {
    let dash = dash.max(1e-6);
    let gap = gap.max(0.0);
    let v = b - a;
    let len = v.hypot();
    if len < 1e-12 {
        return BezPath::new();
    }
    let dir = v / len;
    let mut path = BezPath::new();
    let mut s = 0.0;
    let period = dash + gap;
    while s < len - 1e-12 {
        let e = (s + dash).min(len);
        path.move_to(a + dir * s);
        path.line_to(a + dir * e);
        s += period;
    }
    path
}

pub fn dashed_line_default(a: Point, b: Point) -> BezPath {
    dashed_line(a, b, DEFAULT_DASH_LENGTH * 3.0, DEFAULT_DASH_LENGTH * 2.0)
}

/// Shaft of an arrow: line from `start` to the tip base.
pub fn arrow_shaft(start: Point, end: Point, tip_length: f64) -> BezPath {
    let v = end - start;
    let len = v.hypot();
    let tip = tip_length.min(len * 0.45).max(0.0);
    if len <= tip + 1e-9 {
        return BezPath::new();
    }
    let dir = v / len;
    line(start, end - dir * tip)
}

/// Filled triangular arrow tip pointing at `end`.
pub fn arrow_tip(start: Point, end: Point, tip_length: f64, tip_width: f64) -> BezPath {
    let v = end - start;
    let len = v.hypot();
    if len < 1e-12 {
        return BezPath::new();
    }
    let dir = v / len;
    let tip = tip_length.min(len * 0.45).max(0.0);
    let width = if tip_width <= 0.0 { tip * 0.7 } else { tip_width };
    let perp = Vec2::new(-dir.y, dir.x);
    let base = end - dir * tip;
    polygon(&[end, base + perp * (width * 0.5), base - perp * (width * 0.5)])
}

pub fn default_tip_length(length: f64) -> f64 {
    DEFAULT_ARROW_TIP_LENGTH.min(0.25 * length)
}

/// Sample `f` on `[x_min, x_max]` into a polyline, scaled into scene units.
pub fn plot(x_min: f64, x_max: f64, samples: usize, unit_x: f64, unit_y: f64, f: impl Fn(f64) -> f64) -> BezPath {
    let n = samples.max(2);
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let x = x_min + (x_max - x_min) * t;
        pts.push(Point::new(x * unit_x, f(x) * unit_y));
    }
    polyline(&pts)
}

/// Total arc length, computed per segment (accurate for curves).
pub fn path_length(path: &BezPath) -> f64 {
    path.segments().map(|s| s.arclen(1e-9)).sum()
}

/// Flatten to a polyline. Returns (points, closed).
pub fn flatten_points(path: &BezPath) -> (Vec<Point>, bool) {
    let mut pts = Vec::new();
    let mut closed = false;
    kurbo::flatten(path.clone(), FLATTEN_TOL, |el| match el {
        PathEl::MoveTo(p) => pts.push(p),
        PathEl::LineTo(p) => pts.push(p),
        PathEl::ClosePath => closed = true,
        _ => unreachable!("flatten only emits moveto/lineto/closepath"),
    });
    (pts, closed)
}

fn cumulative_lengths(pts: &[Point], closed: bool) -> Vec<f64> {
    let mut cum = Vec::with_capacity(pts.len() + 1);
    cum.push(0.0);
    for w in pts.windows(2) {
        cum.push(cum.last().unwrap() + w[0].distance(w[1]));
    }
    if closed && pts.len() > 1 {
        cum.push(cum.last().unwrap() + pts[pts.len() - 1].distance(pts[0]));
    }
    cum
}

/// Point at arc length `s` along the flattened path.
fn point_at_length(pts: &[Point], cum: &[f64], closed: bool, s: f64) -> Point {
    if pts.is_empty() {
        return Point::ORIGIN;
    }
    let total = *cum.last().unwrap();
    if total <= f64::EPSILON {
        return pts[0];
    }
    let s = s.clamp(0.0, total);
    // Binary search for the segment containing s. Clamp so that s == total
    // lands in the final segment rather than past it.
    let idx = match cum.binary_search_by(|c| c.partial_cmp(&s).unwrap()) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    }
    .min(cum.len().saturating_sub(2));
    let seg_len = cum[idx + 1] - cum[idx];
    let t = if seg_len <= f64::EPSILON {
        0.0
    } else {
        (s - cum[idx]) / seg_len
    };
    let a = pts[idx];
    let b = if idx + 1 < pts.len() {
        pts[idx + 1]
    } else if closed {
        pts[0]
    } else {
        pts[pts.len() - 1]
    };
    a.lerp(b, t)
}

/// Resample to exactly `n` evenly spaced points (by arc length).
pub fn resample(path: &BezPath, n: usize) -> (Vec<Point>, bool) {
    let (pts, closed) = flatten_points(path);
    if pts.len() < 2 || n == 0 {
        return (pts, closed);
    }
    let cum = cumulative_lengths(&pts, closed);
    let total = *cum.last().unwrap();
    let denom = if closed { n } else { n - 1 };
    let out: Vec<Point> = (0..n)
        .map(|i| point_at_length(&pts, &cum, closed, total * i as f64 / denom.max(1) as f64))
        .collect();
    (out, closed)
}

/// Build a path from points (polyline; curves were flattened upstream).
pub fn points_to_path(points: &[Point], closed: bool) -> BezPath {
    let mut p = BezPath::new();
    if let Some((first, rest)) = points.split_first() {
        p.move_to(*first);
        for pt in rest {
            p.line_to(*pt);
        }
        if closed {
            p.close_path();
        }
    }
    p
}

/// The sub-path covering arc-length fraction `t0..t1` (0..=1). Always open —
/// used for stroke-reveal (`Create`) animations.
pub fn trim(path: &BezPath, t0: f64, t1: f64) -> BezPath {
    let len = path_length(path);
    // Density scales with length so long paths stay smooth.
    let n = ((len * 64.0) as usize).clamp(128, 4096);
    let (pts, closed) = flatten_points(path);
    if pts.len() < 2 {
        return BezPath::new();
    }
    let cum = cumulative_lengths(&pts, closed);
    let total = *cum.last().unwrap();
    let (s0, s1) = (total * t0.clamp(0.0, 1.0), total * t1.clamp(0.0, 1.0));
    if s1 - s0 <= f64::EPSILON {
        return BezPath::new();
    }
    let count = ((t1 - t0).abs() * n as f64).ceil().max(2.0) as usize;
    let out: Vec<Point> = (0..count)
        .map(|i| point_at_length(&pts, &cum, closed, s0 + (s1 - s0) * i as f64 / (count - 1) as f64))
        .collect();
    points_to_path(&out, false)
}

/// Interpolate between two arbitrary paths via common resampling.
pub fn lerp_paths(a: &BezPath, b: &BezPath, samples: usize, t: f64) -> BezPath {
    let (pa, closed_a) = resample(a, samples);
    let (pb, _) = resample(b, samples);
    if pa.is_empty() {
        return b.clone();
    }
    if pb.is_empty() {
        return a.clone();
    }
    let out: Vec<Point> = pa
        .iter()
        .zip(pb.iter())
        .map(|(x, y)| x.lerp(*y, t))
        .collect();
    points_to_path(&out, closed_a)
}

/// Tight-ish bounding box (from curve segments, not just control points).
pub fn bounding_box(path: &BezPath) -> Rect {
    path.bounding_box()
}

/// Centroid of the bounding box — the pivot Manim uses for scale/rotate.
pub fn center(path: &BezPath) -> Point {
    bounding_box(path).center()
}
