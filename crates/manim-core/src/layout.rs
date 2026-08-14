//! Manim-style positioning: bounding boxes, `move_to`, `next_to`, `arrange`.
//!
//! Semantics follow `manim/mobject/mobject.py`. Shifts are applied in world
//! space and converted into the node's local transform so parented objects
//! stay correct under a rotated/scaled parent.

use kurbo::{Affine, Point, Rect, Shape, Vec2};

use crate::constants::{
    DEFAULT_MOBJECT_TO_EDGE_BUFFER, DEFAULT_MOBJECT_TO_MOBJECT_BUFFER, FRAME_X_RADIUS,
    FRAME_Y_RADIUS,
};
use crate::scene::{NodeId, SceneGraph};

fn transform_rect(t: Affine, r: Rect) -> Rect {
    let pts = [
        t * Point::new(r.x0, r.y0),
        t * Point::new(r.x1, r.y0),
        t * Point::new(r.x1, r.y1),
        t * Point::new(r.x0, r.y1),
    ];
    pts.iter()
        .skip(1)
        .fold(Rect::from_center_size(pts[0], (0.0, 0.0)), |acc, p| {
            acc.union_pt(*p)
        })
}

fn union_opt(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.union(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

impl SceneGraph {
    /// Axis-aligned bounding box of `id` and its descendants, in world space.
    pub fn bounding_box(&self, id: NodeId) -> Rect {
        self.world_family_bbox(id)
            .unwrap_or_else(|| Rect::from_center_size(Point::ORIGIN, (0.0, 0.0)))
    }

    fn world_family_bbox(&self, id: NodeId) -> Option<Rect> {
        let world = self.world_transform(id);
        let mut acc = None;
        let path = &self.get(id).path;
        if !path.elements().is_empty() {
            acc = Some(transform_rect(world, path.bounding_box()));
        }
        for &c in self.children_of(id) {
            acc = union_opt(acc, self.world_family_bbox(c));
        }
        acc
    }

    /// Family bbox in this node's local path space (before its own transform).
    /// Scale/rotate pivots use this so groups rotate about their visual center.
    pub fn local_family_bbox(&self, id: NodeId) -> Rect {
        let mut acc = None;
        let path = &self.get(id).path;
        if !path.elements().is_empty() {
            acc = Some(path.bounding_box());
        }
        for &c in self.children_of(id) {
            let child_local = self.local_family_bbox(c);
            let in_parent = transform_rect(self.get(c).transform, child_local);
            acc = union_opt(acc, Some(in_parent));
        }
        acc.unwrap_or_else(|| Rect::from_center_size(Point::ORIGIN, (0.0, 0.0)))
    }

    pub fn local_pivot(&self, id: NodeId) -> Point {
        self.local_family_bbox(id).center()
    }

    /// One of the 9 bbox critical points (center, edge midpoints, corners).
    /// `direction` is used by sign only, matching Manim's `get_critical_point`.
    pub fn critical_point(&self, id: NodeId, direction: Vec2) -> Point {
        let bb = self.bounding_box(id);
        let x = if direction.x > 0.0 {
            bb.x1
        } else if direction.x < 0.0 {
            bb.x0
        } else {
            bb.center().x
        };
        let y = if direction.y > 0.0 {
            bb.y1
        } else if direction.y < 0.0 {
            bb.y0
        } else {
            bb.center().y
        };
        Point::new(x, y)
    }

    pub fn center_of(&self, id: NodeId) -> Point {
        self.critical_point(id, Vec2::ZERO)
    }

    /// Translate `id` by a world-space delta.
    pub fn shift(&mut self, id: NodeId, delta: Vec2) {
        let parent_world = match self.parent(id) {
            Some(p) => self.world_transform(p),
            None => Affine::IDENTITY,
        };
        let new_local =
            parent_world.inverse() * Affine::translate(delta) * parent_world * self.get(id).transform;
        self.get_mut(id).transform = new_local;
    }

    /// Move so `aligned_edge` of this mobject sits on `point` (default: center).
    pub fn move_to(&mut self, id: NodeId, point: Point) {
        self.move_to_aligned(id, point, Vec2::ZERO);
    }

    pub fn move_to_aligned(&mut self, id: NodeId, point: Point, aligned_edge: Vec2) {
        let mine = self.critical_point(id, aligned_edge);
        self.shift(id, point - mine);
    }

    /// Place `id` next to `other` in `direction` (Manim `next_to`).
    pub fn next_to(&mut self, id: NodeId, other: NodeId, direction: Vec2, buff: f64) {
        self.next_to_aligned(id, other, direction, buff, Vec2::ZERO);
    }

    pub fn next_to_aligned(
        &mut self,
        id: NodeId,
        other: NodeId,
        direction: Vec2,
        buff: f64,
        aligned_edge: Vec2,
    ) {
        let target = self.critical_point(other, aligned_edge + direction);
        let mine = self.critical_point(id, aligned_edge - direction);
        self.shift(id, (target - mine) + direction * buff);
    }

    pub fn next_to_point(&mut self, id: NodeId, point: Point, direction: Vec2, buff: f64) {
        let mine = self.critical_point(id, -direction);
        self.shift(id, (point - mine) + direction * buff);
    }

    /// Align the `direction` edge of `id` with the same edge of `other`.
    pub fn align_to(&mut self, id: NodeId, other: NodeId, direction: Vec2) {
        let target = self.critical_point(other, direction);
        let mine = self.critical_point(id, direction);
        let mut delta = Vec2::ZERO;
        if direction.x != 0.0 {
            delta.x = target.x - mine.x;
        }
        if direction.y != 0.0 {
            delta.y = target.y - mine.y;
        }
        self.shift(id, delta);
    }

    /// Move to a screen edge / corner (Manim `to_edge` / `align_on_border`).
    pub fn to_edge(&mut self, id: NodeId, direction: Vec2, buff: f64) {
        // `f64::signum(0.0)` is +1, so a raw signum would treat UP as UR.
        let axis = |v: f64| if v == 0.0 { 0.0 } else { v.signum() };
        let (sx, sy) = (axis(direction.x), axis(direction.y));
        let target = Point::new(sx * FRAME_X_RADIUS, sy * FRAME_Y_RADIUS);
        let mine = self.critical_point(id, direction);
        let mut delta = (target - mine) - direction * buff;
        delta.x *= sx.abs();
        delta.y *= sy.abs();
        self.shift(id, delta);
    }

    pub fn to_edge_default(&mut self, id: NodeId, direction: Vec2) {
        self.to_edge(id, direction, DEFAULT_MOBJECT_TO_EDGE_BUFFER);
    }

    /// Arrange children of `group` along `direction`, then optionally center
    /// the group at the origin (Manim `arrange(..., center=True)`).
    pub fn arrange(&mut self, group: NodeId, direction: Vec2, buff: f64, center: bool) {
        let kids: Vec<NodeId> = self.children_of(group).to_vec();
        for pair in kids.windows(2) {
            self.next_to(pair[1], pair[0], direction, buff);
        }
        if center {
            self.move_to(group, Point::ORIGIN);
        }
    }

    pub fn arrange_default(&mut self, group: NodeId, direction: Vec2) {
        self.arrange(group, direction, DEFAULT_MOBJECT_TO_MOBJECT_BUFFER, true);
    }

    /// Alias for `to_edge` with a corner direction (Manim `to_corner`).
    pub fn to_corner(&mut self, id: NodeId, corner: Vec2, buff: f64) {
        self.to_edge(id, corner, buff);
    }

    pub fn set_x(&mut self, id: NodeId, x: f64) {
        let c = self.center_of(id);
        self.shift(id, Vec2::new(x - c.x, 0.0));
    }

    pub fn set_y(&mut self, id: NodeId, y: f64) {
        let c = self.center_of(id);
        self.shift(id, Vec2::new(0.0, y - c.y));
    }

    /// Manim `flip`: `UP`/`DOWN` mirrors in x; `LEFT`/`RIGHT` mirrors in y.
    pub fn flip(&mut self, id: NodeId, axis: Vec2) {
        let about = self.local_pivot(id);
        let (sx, sy) = if axis.y.abs() >= axis.x.abs() {
            (-1.0, 1.0)
        } else {
            (1.0, -1.0)
        };
        let flip = Affine::translate(about.to_vec2())
            * Affine::scale_non_uniform(sx, sy)
            * Affine::translate(-about.to_vec2());
        let t = self.get(id).transform;
        self.get_mut(id).transform = t * flip;
    }

    /// Non-uniform scale about the local pivot. `dim` 0 = x, 1 = y.
    pub fn stretch(&mut self, id: NodeId, factor: f64, dim: usize) {
        let about = self.local_pivot(id);
        let (sx, sy) = if dim == 0 { (factor, 1.0) } else { (1.0, factor) };
        let s = Affine::translate(about.to_vec2())
            * Affine::scale_non_uniform(sx, sy)
            * Affine::translate(-about.to_vec2());
        let t = self.get(id).transform;
        self.get_mut(id).transform = t * s;
    }

    /// Row-major grid of a group's children (Manim `arrange_in_grid`).
    pub fn arrange_in_grid(
        &mut self,
        group: NodeId,
        rows: Option<usize>,
        cols: Option<usize>,
        buff_x: f64,
        buff_y: f64,
        center: bool,
    ) {
        let kids: Vec<NodeId> = self.children_of(group).to_vec();
        if kids.is_empty() {
            return;
        }
        let n = kids.len();
        let cols = cols.unwrap_or_else(|| {
            let r = rows.unwrap_or((n as f64).sqrt().ceil() as usize).max(1);
            n.div_ceil(r)
        });
        let cols = cols.max(1);
        for (i, &id) in kids.iter().enumerate() {
            let col = i % cols;
            if col > 0 {
                self.next_to(id, kids[i - 1], crate::constants::RIGHT, buff_x);
                self.align_to(id, kids[i - col], crate::constants::UP);
            } else if i > 0 {
                self.next_to(id, kids[i - cols], crate::constants::DOWN, buff_y);
            }
        }
        if center {
            self.move_to(group, Point::ORIGIN);
        }
    }

    /// Uniform scale about the local visual center.
    pub fn scale_about_center(&mut self, id: NodeId, factor: f64) {
        let about = self.local_pivot(id);
        let s = Affine::translate(about.to_vec2())
            * Affine::scale(factor)
            * Affine::translate(-about.to_vec2());
        let t = self.get(id).transform;
        self.get_mut(id).transform = t * s;
    }

    /// Rotate about the local visual center (Manim `rotate`).
    pub fn rotate_about_center(&mut self, id: NodeId, angle: f64) {
        let about = self.local_pivot(id);
        let t = self.get(id).transform;
        self.get_mut(id).transform = t * Affine::rotate_about(angle, about);
    }

    /// Scale so the world-space bbox width becomes `width` (Manim `set_width`).
    pub fn set_width(&mut self, id: NodeId, width: f64) {
        let w = self.bounding_box(id).width();
        if w > 1e-9 && width > 0.0 {
            self.scale_about_center(id, width / w);
        }
    }

    /// Scale so the world-space bbox height becomes `height` (Manim `set_height`).
    pub fn set_height(&mut self, id: NodeId, height: f64) {
        let h = self.bounding_box(id).height();
        if h > 1e-9 && height > 0.0 {
            self.scale_about_center(id, height / h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DOWN, FRAME_Y_RADIUS, LEFT, RIGHT, UP};
    use crate::geometry;
    use crate::mobject::Mobject;

    fn circle_at(g: &mut SceneGraph, p: Point, r: f64) -> NodeId {
        g.add(Mobject::new(geometry::circle(p, r)))
    }

    #[test]
    fn center_of_origin_circle() {
        let mut g = SceneGraph::new();
        let c = circle_at(&mut g, Point::ORIGIN, 1.0);
        let p = g.center_of(c);
        assert!(p.x.abs() < 1e-9 && p.y.abs() < 1e-9);
    }

    #[test]
    fn move_to_shifts_center() {
        let mut g = SceneGraph::new();
        let c = circle_at(&mut g, Point::ORIGIN, 1.0);
        g.move_to(c, Point::new(2.0, -1.0));
        let p = g.center_of(c);
        assert!((p.x - 2.0).abs() < 1e-9 && (p.y + 1.0).abs() < 1e-9);
    }

    #[test]
    fn next_to_right_leaves_a_buff() {
        let mut g = SceneGraph::new();
        let a = circle_at(&mut g, Point::ORIGIN, 1.0);
        let b = circle_at(&mut g, Point::ORIGIN, 0.5);
        g.next_to(b, a, RIGHT, 0.25);
        // a's right edge is x=1; b's left edge should be 1.25; b radius 0.5
        // so b's center x = 1.75
        let p = g.center_of(b);
        assert!((p.x - 1.75).abs() < 1e-6, "x={}", p.x);
        assert!(p.y.abs() < 1e-6);
    }

    #[test]
    fn next_to_down() {
        let mut g = SceneGraph::new();
        let a = circle_at(&mut g, Point::ORIGIN, 1.0);
        let b = circle_at(&mut g, Point::ORIGIN, 1.0);
        g.next_to(b, a, DOWN, 0.5);
        let p = g.center_of(b);
        // a bottom = -1, buff 0.5, b top aligns there → center = -1 - 0.5 - 1 = -2.5
        assert!((p.y + 2.5).abs() < 1e-6, "y={}", p.y);
    }

    #[test]
    fn align_to_top() {
        let mut g = SceneGraph::new();
        let a = circle_at(&mut g, Point::new(0.0, 2.0), 1.0);
        let b = circle_at(&mut g, Point::ORIGIN, 0.5);
        g.align_to(b, a, UP);
        // a's top is y=3; b's top should become 3 → center y = 2.5
        let p = g.center_of(b);
        assert!((p.y - 2.5).abs() < 1e-6, "y={}", p.y);
        assert!(p.x.abs() < 1e-6, "align_to UP must not move x");
    }

    #[test]
    fn to_edge_up_keeps_x() {
        let mut g = SceneGraph::new();
        let c = circle_at(&mut g, Point::ORIGIN, 0.5);
        g.to_edge(c, UP, 0.5);
        let p = g.center_of(c);
        assert!(p.x.abs() < 1e-6, "to_edge UP must not move x, got {p:?}");
        let top = g.critical_point(c, UP);
        assert!((top.y - (FRAME_Y_RADIUS - 0.5)).abs() < 1e-6, "top.y={}", top.y);
    }

    #[test]
    fn to_edge_left() {
        let mut g = SceneGraph::new();
        let c = circle_at(&mut g, Point::ORIGIN, 1.0);
        g.to_edge(c, LEFT, 0.5);
        let left = g.critical_point(c, LEFT);
        assert!(
            (left.x - (-FRAME_X_RADIUS + 0.5)).abs() < 1e-6,
            "left.x={}",
            left.x
        );
    }

    #[test]
    fn arrange_row() {
        let mut g = SceneGraph::new();
        let a = circle_at(&mut g, Point::ORIGIN, 0.5);
        let b = circle_at(&mut g, Point::ORIGIN, 0.5);
        let c = circle_at(&mut g, Point::ORIGIN, 0.5);
        let grp = g.group_nodes(&[a, b, c]);
        g.arrange(grp, RIGHT, 0.25, true);
        let pa = g.center_of(a);
        let pb = g.center_of(b);
        let pc = g.center_of(c);
        // diameter 1 + buff 0.25 = 1.25 between centers
        assert!((pb.x - pa.x - 1.25).abs() < 1e-6);
        assert!((pc.x - pb.x - 1.25).abs() < 1e-6);
        // group centered at origin
        let mid = g.center_of(grp);
        assert!(mid.x.abs() < 1e-6 && mid.y.abs() < 1e-6);
    }

    #[test]
    fn group_bbox_unions_children() {
        let mut g = SceneGraph::new();
        let a = circle_at(&mut g, Point::new(-2.0, 0.0), 1.0);
        let b = circle_at(&mut g, Point::new(2.0, 0.0), 1.0);
        let grp = g.group_nodes(&[a, b]);
        let bb = g.bounding_box(grp);
        assert!((bb.x0 + 3.0).abs() < 1e-6, "x0={}", bb.x0);
        assert!((bb.x1 - 3.0).abs() < 1e-6, "x1={}", bb.x1);
    }

    #[test]
    fn shift_preserves_world_under_parent() {
        let mut g = SceneGraph::new();
        let parent = g.add(Mobject::group().shifted(Vec2::new(1.0, 0.0)));
        let child = g.add_child(parent, Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        g.shift(child, Vec2::new(0.0, 2.0));
        let p = g.center_of(child);
        assert!((p.x - 1.0).abs() < 1e-9 && (p.y - 2.0).abs() < 1e-9);
    }

    #[test]
    fn set_x_keeps_y() {
        let mut g = SceneGraph::new();
        let c = circle_at(&mut g, Point::new(0.0, 1.0), 0.5);
        g.set_x(c, 3.0);
        let p = g.center_of(c);
        assert!((p.x - 3.0).abs() < 1e-6 && (p.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn flip_up_mirrors_x() {
        let mut g = SceneGraph::new();
        let id = g.add(Mobject::new(geometry::line(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
        )));
        g.flip(id, UP);
        let t = g.world_transform(id);
        let a = t * Point::new(0.0, 0.0);
        let b = t * Point::new(1.0, 1.0);
        assert!((a.x - 1.0).abs() < 1e-6 && a.y.abs() < 1e-6, "{a:?}");
        assert!(b.x.abs() < 1e-6 && (b.y - 1.0).abs() < 1e-6, "{b:?}");
    }

    #[test]
    fn arrange_in_grid_is_2x2() {
        let mut g = SceneGraph::new();
        let ids: Vec<_> = (0..4)
            .map(|_| circle_at(&mut g, Point::ORIGIN, 0.4))
            .collect();
        let grp = g.group_nodes(&ids);
        g.arrange_in_grid(grp, Some(2), Some(2), 0.2, 0.2, true);
        let ys: Vec<f64> = ids.iter().map(|&id| g.center_of(id).y).collect();
        assert!((ys[0] - ys[1]).abs() < 1e-6, "row 0 should share y");
        assert!((ys[2] - ys[3]).abs() < 1e-6, "row 1 should share y");
        assert!(ys[0] > ys[2], "row 0 above row 1");
    }

    #[test]
    fn set_width_scales_circle() {
        let mut g = SceneGraph::new();
        let c = circle_at(&mut g, Point::ORIGIN, 1.0);
        g.set_width(c, 3.0);
        let bb = g.bounding_box(c);
        assert!((bb.width() - 3.0).abs() < 1e-6, "w={}", bb.width());
        assert!((bb.height() - 3.0).abs() < 1e-6, "h={}", bb.height());
    }
}
