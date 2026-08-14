//! The timeline: an ordered list of animations with absolute start times,
//! plus `Scene` — the authoring-facing wrapper implementing `play` semantics.

use std::collections::HashMap;

use manim_core::{Mobject, NodeId, SceneGraph};

use crate::animation::{Animation, Prop};

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    pub animations: Vec<Animation>,
    pub duration: f64,
}

impl Timeline {
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// Stateless evaluation against the *final* scene graph.
    ///
    /// For each (target, property) group: apply the last animation that has
    /// started at its eased alpha; if none has started, apply the group's
    /// first animation at alpha 0 (its `from` snapshot = the property's
    /// pre-animation state). This makes evaluation correct for any t,
    /// including before/after the whole timeline, with no scene snapshotting.
    pub fn apply(&self, scene: &mut SceneGraph, t: f64) {
        // Group indices by (target, prop), preserving push order.
        let mut groups: HashMap<(NodeId, Prop), Vec<usize>> = HashMap::new();
        for (i, a) in self.animations.iter().enumerate() {
            groups.entry((a.target, a.kind.prop())).or_default().push(i);
        }
        for indices in groups.values() {
            // Starts are non-decreasing in push order, so the last started
            // animation is found by reverse scan.
            let chosen = indices
                .iter()
                .rev()
                .find(|&&i| self.animations[i].start <= t);
            match chosen {
                Some(&i) => {
                    let a = &self.animations[i];
                    a.apply_at_alpha(scene, a.alpha_at(t));
                }
                None => {
                    let a = &self.animations[indices[0]];
                    a.apply_at_alpha(scene, 0.0);
                }
            }
        }
    }
}

/// Authoring API: a scene graph plus the timeline being built.
///
/// `play()` eagerly applies each animation's end state to `graph`, so
/// post-play code and later animation snapshots see final values (Manim
/// semantics). Rendering evaluates the timeline statelessly against `graph`.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub graph: SceneGraph,
    pub timeline: Timeline,
    now: f64,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, mobject: Mobject) -> NodeId {
        self.graph.add(mobject)
    }

    pub fn add_child(&mut self, parent: NodeId, mobject: Mobject) -> NodeId {
        self.graph.add_child(parent, mobject)
    }

    /// Play animations simultaneously; each starts at the end of the
    /// previous `play`.
    pub fn play(&mut self, animations: impl IntoIterator<Item = Animation>) {
        let mut span = 0.0_f64;
        for mut a in animations {
            a.start += self.now;
            span = span.max(a.duration);
            a.apply_final(&mut self.graph);
            self.timeline.animations.push(a);
        }
        self.now += span;
        self.timeline.duration = self.now;
    }

    /// Advance time with no animation (Manim's `wait`).
    pub fn wait(&mut self, duration: f64) {
        self.now += duration;
        self.timeline.duration = self.now;
    }

    /// Manim `LaggedStart`: each animation begins `lag_ratio * prev.duration`
    /// after the previous one. The play span is the last animation's end.
    pub fn play_lagged(&mut self, animations: impl IntoIterator<Item = Animation>, lag_ratio: f64) {
        let mut cursor = self.now;
        let mut end = self.now;
        for mut a in animations {
            a.start += cursor;
            end = end.max(a.end());
            a.apply_final(&mut self.graph);
            cursor += a.duration * lag_ratio;
            self.timeline.animations.push(a);
        }
        self.now = end;
        self.timeline.duration = self.now;
    }

    /// Create every path-bearing leaf under `target` (Manim `Create` on a VGroup).
    pub fn play_create(&mut self, target: NodeId, duration: f64) {
        let anims: Vec<_> = path_targets(&self.graph, target)
            .into_iter()
            .map(|id| Animation::create(&self.graph, id, duration))
            .collect();
        self.play(anims);
    }

    /// Lagged Create of path leaves so a formula writes on glyph-by-glyph.
    pub fn play_write(&mut self, target: NodeId, duration: f64) {
        let targets = path_targets(&self.graph, target);
        let n = targets.len();
        let lag = 0.1;
        let each = duration / (1.0 + (n.saturating_sub(1) as f64) * lag);
        let anims: Vec<_> = targets
            .iter()
            .map(|&id| Animation::create(&self.graph, id, each))
            .collect();
        self.play_lagged(anims, lag);
    }

    pub fn play_uncreate(&mut self, target: NodeId, duration: f64) {
        let anims: Vec<_> = path_targets(&self.graph, target)
            .into_iter()
            .map(|id| Animation::uncreate(&self.graph, id, duration))
            .collect();
        self.play(anims);
    }

    pub fn duration(&self) -> f64 {
        self.now
    }
}

