//! Authoring-time boolean path operations (union / intersection / difference / xor).
//!
//! Paths are flattened once into closed contours and handed to `i_overlay`.
//! The result is a new `BezPath` — never a per-frame Python callback.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use kurbo::{BezPath, PathEl, Point};

use crate::geometry;
use crate::mobject::Mobject;
use crate::scene::{NodeId, SceneGraph};
use crate::style::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Intersection,
    Difference,
    Exclusion,
}

impl BooleanOp {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "union" | "or" => Some(Self::Union),
            "intersection" | "and" => Some(Self::Intersection),
            "difference" | "sub" => Some(Self::Difference),
            "exclusion" | "xor" => Some(Self::Exclusion),
            _ => None,
        }
    }

    fn overlay_rule(self) -> OverlayRule {
        match self {
            Self::Union => OverlayRule::Union,
            Self::Intersection => OverlayRule::Intersect,
            Self::Difference => OverlayRule::Difference,
            Self::Exclusion => OverlayRule::Xor,
        }
    }
}

/// Flatten `path` (kurbo flatten tol 1e-3) into closed contours.
/// Split on MoveTo. Drop contours with < 3 points. Close if needed.
fn path_to_contours(path: &BezPath) -> Vec<Vec<[f64; 2]>> {
    let mut contours = Vec::new();
    let mut current: Vec<[f64; 2]> = Vec::new();

    kurbo::flatten(path.clone(), geometry::FLATTEN_TOL, |el| match el {
        PathEl::MoveTo(p) => {
            push_contour(&mut contours, &mut current);
            current.push([p.x, p.y]);
        }
        PathEl::LineTo(p) => current.push([p.x, p.y]),
        PathEl::ClosePath => {
            close_if_needed(&mut current);
            push_contour(&mut contours, &mut current);
        }
        _ => unreachable!("flatten only emits moveto/lineto/closepath"),
    });
    close_if_needed(&mut current);
    push_contour(&mut contours, &mut current);
    contours
}

fn close_if_needed(contour: &mut Vec<[f64; 2]>) {
    if contour.len() < 2 {
        return;
    }
    let first = contour[0];
    let last = *contour.last().unwrap();
    if (first[0] - last[0]).abs() < 1e-12 && (first[1] - last[1]).abs() < 1e-12 {
        contour.pop();
    }
}

fn push_contour(contours: &mut Vec<Vec<[f64; 2]>>, current: &mut Vec<[f64; 2]>) {
    if current.len() >= 3 {
        contours.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

/// Rebuild a BezPath from i_overlay Shapes (outer + holes).
fn shapes_to_path(shapes: Vec<Vec<Vec<[f64; 2]>>>) -> BezPath {
    let mut path = BezPath::new();
    for shape in shapes {
        for contour in shape {
            if contour.len() < 3 {
                continue;
            }
            let mut pts = contour.into_iter();
            if let Some([x, y]) = pts.next() {
                path.move_to(Point::new(x, y));
                for [x, y] in pts {
                    path.line_to(Point::new(x, y));
                }
                path.close_path();
            }
        }
    }
    path
}

/// Boolean of two paths in their own local coordinates.
pub fn boolean_path(a: &BezPath, b: &BezPath, op: BooleanOp) -> BezPath {
    let a_contours = path_to_contours(a);
    let b_contours = path_to_contours(b);
    if a_contours.is_empty() && b_contours.is_empty() {
        return BezPath::new();
    }
    if a_contours.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Exclusion => shapes_to_path(vec![b_contours]),
            BooleanOp::Intersection | BooleanOp::Difference => BezPath::new(),
        };
    }
    if b_contours.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Exclusion | BooleanOp::Difference => {
                shapes_to_path(vec![a_contours])
            }
            BooleanOp::Intersection => BezPath::new(),
        };
    }
    let result = a_contours.overlay_as::<i64>(&b_contours, op.overlay_rule(), FillRule::NonZero);
    shapes_to_path(result)
}

fn node_world_path(graph: &SceneGraph, id: NodeId) -> BezPath {
    let mut out = BezPath::new();
    for leaf in graph.path_leaves(id) {
        let t = graph.world_transform(leaf);
        out.extend((t * graph.get(leaf).path.clone()).iter());
    }
    if out.elements().is_empty() {
        let t = graph.world_transform(id);
        out = t * graph.get(id).path.clone();
    }
    out
}

/// World-space boolean of two scene nodes. Uses each node's world transform
/// applied to its path (and path_leaves unioned if the node is a group).
/// Adds a new root mobject named `"boolean"` with `style`.
pub fn add_boolean(
    graph: &mut SceneGraph,
    a: NodeId,
    b: NodeId,
    op: BooleanOp,
    style: Style,
) -> NodeId {
    let path = boolean_path(&node_world_path(graph, a), &node_world_path(graph, b), op);
    graph.add(Mobject::new(path).with_style(style).named("boolean"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Point;

    fn overlapping_squares() -> (BezPath, BezPath) {
        (
            geometry::square(Point::new(-0.4, 0.0), 2.0),
            geometry::square(Point::new(0.4, 0.0), 2.0),
        )
    }

    #[test]
    fn union_of_overlapping_squares_is_larger() {
        let (a, b) = overlapping_squares();
        let u = boolean_path(&a, &b, BooleanOp::Union);
        let w = geometry::bounding_box(&u).width();
        assert!(w > 2.4, "union width {w}");
    }

    #[test]
    fn intersection_of_overlapping_squares_is_smaller() {
        let (a, b) = overlapping_squares();
        let i = boolean_path(&a, &b, BooleanOp::Intersection);
        let w = geometry::bounding_box(&i).width();
        assert!(w < 1.6 && w > 0.5, "intersection width {w}");
    }

    #[test]
    fn difference_cuts_a_notch() {
        // 2×2 subject; a smaller rect bites the right side so the outer
        // width stays 2 while the intersection is removed.
        let a = geometry::square(Point::new(0.0, 0.0), 2.0);
        let b = geometry::rect(Point::new(0.7, 0.0), 1.0, 0.8);
        let d = boolean_path(&a, &b, BooleanOp::Difference);
        assert!(!d.elements().is_empty());
        let w = geometry::bounding_box(&d).width();
        assert!(w >= 2.0, "difference width {w}");
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(BooleanOp::parse("xor"), Some(BooleanOp::Exclusion));
        assert_eq!(BooleanOp::parse("union"), Some(BooleanOp::Union));
        assert_eq!(BooleanOp::parse("or"), Some(BooleanOp::Union));
        assert_eq!(BooleanOp::parse("and"), Some(BooleanOp::Intersection));
        assert_eq!(BooleanOp::parse("sub"), Some(BooleanOp::Difference));
    }
}
