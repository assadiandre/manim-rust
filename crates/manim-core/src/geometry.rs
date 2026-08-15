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
    let width = if tip_width <= 0.0 {
        tip * 0.7
    } else {
        tip_width
    };
    let perp = Vec2::new(-dir.y, dir.x);
    let base = end - dir * tip;
    polygon(&[
        end,
        base + perp * (width * 0.5),
        base - perp * (width * 0.5),
    ])
}

pub fn default_tip_length(length: f64) -> f64 {
    DEFAULT_ARROW_TIP_LENGTH.min(0.25 * length)
}

/// Parametric polyline `t ∈ [t0, t1] → (x(t), y(t))`.
pub fn parametric(t0: f64, t1: f64, samples: usize, f: impl Fn(f64) -> Point) -> BezPath {
    let n = samples.max(2);
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let t = t0 + (t1 - t0) * i as f64 / (n - 1) as f64;
        pts.push(f(t));
    }
    polyline(&pts)
}

/// L-shaped elbow at `corner`, arms of length `size` along +x and +y after `angle`.
pub fn elbow(corner: Point, size: f64, angle: f64) -> BezPath {
    let a = Vec2::new(angle.cos(), angle.sin()) * size;
    let b = Vec2::new(-angle.sin(), angle.cos()) * size;
    polyline(&[corner + a, corner, corner + b])
}

/// Curly brace of `width`, tip pointing +y, centered on the origin.
pub fn brace(width: f64) -> BezPath {
    let half = (width * 0.5).max(0.25);
    let depth = 0.22;
    let tip = 0.16;
    let mut p = BezPath::new();
    p.move_to(Point::new(-half, 0.0));
    p.curve_to(
        Point::new(-half + 0.12, 0.0),
        Point::new(-half * 0.45, -depth),
        Point::new(-0.14, -depth),
    );
    p.curve_to(
        Point::new(-0.05, -depth),
        Point::new(-0.03, tip),
        Point::new(0.0, tip),
    );
    p.curve_to(
        Point::new(0.03, tip),
        Point::new(0.05, -depth),
        Point::new(0.14, -depth),
    );
    p.curve_to(
        Point::new(half * 0.45, -depth),
        Point::new(half - 0.12, 0.0),
        Point::new(half, 0.0),
    );
    p
}

/// Star polygon (Manim `Star`). `inner` `None` uses the CE density-2 radius.
pub fn star(center: Point, n: usize, outer: f64, inner: Option<f64>, rotation: f64) -> BezPath {
    let n = n.max(3);
    let inner_angle = std::f64::consts::TAU / (2.0 * n as f64);
    let inner_r = inner.unwrap_or_else(|| {
        let density = 2.0_f64;
        let outer_angle = std::f64::consts::TAU * density / n as f64;
        let inverse_x = 1.0 - inner_angle.tan() * ((outer_angle.cos() - 1.0) / outer_angle.sin());
        outer / (inner_angle.cos() * inverse_x)
    });
    let mut pts = Vec::with_capacity(2 * n);
    for i in 0..n {
        let ao = rotation + i as f64 / n as f64 * std::f64::consts::TAU;
        let ai = ao + inner_angle;
        pts.push(center + Vec2::new(ao.cos() * outer, ao.sin() * outer));
        pts.push(center + Vec2::new(ai.cos() * inner_r, ai.sin() * inner_r));
    }
    polygon(&pts)
}

/// Circular arc from `start` to `end` with signed sweep (Manim `ArcBetweenPoints`).
pub fn arc_between_points(start: Point, end: Point, sweep: f64) -> BezPath {
    if sweep.abs() < 1e-9 {
        return line(start, end);
    }
    let chord = end - start;
    let dist = chord.hypot();
    if dist < 1e-12 {
        return BezPath::new();
    }
    let r = dist / (2.0 * (sweep.abs() / 2.0).sin());
    let mid = start.lerp(end, 0.5);
    let perp = Vec2::new(-chord.y, chord.x) / dist;
    let offset = r * (sweep.abs() / 2.0).cos();
    let center = mid + perp * offset * sweep.signum();
    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    arc(center, r, start_angle, sweep)
}

/// Ring slice (Manim `AnnularSector`).
pub fn annular_sector(center: Point, inner: f64, outer: f64, start: f64, sweep: f64) -> BezPath {
    let n = 32.max((sweep.abs() * 24.0) as usize);
    let mut p = BezPath::new();
    for i in 0..=n {
        let a = start + sweep * i as f64 / n as f64;
        let pt = center + Vec2::new(a.cos() * outer, a.sin() * outer);
        if i == 0 {
            p.move_to(pt);
        } else {
            p.line_to(pt);
        }
    }
    for i in 0..=n {
        let a = start + sweep * (1.0 - i as f64 / n as f64);
        p.line_to(center + Vec2::new(a.cos() * inner, a.sin() * inner));
    }
    p.close_path();
    p
}