/// Leaves to animate for Create/Write/Uncreate. Falls back to `target` itself
/// when the node has no path-bearing descendants (empty group, or a leaf).
pub fn path_targets(graph: &SceneGraph, target: NodeId) -> Vec<NodeId> {
    let leaves = graph.path_leaves(target);
    if leaves.is_empty() {
        vec![target]
    } else {
        leaves
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::Easing;
    use kurbo::{Point, Vec2};
    use manim_core::geometry;

    #[test]
    fn play_sequences_and_evaluates_statelessly() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        let original_len = geometry::path_length(&scene.graph.get(c).path);

        scene.play([Animation::create(&scene.graph, c, 1.0)]);
        scene.play([Animation::shift(&scene.graph, c, Vec2::new(2.0, 0.0), 1.0)]);
        assert!((scene.duration() - 2.0).abs() < 1e-9);

        // Mid-Create: path is partially drawn, not yet shifted.
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.5);
        let len = geometry::path_length(&sim.get(c).path);
        assert!(len < original_len * 0.6, "len={len}");
        let p = sim.get(c).transform * Point::ORIGIN;
        assert!(p.x.abs() < 1e-9);

        // After both: full path, shifted by 2.
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 2.0);
        let len = geometry::path_length(&sim.get(c).path);
        assert!((len - original_len).abs() / original_len < 0.05);
        let p = sim.get(c).transform * Point::ORIGIN;
        assert!((p.x - 2.0).abs() < 1e-9);
    }

    #[test]
    fn fade_in_is_invisible_before_start() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.wait(0.5);
        scene.play([Animation::fade_in(&scene.graph, c, 1.0)]);

        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        assert_eq!(sim.get(c).style.opacity, 0.0);
        scene.timeline.apply(&mut sim, 1.0);
        assert!((sim.get(c).style.opacity - 0.5).abs() < 1e-6);
    }

    #[test]
    fn shift_before_start_shows_initial_position() {
        // Regression: non-intro animations must not leak their end state
        // into frames before they start.
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.wait(1.0);
        scene.play([Animation::shift(&scene.graph, c, Vec2::new(3.0, 0.0), 1.0)]);

        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.5);
        let p = sim.get(c).transform * Point::ORIGIN;
        assert!(p.x.abs() < 1e-9, "x={}", p.x);
    }

    #[test]
    fn easing_is_applied() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.play([
            Animation::shift(&scene.graph, c, Vec2::new(1.0, 0.0), 1.0)
                .with_easing(Easing::Linear),
        ]);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.25);
        let p = sim.get(c).transform * Point::ORIGIN;
        assert!((p.x - 0.25).abs() < 1e-9);
    }

    #[test]
    fn sequential_same_property_animations_compose() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.play([Animation::shift(&scene.graph, c, Vec2::new(1.0, 0.0), 1.0)
            .with_easing(Easing::Linear)]);
        scene.play([Animation::shift(&scene.graph, c, Vec2::new(1.0, 0.0), 1.0)
            .with_easing(Easing::Linear)]);

        // Midway through the second shift: x should be 1.5, not reset by
        // the first animation.
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 1.5);
        let p = sim.get(c).transform * Point::ORIGIN;
        assert!((p.x - 1.5).abs() < 1e-9, "x={}", p.x);
    }

    #[test]
    fn uncreate_shrinks_the_path() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        let full = geometry::path_length(&scene.graph.get(c).path);
        scene.play([Animation::uncreate(&scene.graph, c, 1.0).with_easing(Easing::Linear)]);

        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        assert!((geometry::path_length(&sim.get(c).path) - full).abs() / full < 0.05);

        scene.timeline.apply(&mut sim, 0.5);
        let mid = geometry::path_length(&sim.get(c).path);
        assert!((mid - full * 0.5).abs() / full < 0.05, "mid={mid}");

        scene.timeline.apply(&mut sim, 1.0);
        assert!(geometry::path_length(&sim.get(c).path) < 1e-6);
    }

    #[test]
    fn rotate_quarter_turn() {
        let mut scene = Scene::new();
        let s = scene.add(Mobject::new(geometry::square(Point::ORIGIN, 2.0)));
        scene.play([
            Animation::rotate(&scene.graph, s, std::f64::consts::FRAC_PI_2, 1.0)
                .with_easing(Easing::Linear),
        ]);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 1.0);
        // Local (1, 0) on the square's right side rotates to (0, 1).
        let p = sim.get(s).transform * Point::new(1.0, 0.0);
        assert!(p.x.abs() < 1e-9 && (p.y - 1.0).abs() < 1e-9, "{p:?}");
    }

    #[test]
    fn grow_starts_collapsed() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.play([
            Animation::grow_from_center(&scene.graph, c, 1.0).with_easing(Easing::Linear),
        ]);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let p = sim.get(c).transform * Point::new(1.0, 0.0);
        assert!(p.x.abs() < 1e-9 && p.y.abs() < 1e-9);
        scene.timeline.apply(&mut sim, 1.0);
        let p = sim.get(c).transform * Point::new(1.0, 0.0);
        assert!((p.x - 1.0).abs() < 1e-9);
    }

    #[test]
    fn indicate_returns_to_original_scale() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.play([Animation::indicate(&scene.graph, c, 1.0)]);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.5);
        let mid = sim.get(c).transform * Point::new(1.0, 0.0);
        assert!(mid.x > 1.05, "should be enlarged at midpoint, x={}", mid.x);
        scene.timeline.apply(&mut sim, 1.0);
        let end = sim.get(c).transform * Point::new(1.0, 0.0);
        assert!((end.x - 1.0).abs() < 1e-6, "x={}", end.x);
    }

    #[test]
    fn play_lagged_staggers_starts() {
        let mut scene = Scene::new();
        let a = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.5)));
        let b = scene.add(Mobject::new(geometry::circle(Point::new(2.0, 0.0), 0.5)));
        scene.play_lagged(
            [
                Animation::fade_in(&scene.graph, a, 1.0).with_easing(Easing::Linear),
                Animation::fade_in(&scene.graph, b, 1.0).with_easing(Easing::Linear),
            ],
            0.5,
        );
        assert!((scene.duration() - 1.5).abs() < 1e-9);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.25);
        assert!((sim.get(a).style.opacity - 0.25).abs() < 1e-6);
        assert_eq!(sim.get(b).style.opacity, 0.0);
    }
}
