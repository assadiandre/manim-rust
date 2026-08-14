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

    pub fn duration(&self) -> f64 {
        self.now
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
}