/// CCW arc from `p1` to `p2` about `vertex` (Manim `Angle`).
pub fn angle_arc(vertex: Point, p1: Point, p2: Point, radius: f64) -> BezPath {
    let a0 = (p1.y - vertex.y).atan2(p1.x - vertex.x);
    let a1 = (p2.y - vertex.y).atan2(p2.x - vertex.x);
    let mut sweep = a1 - a0;
    if sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }
    arc(vertex, radius, a0, sweep)
}

/// Square corner between two directions (Manim `RightAngle`).
pub fn right_angle(vertex: Point, p1: Point, p2: Point, size: f64) -> BezPath {
    let n1 = {
        let v = p1 - vertex;
        let len = v.hypot();
        if len < 1e-12 {
            Vec2::new(1.0, 0.0)
        } else {
            v / len
        }
    };
    let n2 = {
        let v = p2 - vertex;
        let len = v.hypot();
        if len < 1e-12 {
            Vec2::new(0.0, 1.0)
        } else {
            v / len
        }
    };
    polyline(&[
        vertex + n1 * size,
        vertex + n1 * size + n2 * size,
        vertex + n2 * size,
    ])
}

pub fn cubic_bezier(p0: Point, p1: Point, p2: Point, p3: Point) -> BezPath {
    let mut p = BezPath::new();
    p.move_to(p0);
    p.curve_to(p1, p2, p3);
    p
}

/// Point at arc-length fraction `t` ∈ [0, 1].
pub fn point_along(path: &BezPath, t: f64) -> Point {
    let (pts, closed) = flatten_points(path);
    if pts.is_empty() {
        return Point::ORIGIN;
    }
    let cum = cumulative_lengths(&pts, closed);
    let total = *cum.last().unwrap();
    point_at_length(&pts, &cum, closed, total * t.clamp(0.0, 1.0))
}

/// Unit tangent at arc-length fraction `t`.
pub fn tangent_along(path: &BezPath, t: f64) -> Vec2 {
    let a = point_along(path, (t - 0.02).max(0.0));
    let b = point_along(path, (t + 0.02).min(1.0));
    let v = b - a;
    let len = v.hypot();
    if len < 1e-12 {
        Vec2::new(1.0, 0.0)
    } else {
        v / len
    }
}

/// Point on a polyline whose x-coordinate matches `x` (lerp in the bracketing
/// segment). Out-of-range `x` snaps to the nearer endpoint.
pub fn point_at_x(path: &BezPath, x: f64) -> Point {
    let (pts, _) = flatten_points(path);
    if pts.is_empty() {
        return Point::ORIGIN;
    }
    if pts.len() == 1 {
        return pts[0];
    }
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (lo, hi) = if a.x <= b.x { (a.x, b.x) } else { (b.x, a.x) };
        if x >= lo - 1e-12 && x <= hi + 1e-12 {
            let dx = b.x - a.x;
            let t = if dx.abs() < 1e-12 {
                0.0
            } else {
                (x - a.x) / dx
            };
            return a.lerp(b, t);
        }
    }
    let first = pts[0];
    let last = pts[pts.len() - 1];
    if (x - first.x).abs() <= (x - last.x).abs() {
        first
    } else {
        last
    }
}

/// Sample `f` on `[x_min, x_max]` into a polyline, scaled into scene units.
pub fn plot(
    x_min: f64,
    x_max: f64,
    samples: usize,
    unit_x: f64,
    unit_y: f64,
    f: impl Fn(f64) -> f64,
) -> BezPath {
    let n = samples.max(2);
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let x = x_min + (x_max - x_min) * t;
        pts.push(Point::new(x * unit_x, f(x) * unit_y));
    }
    polyline(&pts)
}

