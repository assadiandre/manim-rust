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
        let target = Point::new(
            direction.x.signum() * FRAME_X_RADIUS,
            direction.y.signum() * FRAME_Y_RADIUS,
        );
        let mine = self.critical_point(id, direction);
        let mut delta = (target - mine) - direction * buff;
        delta.x *= direction.x.signum().abs();
        delta.y *= direction.y.signum().abs();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DOWN, LEFT, RIGHT, UP};
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
}
