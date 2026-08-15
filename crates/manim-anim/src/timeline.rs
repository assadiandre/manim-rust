//! The timeline: an ordered list of animations with absolute start times,
//! plus `Scene` — the authoring-facing wrapper implementing `play` semantics.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use manim_core::geometry;
use manim_core::peniko::Color;
use manim_core::{add_surrounding_rect, Style};
use manim_core::{Mobject, NodeId, OrthoCamera2D, SceneGraph};

use crate::animation::{Animation, Prop};
use crate::easing::Easing;

#[derive(Clone, Debug)]
pub struct CameraAnim {
    pub start: f64,
    pub duration: f64,
    pub easing: Easing,
    pub from: OrthoCamera2D,
    pub to: OrthoCamera2D,
}

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    pub animations: Vec<Animation>,
    pub duration: f64,
    pub camera_base: OrthoCamera2D,
    pub camera_anims: Vec<CameraAnim>,
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

    pub fn camera_at(&self, t: f64) -> OrthoCamera2D {
        if self.camera_anims.is_empty() {
            return self.camera_base.clone();
        }
        let chosen = self.camera_anims.iter().rev().find(|a| a.start <= t);
        match chosen {
            Some(a) => {
                let alpha = a.easing.eval((t - a.start) / a.duration.max(1e-9));
                lerp_camera(&a.from, &a.to, alpha)
            }
            None => self.camera_anims[0].from.clone(),
        }
    }
}