/// Marching-squares isoline `f(x,y) = 0` on a uniform grid.
///
/// `nx`/`ny` are cell counts (clamped to at least 2). Each cell with a
/// sign change emits one or two line segments. Degenerate zeros on a
/// corner count as negative (use a tiny epsilon) so edges don't vanish.
pub fn implicit_curve(
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    nx: usize,
    ny: usize,
    f: impl Fn(f64, f64) -> f64,
) -> BezPath {
    const EPS: f64 = 1e-12;
    let nx = nx.max(2);
    let ny = ny.max(2);
    let dx = (x_max - x_min) / nx as f64;
    let dy = (y_max - y_min) / ny as f64;
    let cols = nx + 1;
    let rows = ny + 1;

    let mut values = vec![0.0; rows * cols];
    for j in 0..rows {
        let y = y_min + j as f64 * dy;
        for i in 0..cols {
            let x = x_min + i as f64 * dx;
            values[j * cols + i] = f(x, y);
        }
    }

    let neg = |v: f64| v < EPS;
    let crossing = |v0: f64, v1: f64, p0: Point, p1: Point| -> Point {
        let denom = v1 - v0;
        let t = if denom.abs() < EPS {
            0.5
        } else {
            (-v0 / denom).clamp(0.0, 1.0)
        };
        p0.lerp(p1, t)
    };

    let mut path = BezPath::new();
    let mut emit = |a: Point, b: Point| {
        path.move_to(a);
        path.line_to(b);
    };

    for j in 0..ny {
        let y0 = y_min + j as f64 * dy;
        let y1 = y0 + dy;
        for i in 0..nx {
            let x0 = x_min + i as f64 * dx;
            let x1 = x0 + dx;
            let sw = values[j * cols + i];
            let se = values[j * cols + i + 1];
            let ne = values[(j + 1) * cols + i + 1];
            let nw = values[(j + 1) * cols + i];

            let mut case = 0u8;
            if neg(sw) {
                case |= 1;
            }
            if neg(se) {
                case |= 2;
            }
            if neg(ne) {
                case |= 4;
            }
            if neg(nw) {
                case |= 8;
            }
            if case == 0 || case == 15 {
                continue;
            }

            let p_sw = Point::new(x0, y0);
            let p_se = Point::new(x1, y0);
            let p_ne = Point::new(x1, y1);
            let p_nw = Point::new(x0, y1);
            let south = crossing(sw, se, p_sw, p_se);
            let east = crossing(se, ne, p_se, p_ne);
            let north = crossing(nw, ne, p_nw, p_ne);
            let west = crossing(sw, nw, p_sw, p_nw);

            match case {
                1 | 14 => emit(west, south),
                2 | 13 => emit(south, east),
                3 | 12 => emit(west, east),
                4 | 11 => emit(east, north),
                6 | 9 => emit(south, north),
                7 | 8 => emit(west, north),
                5 => {
                    // SW+NE saddle: average of the four corners picks the pairing.
                    if (sw + se + ne + nw) * 0.25 < EPS {
                        emit(west, north);
                        emit(south, east);
                    } else {
                        emit(west, south);
                        emit(east, north);
                    }
                }
                10 => {
                    // SE+NW saddle.
                    if (sw + se + ne + nw) * 0.25 < EPS {
                        emit(west, south);
                        emit(east, north);
                    } else {
                        emit(west, north);
                        emit(south, east);
                    }
                }
                _ => {}
            }
        }
    }
    path
}

/// Region under `f` on `[x_min, x_max]`, closed down to the x-axis.
/// Negative `f` is included (signed area).
pub fn area_under(
    x_min: f64,
    x_max: f64,
    samples: usize,
    unit_x: f64,
    unit_y: f64,
    f: impl Fn(f64) -> f64,
) -> BezPath {
    let mut p = plot(x_min, x_max, samples, unit_x, unit_y, f);
    p.line_to(Point::new(x_max * unit_x, 0.0));
    p.line_to(Point::new(x_min * unit_x, 0.0));
    p.close_path();
    p
}

/// Region between `f` and `g` on `[x_min, x_max]`, closed.
pub fn area_between(
    x_min: f64,
    x_max: f64,
    samples: usize,
    unit_x: f64,
    unit_y: f64,
    f: impl Fn(f64) -> f64,
    g: impl Fn(f64) -> f64,
) -> BezPath {
    let n = samples.max(2);
    let mut p = plot(x_min, x_max, samples, unit_x, unit_y, f);
    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let x = x_max + (x_min - x_max) * t;
        p.line_to(Point::new(x * unit_x, g(x) * unit_y));
    }
    p.close_path();
    p
}

/// ManimCE `DashedVMobject`: `num_dashes` equal arc-length windows, each
/// keeping the first `dashed_ratio` fraction (clamped to `0.05..=0.95`).
pub fn dashed_path(path: &BezPath, num_dashes: usize, dashed_ratio: f64) -> BezPath {
    let n = num_dashes.max(1);
    let ratio = dashed_ratio.clamp(0.05, 0.95);
    let mut out = BezPath::new();
    for i in 0..n {
        let t0 = i as f64 / n as f64;
        let t1 = t0 + ratio / n as f64;
        out.extend(trim(path, t0, t1).iter());
    }
    out
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
        .map(|i| {
            point_at_length(
                &pts,
                &cum,
                closed,
                s0 + (s1 - s0) * i as f64 / (count - 1) as f64,
            )
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_circle_implicit_is_closedish() {
        let path = implicit_curve(-1.5, 1.5, -1.5, 1.5, 40, 40, |x, y| x * x + y * y - 1.0);
        let len = path_length(&path);
        assert!(
            (4.5..8.0).contains(&len),
            "unit circle path length {len} not in 4.5..8.0"
        );
        let bb = bounding_box(&path);
        assert!(
            bb.x0 < -0.9 && bb.x1 > 0.9 && bb.y0 < -0.9 && bb.y1 > 0.9,
            "bbox should cover roughly [-1, 1], got {bb:?}"
        );
        assert!(
            bb.x0 > -1.2 && bb.x1 < 1.2 && bb.y0 > -1.2 && bb.y1 < 1.2,
            "bbox should stay near the unit circle, got {bb:?}"
        );
    }

    #[test]
    fn empty_field_is_empty_path() {
        let path = implicit_curve(-1.0, 1.0, -1.0, 1.0, 8, 8, |_, _| 1.0);
        assert!(
            path.is_empty(),
            "constant-positive field should emit no segments"
        );
    }
}
