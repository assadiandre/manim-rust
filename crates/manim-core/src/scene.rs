//! Retained, dirty-tracked scene graph (arena of nodes).
//!
//! The renderer's frame-reuse fast path depends on one rule: **all mutation
//! goes through `get_mut` / `mark_dirty`**. Reading is free.

use kurbo::Affine;

use crate::mobject::Mobject;

pub type NodeId = usize;

#[derive(Clone, Debug)]
struct Node {
    mobject: Mobject,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    dirty: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SceneGraph {
    nodes: Vec<Node>,
    roots: Vec<NodeId>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a root mobject. New nodes start dirty.
    pub fn add(&mut self, mobject: Mobject) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node {
            mobject,
            parent: None,
            children: Vec::new(),
            dirty: true,
        });
        self.roots.push(id);
        id
    }

    pub fn add_child(&mut self, parent: NodeId, mobject: Mobject) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node {
            mobject,
            parent: Some(parent),
            children: Vec::new(),
            dirty: true,
        });
        self.nodes[parent].children.push(id);
        self.mark_dirty(parent);
        id
    }

    pub fn get(&self, id: NodeId) -> &Mobject {
        &self.nodes[id].mobject
    }

    /// Mutable access marks the node (and ancestors, whose cached subtree
    /// would contain it) dirty.
    pub fn get_mut(&mut self, id: NodeId) -> &mut Mobject {
        self.mark_dirty(id);
        &mut self.nodes[id].mobject
    }

    pub fn mark_dirty(&mut self, id: NodeId) {
        let mut cur = Some(id);
        while let Some(i) = cur {
            let node = &mut self.nodes[i];
            if node.dirty {
                // Ancestors must already be dirty too.
                break;
            }
            node.dirty = true;
            cur = node.parent;
        }
    }

    pub fn any_dirty(&self) -> bool {
        self.nodes.iter().any(|n| n.dirty)
    }

    /// Called by the renderer after a frame has been produced.
    pub fn clear_dirty(&mut self) {
        for n in &mut self.nodes {
            n.dirty = false;
        }
    }

    pub fn remove(&mut self, id: NodeId) {
        if let Some(parent) = self.nodes[id].parent {
            self.nodes[parent].children.retain(|&c| c != id);
            self.mark_dirty(parent);
        }
        self.roots.retain(|&r| r != id);
        // Tombstone: hide + detach children. Keeps NodeIds stable.
        self.nodes[id].mobject.visible = false;
        self.nodes[id].mobject.path = kurbo::BezPath::new();
        let children = std::mem::take(&mut self.nodes[id].children);
        for c in children {
            self.remove(c);
        }
        self.nodes[id].dirty = true;
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn children_of(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id].children
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id].parent
    }

    /// Reparent `id` under `new_parent` (or to a root if `None`), preserving
    /// the current world transform so the object does not jump.
    pub fn reparent(&mut self, id: NodeId, new_parent: Option<NodeId>) {
        let world = self.world_transform(id);
        if let Some(old) = self.nodes[id].parent {
            self.nodes[old].children.retain(|&c| c != id);
            self.mark_dirty(old);
        } else {
            self.roots.retain(|&r| r != id);
        }
        self.nodes[id].parent = new_parent;
        if let Some(p) = new_parent {
            self.nodes[p].children.push(id);
            let parent_world = self.world_transform(p);
            self.nodes[id].mobject.transform = parent_world.inverse() * world;
            self.mark_dirty(p);
        } else {
            self.nodes[id].mobject.transform = world;
            self.roots.push(id);
            self.mark_dirty(id);
        }
    }

    /// Wrap `ids` in a new group, preserving each child's world transform.
    pub fn group_nodes(&mut self, ids: &[NodeId]) -> NodeId {
        let g = self.add(Mobject::group());
        for &id in ids {
            if id != g {
                self.reparent(id, Some(g));
            }
        }
        g
    }

    /// Path-bearing leaves under `id` (the node itself if it has a path and
    /// no children). Used by Create/Write on groups such as Typst formulas.
    pub fn path_leaves(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.collect_path_leaves(id, &mut out);
        out
    }

    fn collect_path_leaves(&self, id: NodeId, out: &mut Vec<NodeId>) {
        let children = &self.nodes[id].children;
        if children.is_empty() {
            if !self.nodes[id].mobject.path.elements().is_empty() {
                out.push(id);
            }
            return;
        }
        for &c in children {
            self.collect_path_leaves(c, out);
        }
    }

    /// local-to-world transform, walking up the parent chain.
    pub fn world_transform(&self, id: NodeId) -> Affine {
        let mut t = self.nodes[id].mobject.transform;
        let mut cur = self.nodes[id].parent;
        while let Some(p) = cur {
            t = self.nodes[p].mobject.transform * t;
            cur = self.nodes[p].parent;
        }
        t
    }

    /// Depth-first traversal yielding (id, world transform, accumulated
    /// opacity). Opacity multiplies down the tree so animating a group's
    /// opacity fades its children (e.g. a formula's glyphs).
    pub fn traverse(&self) -> Vec<(NodeId, Affine, f32)> {
        let mut out = Vec::with_capacity(self.nodes.len());
        let mut stack: Vec<(NodeId, Affine, f32)> = self
            .roots
            .iter()
            .rev()
            .map(|&r| (r, Affine::IDENTITY, 1.0))
            .collect();
        while let Some((id, parent_t, parent_opacity)) = stack.pop() {
            let node = &self.nodes[id];
            let world = parent_t * node.mobject.transform;
            let opacity = parent_opacity * node.mobject.style.opacity;
            out.push((id, world, opacity));
            for &c in node.children.iter().rev() {
                stack.push((c, world, opacity));
            }
        }
        // Stable by tree order, then z_index so higher values paint on top.
        out.sort_by_key(|(id, _, _)| self.nodes[*id].mobject.z_index);
        out
    }

    pub fn set_z_index(&mut self, id: NodeId, z: i32) {
        self.get_mut(id).z_index = z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry;

    #[test]
    fn dirty_flags_track_mutation() {
        let mut g = SceneGraph::new();
        let a = g.add(Mobject::new(geometry::circle(kurbo::Point::ORIGIN, 1.0)));
        let b = g.add(Mobject::new(geometry::square(kurbo::Point::ORIGIN, 2.0)));
        assert!(g.any_dirty());
        g.clear_dirty();
        assert!(!g.any_dirty());

        g.get_mut(a).style.opacity = 0.5;
        assert!(g.any_dirty());
        g.clear_dirty();

        // Read-only access must not dirty the graph.
        let _ = g.get(b).path.elements().len();
        assert!(!g.any_dirty());
    }

    #[test]
    fn world_transform_composes() {
        let mut g = SceneGraph::new();
        let parent = g.add(Mobject::group().shifted(kurbo::Vec2::new(1.0, 0.0)));
        let child = g.add_child(
            parent,
            Mobject::new(geometry::circle(kurbo::Point::ORIGIN, 1.0))
                .shifted(kurbo::Vec2::new(0.0, 2.0)),
        );
        let t = g.world_transform(child);
        let p = t * kurbo::Point::ORIGIN;
        assert!((p.x - 1.0).abs() < 1e-9 && (p.y - 2.0).abs() < 1e-9);
    }

    #[test]
    fn higher_z_index_paints_later() {
        let mut g = SceneGraph::new();
        let a = g.add(Mobject::new(geometry::circle(kurbo::Point::ORIGIN, 1.0)));
        let b = g.add(Mobject::new(geometry::square(kurbo::Point::ORIGIN, 2.0)));
        g.set_z_index(a, 5);
        let order: Vec<NodeId> = g.traverse().into_iter().map(|(id, _, _)| id).collect();
        assert_eq!(order, vec![b, a]);
    }
}