fn lerp_camera(from: &OrthoCamera2D, to: &OrthoCamera2D, t: f64) -> OrthoCamera2D {
    let t = t.clamp(0.0, 1.0);
    OrthoCamera2D {
        center: from.center.lerp(to.center, t),
        frame_height: from.frame_height + (to.frame_height - from.frame_height) * t,
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
    /// previous `play`. An animation's existing `start` is treated as an
    /// offset from that instant (used by Write / Circumscribe / LaggedStart).
    pub fn play(&mut self, animations: impl IntoIterator<Item = Animation>) {
        let mut span = 0.0_f64;
        for mut a in animations {
            let rel = a.start;
            a.start = self.now + rel;
            span = span.max(rel + a.duration);
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

    /// Reverse Write: last glyph uncreates first (Manim `Unwrite`).
    pub fn play_unwrite(&mut self, target: NodeId, duration: f64) {
        let mut targets = path_targets(&self.graph, target);
        targets.reverse();
        let n = targets.len();
        let lag = 0.1;
        let each = duration / (1.0 + (n.saturating_sub(1) as f64) * lag);
        let anims: Vec<_> = targets
            .iter()
            .map(|&id| Animation::uncreate(&self.graph, id, each))
            .collect();
        self.play_lagged(anims, lag);
    }

    /// Fade out then in (Manim `Blink`).
    pub fn play_blink(&mut self, target: NodeId, duration: f64) {
        let half = (duration * 0.5).max(1e-3);
        let mut fade_in = Animation::fade_in(&self.graph, target, half);
        fade_in.start = half;
        self.play([
            Animation::fade_out(&self.graph, target, half),
            fade_in,
        ]);
    }

    /// Ring shrinks onto a baked point (Manim `FocusOn`, no live follow).
    pub fn play_focus_on(&mut self, at: kurbo::Point, duration: f64, color: Color) {
        let style = Style::default().no_fill().with_stroke(color, 6.0);
        let id = self.add(Mobject::new(geometry::circle(at, 1.6)).with_style(style));
        self.play([
            Animation::scale(&self.graph, id, 0.08, duration),
            Animation::fade_out(&self.graph, id, duration),
        ]);
    }

    pub fn play_uncreate(&mut self, target: NodeId, duration: f64) {
        let anims: Vec<_> = path_targets(&self.graph, target)
            .into_iter()
            .map(|id| Animation::uncreate(&self.graph, id, duration))
            .collect();
        self.play(anims);
    }

    pub fn play_draw_border_then_fill(&mut self, target: NodeId, duration: f64) {
        let anims: Vec<_> = path_targets(&self.graph, target)
            .into_iter()
            .map(|id| Animation::draw_border_then_fill(&self.graph, id, duration))
            .collect();
        self.play(anims);
    }

    /// Create then uncreate a surrounding rectangle (Manim `Circumscribe`).
    pub fn play_circumscribe(&mut self, target: NodeId, duration: f64, color: Color) {
        let style = Style::default().no_fill().with_stroke(color, 4.0);
        let rect = add_surrounding_rect(&mut self.graph, target, 0.15, 0.0, style);
        let half = (duration * 0.5).max(1e-3);
        self.play_create(rect, half);
        self.play_uncreate(rect, half);
    }

    pub fn play_succession(&mut self, animations: impl IntoIterator<Item = Animation>) {
        for a in animations {
            self.play([a]);
        }
    }

    fn current_camera(&self) -> OrthoCamera2D {
        self.timeline
            .camera_anims
            .last()
            .map(|a| a.to.clone())
            .unwrap_or_else(|| self.timeline.camera_base.clone())
    }

    pub fn play_camera(&mut self, to: OrthoCamera2D, duration: f64) {
        let from = self.current_camera();
        self.timeline.camera_anims.push(CameraAnim {
            start: self.now,
            duration: duration.max(1e-9),
            easing: Easing::default(),
            from,
            to,
        });
        self.now += duration;
        self.timeline.duration = self.now;
    }

    pub fn play_camera_shift(&mut self, delta: kurbo::Vec2, duration: f64) {
        let mut to = self.current_camera();
        to.center = to.center + delta;
        self.play_camera(to, duration);
    }

    /// Zoom in (`factor` > 1 shrinks the frame).
    pub fn play_camera_zoom(&mut self, factor: f64, duration: f64) {
        let mut to = self.current_camera();
        to.frame_height /= factor.max(1e-6);
        self.play_camera(to, duration);
    }

    pub fn play_show_passing_flash(&mut self, target: NodeId, duration: f64) {
        let anims: Vec<_> = path_targets(&self.graph, target)
            .into_iter()
            .map(|id| Animation::show_passing_flash(&self.graph, id, duration))
            .collect();
        self.play(anims);
    }

    /// Radiating lines that flash past a point (Manim `Flash`).
    pub fn play_flash(&mut self, at: kurbo::Point, duration: f64, color: Color) {
        let n = 12;
        let radius = 0.4;
        let len = 0.22;
        let group = self.graph.add(Mobject::group().named("flash"));
        let style = Style::default().no_fill().with_stroke(color, 4.0);
        let mut anims = Vec::with_capacity(n);
        for i in 0..n {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            let dir = kurbo::Vec2::new(a.cos(), a.sin());
            let start = at + dir * (radius - len);
            let end = at + dir * radius;
            let id = self.graph.add_child(
                group,
                Mobject::new(manim_core::geometry::line(start, end)).with_style(style.clone()),
            );
            anims.push(Animation::show_passing_flash(&self.graph, id, duration));
        }
        self.play(anims);
    }

    pub fn play_move_along_path(&mut self, target: NodeId, path: NodeId, duration: f64) {
        self.play([Animation::move_along_path(
            &self.graph,
            target,
            path,
            duration,
        )]);
    }

    pub fn play_spin_in(&mut self, target: NodeId, duration: f64) {
        self.play([Animation::spin_in(&self.graph, target, duration)]);
    }

    pub fn play_shrink(&mut self, target: NodeId, duration: f64) {
        self.play([Animation::shrink_to_center(&self.graph, target, duration)]);
    }

    /// Manim `TransformMatchingShapes`: pair path leaves by normalized shape
    /// hash, shift matches, fade leftovers. Matched target leaves are hidden
    /// so they do not double-draw.
    pub fn transform_matching_anims(
        &mut self,
        source: NodeId,
        target: NodeId,
        duration: f64,
    ) -> Vec<Animation> {
        let src = path_targets(&self.graph, source);
        let dst = path_targets(&self.graph, target);
        let (pairs, extra_src, extra_dst) = match_leaves_by_shape(&self.graph, &src, &dst);
        let mut anims = Vec::new();
        for (s, d) in pairs {
            let delta = self.graph.center_of(d) - self.graph.center_of(s);
            // Same-shape matches only travel. Morphing through resampled
            // polylines turns identical letters into doubled strokes.
            anims.push(Animation::shift(&self.graph, s, delta, duration));
            self.graph.get_mut(d).style.opacity = 0.0;
        }
        for s in extra_src {
            anims.push(Animation::fade_out(&self.graph, s, duration));
        }
        for d in extra_dst {
            anims.push(Animation::fade_in(&self.graph, d, duration));
        }
        anims
    }

    pub fn play_transform_matching(&mut self, source: NodeId, target: NodeId, duration: f64) {
        let anims = self.transform_matching_anims(source, target, duration);
        self.play(anims);
    }

    /// Pair `tex-part:` children by substring (Manim `TransformMatchingTex`).
    /// Falls back to shape-hash matching when neither side has named parts.
    pub fn transform_matching_tex_anims(
        &mut self,
        source: NodeId,
        target: NodeId,
        duration: f64,
    ) -> Vec<Animation> {
        let src = tex_part_children(&self.graph, source);
        let dst = tex_part_children(&self.graph, target);
        if src.is_empty() || dst.is_empty() {
            return self.transform_matching_anims(source, target, duration);
        }
        match_named_parts(&mut self.graph, &src, &dst, duration)
    }

    pub fn play_transform_matching_tex(
        &mut self,
        source: NodeId,
        target: NodeId,
        duration: f64,
    ) {
        let anims = self.transform_matching_tex_anims(source, target, duration);
        self.play(anims);
    }

    /// Fade in children one after another (Manim `ShowIncreasingSubsets`).
    pub fn play_show_increasing_subsets(&mut self, target: NodeId, duration: f64) {
        let kids = subset_targets(&self.graph, target);
        let n = kids.len();
        let lag = 0.5;
        let each = duration / (1.0 + (n.saturating_sub(1) as f64) * lag);
        let anims: Vec<_> = kids
            .iter()
            .map(|&id| Animation::fade_in(&self.graph, id, each))
            .collect();
        self.play_lagged(anims, lag);
    }

    /// Each mobject travels to the next one's center; last wraps to first
    /// (Manim `CyclicReplace`).
    pub fn play_cyclic_replace(&mut self, ids: &[NodeId], duration: f64) {
        if ids.len() < 2 {
            return;
        }
        let centers: Vec<_> = ids.iter().map(|&id| self.graph.center_of(id)).collect();
        let anims: Vec<_> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let dest = centers[(i + 1) % centers.len()];
                Animation::shift(&self.graph, id, dest - centers[i], duration)
            })
            .collect();
        self.play(anims);
    }

    /// Source fades out while traveling toward the target; the target fades
    /// in while traveling from the source (Manim `FadeTransform`).
    ///
    /// Stretch uses a single `Travel` animation (shift+scale) so it does not
    /// fight `Prop::Transform`. The target is moved to the source center and
    /// scaled to the source extent first; `apply_final` restores dest size
    /// and position.
    pub fn fade_transform_anims(
        &mut self,
        source: NodeId,
        target: NodeId,
        duration: f64,
    ) -> Vec<Animation> {
        let src_c = self.graph.center_of(source);
        let dst_c = self.graph.center_of(target);
        let delta = dst_c - src_c;
        let src_e = node_extent(&self.graph, source);
        let dst_e = node_extent(&self.graph, target);
        let src_to_dst = dst_e / src_e;
        let dst_to_src = src_e / dst_e;

        let fade_out = Animation::fade_out(&self.graph, source, duration);
        let travel_src = Animation::travel(&self.graph, source, delta, src_to_dst, duration);

        self.graph.move_to(target, src_c);
        self.graph.scale_about_center(target, dst_to_src);

        let fade_in = Animation::fade_in(&self.graph, target, duration);
        let travel_dst = Animation::travel(&self.graph, target, delta, src_to_dst, duration);
        vec![fade_out, travel_src, fade_in, travel_dst]
    }

    pub fn play_fade_transform(&mut self, source: NodeId, target: NodeId, duration: f64) {
        let anims = self.fade_transform_anims(source, target, duration);
        self.play(anims);
    }

    /// Standing wave along the path (Manim `ApplyWave`).
    pub fn play_apply_wave(&mut self, target: NodeId, duration: f64) {
        self.play([Animation::apply_wave(&self.graph, target, 0.25, 2.0, duration)]);
    }

    /// Animate from the current state back to the last `save_state` snapshot.
    pub fn play_restore(&mut self, target: NodeId, duration: f64) {
        self.play([Animation::restore(&self.graph, target, duration)]);
    }

    pub fn duration(&self) -> f64 {
        self.now
    }
}

/// CE `TransformMatchingShapes.get_mobject_key`: same outline after
/// centering and fixing size, rounded so tiny flatten noise does not split
/// identical letters.
fn shape_key(path: &kurbo::BezPath) -> u64 {
    const N: usize = 24;
    let (pts, closed) = geometry::resample(path, N);
    if pts.is_empty() {
        return 0;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for p in &pts {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let cx = 0.5 * (min_x + max_x);
    let cy = 0.5 * (min_y + max_y);
    let extent = (max_y - min_y).max(max_x - min_x).max(1e-9);
    let scale = 1.0 / extent;
    let mut hasher = DefaultHasher::new();
    closed.hash(&mut hasher);
    for p in &pts {
        let x = ((p.x - cx) * scale * 32.0).round() as i32;
        let y = ((p.y - cy) * scale * 32.0).round() as i32;
        x.hash(&mut hasher);
        y.hash(&mut hasher);
    }
    hasher.finish()
}

fn match_leaves_by_shape(
    graph: &SceneGraph,
    src: &[NodeId],
    dst: &[NodeId],
) -> (Vec<(NodeId, NodeId)>, Vec<NodeId>, Vec<NodeId>) {
    let mut src_by: HashMap<u64, Vec<NodeId>> = HashMap::new();
    let mut dst_by: HashMap<u64, Vec<NodeId>> = HashMap::new();
    for &id in src {
        src_by
            .entry(shape_key(&graph.get(id).path))
            .or_default()
            .push(id);
    }
    for &id in dst {
        dst_by
            .entry(shape_key(&graph.get(id).path))
            .or_default()
            .push(id);
    }
    let mut pairs = Vec::new();
    let mut extra_src = Vec::new();
    let mut extra_dst = Vec::new();
    let mut keys: Vec<u64> = src_by.keys().copied().collect();
    for k in dst_by.keys() {
        if !src_by.contains_key(k) {
            keys.push(*k);
        }
    }
    for key in keys {
        let mut s = src_by.remove(&key).unwrap_or_default();
        let mut d = dst_by.remove(&key).unwrap_or_default();
        s.sort_by(|a, b| {
            graph
                .center_of(*a)
                .x
                .partial_cmp(&graph.center_of(*b).x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        d.sort_by(|a, b| {
            graph
                .center_of(*a)
                .x
                .partial_cmp(&graph.center_of(*b).x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = s.len().min(d.len());
        for i in 0..n {
            pairs.push((s[i], d[i]));
        }
        extra_src.extend(s.into_iter().skip(n));
        extra_dst.extend(d.into_iter().skip(n));
    }
    (pairs, extra_src, extra_dst)
}

fn tex_part_key(name: &str) -> Option<&str> {
    name.strip_prefix("tex-part:")
}

fn tex_part_children(graph: &SceneGraph, id: NodeId) -> Vec<(String, NodeId)> {
    let mut out = Vec::new();
    if let Some(name) = graph.get(id).name.as_deref() {
        if let Some(key) = tex_part_key(name) {
            out.push((key.to_string(), id));
            return out;
        }
    }
    for &c in graph.children_of(id) {
        if let Some(name) = graph.get(c).name.as_deref() {
            if let Some(key) = tex_part_key(name) {
                out.push((key.to_string(), c));
            }
        }
    }
    out
}

fn match_named_parts(
    graph: &mut SceneGraph,
    src: &[(String, NodeId)],
    dst: &[(String, NodeId)],
    duration: f64,
) -> Vec<Animation> {
    let mut src_by: HashMap<String, Vec<NodeId>> = HashMap::new();
    let mut dst_by: HashMap<String, Vec<NodeId>> = HashMap::new();
    for (k, id) in src {
        src_by.entry(k.clone()).or_default().push(*id);
    }
    for (k, id) in dst {
        dst_by.entry(k.clone()).or_default().push(*id);
    }
    let mut pairs = Vec::new();
    let mut extra_src = Vec::new();
    let mut extra_dst = Vec::new();
    let mut keys: Vec<String> = src_by.keys().cloned().collect();
    for k in dst_by.keys() {
        if !src_by.contains_key(k) {
            keys.push(k.clone());
        }
    }
    for key in keys {
        let mut s = src_by.remove(&key).unwrap_or_default();
        let mut d = dst_by.remove(&key).unwrap_or_default();
        s.sort_by(|a, b| {
            graph
                .center_of(*a)
                .x
                .partial_cmp(&graph.center_of(*b).x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        d.sort_by(|a, b| {
            graph
                .center_of(*a)
                .x
                .partial_cmp(&graph.center_of(*b).x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = s.len().min(d.len());
        for i in 0..n {
            pairs.push((s[i], d[i]));
        }
        extra_src.extend(s.into_iter().skip(n));
        extra_dst.extend(d.into_iter().skip(n));
    }
    let mut anims = Vec::new();
    for (s, d) in pairs {
        let delta = graph.center_of(d) - graph.center_of(s);
        anims.push(Animation::shift(graph, s, delta, duration));
        graph.get_mut(d).style.opacity = 0.0;
    }
    for s in extra_src {
        anims.push(Animation::fade_out(graph, s, duration));
    }
    for d in extra_dst {
        anims.push(Animation::fade_in(graph, d, duration));
    }
    anims
}

fn node_extent(graph: &SceneGraph, id: NodeId) -> f64 {
    let b = graph.bounding_box(id);
    b.width().max(b.height()).max(1e-6)
}

fn subset_targets(graph: &SceneGraph, target: NodeId) -> Vec<NodeId> {
    let kids = graph.children_of(target);
    if kids.is_empty() {
        path_targets(graph, target)
    } else {
        kids.to_vec()
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
        scene
            .play([Animation::shift(&scene.graph, c, Vec2::new(1.0, 0.0), 1.0)
                .with_easing(Easing::Linear)]);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.25);
        let p = sim.get(c).transform * Point::ORIGIN;
        assert!((p.x - 0.25).abs() < 1e-9);
    }

    #[test]
    fn sequential_same_property_animations_compose() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene
            .play([Animation::shift(&scene.graph, c, Vec2::new(1.0, 0.0), 1.0)
                .with_easing(Easing::Linear)]);
        scene
            .play([Animation::shift(&scene.graph, c, Vec2::new(1.0, 0.0), 1.0)
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
        scene.play([Animation::grow_from_center(&scene.graph, c, 1.0).with_easing(Easing::Linear)]);
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
    fn play_honors_relative_start_offsets() {
        let mut scene = Scene::new();
        let a = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.5)));
        let b = scene.add(Mobject::new(geometry::circle(Point::new(2.0, 0.0), 0.5)));
        let mut late = Animation::fade_in(&scene.graph, b, 1.0).with_easing(Easing::Linear);
        late.start = 0.5;
        scene.play([
            Animation::fade_in(&scene.graph, a, 1.0).with_easing(Easing::Linear),
            late,
        ]);
        assert!((scene.duration() - 1.5).abs() < 1e-9);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.25);
        assert!((sim.get(a).style.opacity - 0.25).abs() < 1e-6);
        assert_eq!(sim.get(b).style.opacity, 0.0);
    }

    #[test]
    fn play_unwrite_reverses_leaf_order() {
        let mut scene = Scene::new();
        let a = scene.add(Mobject::new(geometry::circle(Point::new(-1.0, 0.0), 0.4)));
        let b = scene.add(Mobject::new(geometry::circle(Point::new(1.0, 0.0), 0.4)));
        let g = scene.graph.group_nodes(&[a, b]);
        scene.play_unwrite(g, 1.0);
        assert!(scene.timeline.animations.len() >= 2);
        let first = &scene.timeline.animations[0];
        assert_eq!(first.target, b, "unwrite should start with the last leaf");
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

    #[test]
    fn recolor_lerps_fill() {
        let mut scene = Scene::new();
        let c = scene.add(
            Mobject::new(geometry::circle(Point::ORIGIN, 1.0))
                .with_style(manim_core::Style::filled(manim_core::palette::red())),
        );
        scene.play([
            Animation::recolor(&scene.graph, c, manim_core::palette::blue(), 1.0)
                .with_easing(Easing::Linear),
        ]);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let a = sim.get(c).style.fill.unwrap().to_rgba8();
        assert_eq!(a.r, 252);
        scene.timeline.apply(&mut sim, 1.0);
        let b = sim.get(c).style.fill.unwrap().to_rgba8();
        assert_eq!(b.r, 88);
    }

    #[test]
    fn wiggle_returns_home() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.play([Animation::wiggle(&scene.graph, c, 1.0)]);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 1.0);
        let p = sim.get(c).transform * Point::new(1.0, 0.0);
        assert!((p.x - 1.0).abs() < 1e-6 && p.y.abs() < 1e-6);
    }

    #[test]
    fn camera_zoom_shrinks_frame() {
        let mut scene = Scene::new();
        scene.play_camera_zoom(2.0, 1.0);
        let mid = scene.timeline.camera_at(0.5);
        let end = scene.timeline.camera_at(1.0);
        assert!(mid.frame_height < 8.0 && mid.frame_height > 4.0);
        assert!((end.frame_height - 4.0).abs() < 1e-6);
    }

    #[test]
    fn draw_border_then_fill_hides_fill_first() {
        let mut scene = Scene::new();
        let c = scene.add(
            Mobject::new(geometry::circle(Point::ORIGIN, 1.0))
                .with_style(manim_core::Style::filled(manim_core::palette::red())),
        );
        scene.play_draw_border_then_fill(c, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.3);
        assert!(sim.get(c).style.fill.is_none());
        scene.timeline.apply(&mut sim, 1.0);
        assert!(sim.get(c).style.fill.is_some());
    }

    #[test]
    fn move_along_path_ends_at_path_end() {
        let mut scene = Scene::new();
        let path = scene.add(Mobject::new(geometry::line(
            Point::new(-2.0, 0.0),
            Point::new(2.0, 0.0),
        )));
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 0.3)));
        scene.play_move_along_path(c, path, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let start = sim.center_of(c);
        assert!((start.x + 2.0).abs() < 0.05, "x={}", start.x);
        scene.timeline.apply(&mut sim, 1.0);
        let end = sim.center_of(c);
        assert!((end.x - 2.0).abs() < 0.05, "x={}", end.x);
    }

    #[test]
    fn passing_flash_empty_at_ends() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.play_show_passing_flash(c, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        assert!(geometry::path_length(&sim.get(c).path) < 1e-6);
        scene.timeline.apply(&mut sim, 0.5);
        assert!(geometry::path_length(&sim.get(c).path) > 0.1);
        scene.timeline.apply(&mut sim, 1.0);
        assert!(geometry::path_length(&sim.get(c).path) < 1e-6);
    }

    #[test]
    fn spin_in_starts_collapsed() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        scene.play_spin_in(c, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let p = sim.get(c).transform * Point::new(1.0, 0.0);
        assert!(p.x.abs() < 1e-6 && p.y.abs() < 1e-6);
        scene.timeline.apply(&mut sim, 1.0);
        let p = sim.get(c).transform * Point::new(1.0, 0.0);
        assert!((p.x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fade_transform_hides_source_shows_target() {
        let mut scene = Scene::new();
        let source = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.5)));
        let target = scene.add(Mobject::new(geometry::circle(Point::new(2.0, 0.0), 0.5)));
        scene.play_fade_transform(source, target, 1.0);

        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        assert!((sim.get(source).style.opacity - 1.0).abs() < 1e-6);
        assert_eq!(sim.get(target).style.opacity, 0.0);

        scene.timeline.apply(&mut sim, 1.0);
        assert_eq!(sim.get(source).style.opacity, 0.0);
        assert!((sim.get(target).style.opacity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fade_transform_target_travels() {
        let mut scene = Scene::new();
        let source = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.5)));
        let target = scene.add(Mobject::new(geometry::circle(Point::new(2.0, 0.0), 0.5)));
        let src_start = scene.graph.center_of(source);
        let dst_end = scene.graph.center_of(target);
        scene.play_fade_transform(source, target, 1.0);

        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let t0 = sim.center_of(target);
        assert!(
            (t0.x - src_start.x).abs() < 0.05 && (t0.y - src_start.y).abs() < 0.05,
            "t0={t0:?}"
        );

        scene.timeline.apply(&mut sim, 1.0);
        let t1 = sim.center_of(target);
        assert!(
            (t1.x - dst_end.x).abs() < 0.05 && (t1.y - dst_end.y).abs() < 0.05,
            "t1={t1:?}"
        );
    }

    #[test]
    fn changing_decimal_widens_as_value_grows() {
        let mut atlas = manim_core::DigitAtlas::default();
        for ch in ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', '-'] {
            atlas.insert(ch, geometry::rect(Point::new(0.2, 0.3), 0.4, 0.6), 0.45);
        }
        let mut scene = Scene::new();
        let id = scene.add(Mobject::new(atlas.compose(1.0, 0)));
        scene.play([
            Animation::changing_decimal(id, 1.0, 12.0, 0, atlas, 1.0).with_easing(Easing::Linear),
        ]);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let w0 = geometry::path_length(&sim.get(id).path);
        scene.timeline.apply(&mut sim, 1.0);
        let w1 = geometry::path_length(&sim.get(id).path);
        assert!(w1 > w0, "start={w0} end={w1}");
    }

    #[test]
    fn transform_matching_moves_source_to_target_center() {
        let mut scene = Scene::new();
        let src = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.4)));
        let dst = scene.add(Mobject::new(geometry::circle(Point::new(2.0, 0.0), 0.4)));
        let dest_c = scene.graph.center_of(dst);
        scene.play_transform_matching(src, dst, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 1.0);
        let p = sim.center_of(src);
        assert!(
            (p.x - dest_c.x).abs() < 0.1 && (p.y - dest_c.y).abs() < 0.1,
            "{p:?}"
        );
        assert_eq!(sim.get(dst).style.opacity, 0.0);
    }

    #[test]
    fn circle_and_square_have_different_shape_keys() {
        let c = geometry::circle(Point::ORIGIN, 0.4);
        let s = geometry::square(Point::ORIGIN, 0.8);
        assert_ne!(shape_key(&c), shape_key(&s));
        assert_eq!(
            shape_key(&geometry::circle(Point::new(-2.0, 0.0), 0.4)),
            shape_key(&geometry::circle(Point::new(2.0, 0.0), 0.4)),
        );
    }

    #[test]
    fn transform_matching_fades_unmatched_shapes() {
        let mut scene = Scene::new();
        let src = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.4)));
        let dst = scene.add(Mobject::new(geometry::square(Point::new(2.0, 0.0), 0.8)));
        scene.play_transform_matching(src, dst, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 1.0);
        assert!(
            sim.get(src).style.opacity < 0.01,
            "src opacity {}",
            sim.get(src).style.opacity
        );
        assert!(
            sim.get(dst).style.opacity > 0.99,
            "dst opacity {}",
            sim.get(dst).style.opacity
        );
    }

    #[test]
    fn transform_matching_pairs_same_shape_not_nearest() {
        let mut scene = Scene::new();
        let src = scene.add(Mobject::new(geometry::circle(Point::new(-1.0, 0.0), 0.4)));
        let dst_g = scene.add(Mobject::group());
        scene.add_child(
            dst_g,
            Mobject::new(geometry::square(Point::new(0.0, 0.0), 0.6)),
        );
        let far_circle = scene.add_child(
            dst_g,
            Mobject::new(geometry::circle(Point::new(3.0, 0.0), 0.4)),
        );
        let dest_c = scene.graph.center_of(far_circle);
        scene.play_transform_matching(src, dst_g, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 1.0);
        let p = sim.center_of(src);
        assert!(
            (p.x - dest_c.x).abs() < 0.15 && (p.y - dest_c.y).abs() < 0.15,
            "{p:?} dest={dest_c:?}"
        );
        assert_eq!(sim.get(far_circle).style.opacity, 0.0);
    }

    #[test]
    fn transform_matching_tex_pairs_by_name() {
        let mut scene = Scene::new();
        let src_a = scene.add(
            Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.3)).named("tex-part:a"),
        );
        let src_b = scene.add(
            Mobject::new(geometry::square(Point::new(-1.0, 0.0), 0.4)).named("tex-part:b"),
        );
        let src = scene.graph.group_nodes(&[src_a, src_b]);
        let dst_a = scene.add(
            Mobject::new(geometry::circle(Point::new(2.0, 1.0), 0.3)).named("tex-part:a"),
        );
        let dst_b = scene.add(
            Mobject::new(geometry::square(Point::new(1.0, -1.0), 0.4)).named("tex-part:b"),
        );
        let dst = scene.graph.group_nodes(&[dst_a, dst_b]);
        let dest_a = scene.graph.center_of(dst_a);
        scene.play_transform_matching_tex(src, dst, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 1.0);
        let p = sim.center_of(src_a);
        assert!(
            (p.x - dest_a.x).abs() < 0.1 && (p.y - dest_a.y).abs() < 0.1,
            "{p:?} dest={dest_a:?}"
        );
        assert_eq!(sim.get(dst_a).style.opacity, 0.0);
    }

    #[test]
    fn show_increasing_subsets_staggers_children() {
        let mut scene = Scene::new();
        let a = scene.add(Mobject::new(geometry::circle(Point::new(-1.0, 0.0), 0.3)));
        let b = scene.add(Mobject::new(geometry::circle(Point::new(1.0, 0.0), 0.3)));
        let g = scene.graph.group_nodes(&[a, b]);
        scene.play_show_increasing_subsets(g, 1.5);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.25);
        assert!(sim.get(a).style.opacity > 0.0);
        assert_eq!(sim.get(b).style.opacity, 0.0);
    }

    #[test]
    fn cyclic_replace_rotates_centers() {
        let mut scene = Scene::new();
        let a = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.3)));
        let b = scene.add(Mobject::new(geometry::circle(Point::new(2.0, 0.0), 0.3)));
        let a0 = scene.graph.center_of(a);
        let b0 = scene.graph.center_of(b);
        scene.play_cyclic_replace(&[a, b], 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 1.0);
        let a1 = sim.center_of(a);
        let b1 = sim.center_of(b);
        assert!((a1.x - b0.x).abs() < 0.05 && (a1.y - b0.y).abs() < 0.05);
        assert!((b1.x - a0.x).abs() < 0.05 && (b1.y - a0.y).abs() < 0.05);
    }

    #[test]
    fn fade_transform_stretches_small_toward_large() {
        let mut scene = Scene::new();
        let source = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.3)));
        let target = scene.add(Mobject::new(geometry::circle(Point::new(2.0, 0.0), 1.0)));
        let w0 = scene.graph.bounding_box(source).width();
        let w_dst = scene.graph.bounding_box(target).width();
        let src_c = scene.graph.center_of(source);
        let dst_c = scene.graph.center_of(target);
        scene.play_fade_transform(source, target, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let start = sim.bounding_box(source).width();
        scene.timeline.apply(&mut sim, 0.5);
        let mid = sim.bounding_box(source).width();
        scene.timeline.apply(&mut sim, 1.0);
        let end = sim.bounding_box(source).width();
        let t1 = sim.center_of(target);
        assert!((start - w0).abs() < 0.08, "start={start} w0={w0}");
        assert!(mid > start + 0.15, "mid={mid} start={start}");
        assert!((end - w_dst).abs() < 0.15, "end={end} dst={w_dst}");
        assert!(
            (t1.x - dst_c.x).abs() < 0.08 && (t1.y - dst_c.y).abs() < 0.08,
            "target end {t1:?} dest {dst_c:?} (src was {src_c:?})"
        );
    }

    #[test]
    fn fade_transform_stretch_keeps_centers_together() {
        let mut scene = Scene::new();
        let source = scene.add(Mobject::new(geometry::circle(Point::new(-2.0, 0.0), 0.3)));
        let target = scene.add(Mobject::new(geometry::square(Point::new(2.0, 0.0), 2.0)));
        scene.play_fade_transform(source, target, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.5);
        let a = sim.center_of(source);
        let b = sim.center_of(target);
        assert!(
            (a.x - b.x).abs() < 0.12 && (a.y - b.y).abs() < 0.12,
            "mid source={a:?} target={b:?}"
        );
    }

    #[test]
    fn apply_wave_rests_at_endpoints() {
        let mut scene = Scene::new();
        let line = scene.add(Mobject::new(geometry::line(
            Point::new(-2.0, 0.0),
            Point::new(2.0, 0.0),
        )));
        scene.play_apply_wave(line, 1.0);
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let h0 = sim.bounding_box(line).height();
        scene.timeline.apply(&mut sim, 0.5);
        let h1 = sim.bounding_box(line).height();
        scene.timeline.apply(&mut sim, 1.0);
        let h2 = sim.bounding_box(line).height();
        assert!(h0 < 0.05, "start height {h0}");
        assert!(h1 > 0.2, "mid height {h1}");
        assert!(h2 < 0.05, "end height {h2}");
    }

    #[test]
    fn play_restore_interpolates_mid_opacity_and_center() {
        let mut scene = Scene::new();
        let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
        let saved_c = scene.graph.center_of(c);
        let saved_o = scene.graph.get(c).style.opacity;
        scene.graph.save_state(c);
        scene.graph.shift(c, Vec2::new(2.0, 0.0));
        scene.graph.set_opacity(c, 0.0);
        let cur_c = scene.graph.center_of(c);
        let cur_o = scene.graph.get(c).style.opacity;
        scene.play_restore(c, 1.0);

        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, 0.0);
        let p0 = sim.center_of(c);
        assert!((p0.x - cur_c.x).abs() < 0.05 && (p0.y - cur_c.y).abs() < 0.05);
        assert!((sim.get(c).style.opacity - cur_o).abs() < 1e-6);

        scene.timeline.apply(&mut sim, 0.5);
        let mid_c = sim.center_of(c);
        let mid_o = sim.get(c).style.opacity;
        let lo_x = saved_c.x.min(cur_c.x);
        let hi_x = saved_c.x.max(cur_c.x);
        assert!(
            mid_c.x > lo_x + 0.2 && mid_c.x < hi_x - 0.2,
            "mid center {mid_c:?} between {saved_c:?} and {cur_c:?}"
        );
        let lo_o = saved_o.min(cur_o);
        let hi_o = saved_o.max(cur_o);
        assert!(
            mid_o > lo_o + 0.2 && mid_o < hi_o - 0.2,
            "mid opacity {mid_o} between {saved_o} and {cur_o}"
        );

        scene.timeline.apply(&mut sim, 1.0);
        let p1 = sim.center_of(c);
        assert!((p1.x - saved_c.x).abs() < 0.05 && (p1.y - saved_c.y).abs() < 0.05);
        assert!((sim.get(c).style.opacity - saved_o).abs() < 1e-6);
    }
}
