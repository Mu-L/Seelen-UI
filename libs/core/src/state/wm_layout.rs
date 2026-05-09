use std::collections::HashMap;

use crate::{
    state::{
        twm::{
            TwmCondition, TwmConditionContext, TwmNodeKind, TwmNodeLifetime, TwmPlugin,
            TwmPluginNode, TwmStackPolicy,
        },
        WorkspaceId,
    },
    Rect,
};

pub type NodeId = u64;
pub type WindowId = isize;

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct TwmGlobalRuntimeTree {
    pub workspaces: HashMap<WorkspaceId, TwmRuntimeTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct TwmRuntimeTree {
    pub next_id: NodeId,
    pub root: NodeId,
    pub nodes: HashMap<NodeId, TwmRuntimeNode>,
    pub window_map: HashMap<WindowId, WindowLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub enum WindowLocation {
    Tiled(NodeId),
    Floating,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct TwmRuntimeNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub kind: TwmNodeKind,
    pub lifetime: TwmNodeLifetime,
    pub priority: u32,
    pub grow_factor: f32,
    pub condition: Option<TwmCondition>,
    pub max_stack_size: Option<usize>,
    pub stack_policy: TwmStackPolicy,

    // Runtime-only
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub rect: Option<Rect>,
}

impl Default for TwmRuntimeTree {
    fn default() -> Self {
        Self::new()
    }
}

impl TwmRuntimeTree {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            root: 0,
            nodes: HashMap::new(),
            window_map: HashMap::new(),
        }
    }

    pub fn generate_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn from_plugin(tree: &TwmPlugin) -> Self {
        let mut runtime = Self::new();
        let root_id = runtime.insert_plugin_node(&tree.structure, None);
        runtime.root = root_id;
        runtime
    }

    fn insert_plugin_node(&mut self, node: &TwmPluginNode, parent: Option<NodeId>) -> NodeId {
        let mut runtime_node = TwmRuntimeNode::from_plugin(node);
        runtime_node.parent = parent;

        let id = self.generate_id();
        runtime_node.id = id;
        self.nodes.insert(id, runtime_node);

        let child_ids: Vec<NodeId> = node
            .children
            .iter()
            .map(|child| self.insert_plugin_node(child, Some(id)))
            .collect();

        self.nodes.get_mut(&id).unwrap().children = child_ids;
        id
    }

    pub fn contains(&self, window_id: &WindowId) -> bool {
        self.window_map.contains_key(window_id)
    }

    pub fn is_tiled(&self, id: &WindowId) -> bool {
        matches!(self.window_map.get(id), Some(WindowLocation::Tiled(_)))
    }

    pub fn is_floating(&self, id: &WindowId) -> bool {
        matches!(self.window_map.get(id), Some(WindowLocation::Floating))
    }

    /// traverse sorts nodes by priority, so tree order ≠ processing order
    pub fn traverse<F>(&self, node_id: NodeId, callback: &mut F)
    where
        F: FnMut(&TwmRuntimeNode),
    {
        let node = self.nodes.get(&node_id).unwrap();
        callback(node);

        let mut children = node.children.clone();
        children.sort_by_key(|id| self.nodes.get(id).unwrap().priority);

        for child in children {
            self.traverse(child, callback);
        }
    }

    /// traverse sorts nodes by priority, so tree order ≠ processing order
    pub fn traverse_mut<F>(&mut self, node_id: NodeId, callback: &mut F)
    where
        F: FnMut(&mut TwmRuntimeNode),
    {
        let node = self.nodes.get_mut(&node_id).unwrap();
        callback(node);

        let mut children = node.children.clone();
        children.sort_by_key(|id| self.nodes.get(id).unwrap().priority);

        for child in children {
            self.traverse_mut(child, callback);
        }
    }

    pub fn find<F>(&self, node_id: NodeId, predicate: &mut F) -> Option<NodeId>
    where
        F: FnMut(&TwmRuntimeNode) -> bool,
    {
        let node = self.nodes.get(&node_id)?;
        if predicate(node) {
            return Some(node_id);
        }

        let mut children = node.children.clone();
        children.sort_by_key(|id| self.nodes.get(id).unwrap().priority);
        for child in children {
            if let Some(found) = self.find(child, predicate) {
                return Some(found);
            }
        }
        None
    }

    // TODO: consider cached counters if condition eval becomes hot path
    fn get_context(&self) -> TwmConditionContext {
        let mut tiling_windows = 0;
        let mut floating_windows = 0;
        let mut total_windows = 0;

        for window in self.window_map.values() {
            match window {
                WindowLocation::Tiled(_) => tiling_windows += 1,
                WindowLocation::Floating => floating_windows += 1,
            }
            total_windows += 1;
        }

        TwmConditionContext {
            tiling_windows,
            floating_windows,
            total_windows,
        }
    }

    /// returns true if the window was added, false in case of overflow
    fn try_add_window(&mut self, window_id: WindowId, ctx: &TwmConditionContext) -> bool {
        if let Some(node_id) = self.find(self.root, &mut |n| n.accepts_windows(ctx)) {
            let node = self.nodes.get_mut(&node_id).unwrap();
            node.windows.push(window_id);
            node.active_window = Some(window_id);
            self.window_map
                .insert(window_id, WindowLocation::Tiled(node_id));
            return true;
        }

        if let Some(node_id) = self.find(self.root, &mut |n| n.accepts_windows_on_overflow(ctx)) {
            let node = self.nodes.get_mut(&node_id).unwrap();
            node.windows.push(window_id);
            node.active_window = Some(window_id);
            self.window_map
                .insert(window_id, WindowLocation::Tiled(node_id));
            return true;
        }

        false
    }

    pub fn drain_tiled(&mut self) -> Vec<WindowId> {
        let mut drained = Vec::new();
        self.traverse_mut(self.root, &mut |node| {
            drained.append(&mut node.windows);
            node.active_window = None;
        });
        self.window_map
            .retain(|_, location| matches!(location, WindowLocation::Floating));
        drained
    }

    /// reindexes windows to handle logical condition like `managed < 4` and returns residual windows
    pub fn reindex_windows(&mut self) -> Vec<WindowId> {
        let ctx = self.get_context();

        let windows = self.drain_tiled();
        let mut residual = Vec::new();
        for window in windows {
            if !self.try_add_window(window, &ctx) {
                residual.push(window);
            }
        }
        residual
    }

    /// returns residual windows that should be added to floating
    pub fn add_to_tiled(&mut self, window_id: WindowId) -> Vec<WindowId> {
        let ctx = self.get_context();
        if !self.try_add_window(window_id, &ctx) {
            return vec![window_id];
        }
        self.reindex_windows()
    }

    pub fn add_to_floating(&mut self, window_id: WindowId) {
        self.window_map.insert(window_id, WindowLocation::Floating);
    }

    pub fn remove_window(&mut self, window_id: &WindowId) -> Vec<isize> {
        let Some(location) = self.window_map.remove(window_id) else {
            return Vec::new();
        };
        match location {
            WindowLocation::Tiled(node_id) => {
                let node = self.nodes.get_mut(&node_id).unwrap();
                node.windows.retain(|w| w != window_id);
                if node.active_window == Some(*window_id) {
                    node.active_window = node.windows.first().copied();
                }
            }
            WindowLocation::Floating => {}
        }
        self.reindex_windows()
    }

    pub fn has_any_windows(&self, node_id: NodeId) -> bool {
        let node = &self.nodes[&node_id];
        if !node.windows.is_empty() {
            return true;
        }
        node.children.iter().any(|&c| self.has_any_windows(c))
    }

    pub fn node_of_window(&self, window_id: &WindowId) -> Option<NodeId> {
        match self.window_map.get(window_id)? {
            WindowLocation::Tiled(node_id) => Some(*node_id),
            WindowLocation::Floating => None,
        }
    }

    pub fn face_of_node(&self, node_id: NodeId) -> Option<WindowId> {
        let node = self.nodes.get(&node_id)?;
        match node.kind {
            TwmNodeKind::Leaf | TwmNodeKind::Stack => {
                node.active_window.or_else(|| node.windows.first().copied())
            }
            TwmNodeKind::Horizontal | TwmNodeKind::Vertical => {
                let mut children = node.children.clone();
                children.sort_by_key(|id| self.nodes[id].priority);
                children.iter().find_map(|&c| self.face_of_node(c))
            }
        }
    }

    pub fn node_is_stack(&self, window_id: &WindowId) -> bool {
        self.node_of_window(window_id)
            .and_then(|id| self.nodes.get(&id))
            .map(|n| n.kind == TwmNodeKind::Stack)
            .unwrap_or(false)
    }

    pub fn swap_windows(&mut self, a: WindowId, b: WindowId) {
        let node_a = match self.window_map.get(&a) {
            Some(WindowLocation::Tiled(id)) => *id,
            _ => return,
        };
        let node_b = match self.window_map.get(&b) {
            Some(WindowLocation::Tiled(id)) => *id,
            _ => return,
        };
        if node_a == node_b {
            return;
        }

        // SAFETY: node_a != node_b, so we're getting two distinct entries
        let ptr = &mut self.nodes as *mut HashMap<NodeId, TwmRuntimeNode>;
        let na = unsafe { &mut *ptr }.get_mut(&node_a).unwrap();
        let nb = unsafe { &mut *ptr }.get_mut(&node_b).unwrap();
        std::mem::swap(&mut na.windows, &mut nb.windows);
        std::mem::swap(&mut na.active_window, &mut nb.active_window);

        let windows_a: Vec<WindowId> = self.nodes[&node_a].windows.clone();
        let windows_b: Vec<WindowId> = self.nodes[&node_b].windows.clone();
        for w in windows_a {
            self.window_map.insert(w, WindowLocation::Tiled(node_a));
        }
        for w in windows_b {
            self.window_map.insert(w, WindowLocation::Tiled(node_b));
        }
    }

    pub fn get_nearest_leaf_to_rect(&self, rect: &Rect) -> Option<NodeId> {
        let target = rect.center();
        let mut best: Option<(NodeId, i32)> = None;
        self.traverse(self.root, &mut |node| {
            if !matches!(node.kind, TwmNodeKind::Leaf | TwmNodeKind::Stack) {
                return;
            }
            let Some(node_rect) = &node.rect else {
                return;
            };
            let distance = target.distance_squared(&node_rect.center());
            if best.is_none() || distance < best.unwrap().1 {
                best = Some((node.id, distance));
            }
        });
        best.map(|(id, _)| id)
    }

    pub fn siblings_at_side(
        &self,
        window_id: &WindowId,
        match_horizontal: bool,
        want_before: bool,
    ) -> Vec<NodeId> {
        let Some(mut current_id) = self.node_of_window(window_id) else {
            return vec![];
        };
        let wanted_kind = if match_horizontal {
            TwmNodeKind::Horizontal
        } else {
            TwmNodeKind::Vertical
        };

        loop {
            let Some(parent_id) = self.nodes[&current_id].parent else {
                return vec![];
            };
            let parent = &self.nodes[&parent_id];
            if parent.kind == wanted_kind {
                let child_idx = parent
                    .children
                    .iter()
                    .position(|&c| c == current_id)
                    .unwrap();
                let siblings: Vec<NodeId> = parent
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(idx, &c)| {
                        let correct_side = if want_before {
                            *idx < child_idx
                        } else {
                            *idx > child_idx
                        };
                        *idx != child_idx && correct_side && self.has_any_windows(c)
                    })
                    .map(|(_, &c)| c)
                    .collect();
                if !siblings.is_empty() {
                    return siblings;
                }
            }
            current_id = parent_id;
        }
    }
}

impl TwmRuntimeNode {
    pub fn from_plugin(node: &TwmPluginNode) -> Self {
        Self {
            id: 0,                // to be filled
            parent: None,         // to be filled
            children: Vec::new(), // to be filled
            kind: node.kind,
            lifetime: node.lifetime,
            priority: node.priority,
            grow_factor: node.grow_factor,
            condition: node.condition.clone(),
            max_stack_size: node.max_stack_size,
            stack_policy: node.stack_policy,
            windows: Vec::new(),
            active_window: None,
            rect: None,
        }
    }

    fn accepts_windows(&self, ctx: &TwmConditionContext) -> bool {
        // 1. condition check (DSL rule)
        if let Some(cond) = &self.condition {
            if !cond.evaluate(ctx) {
                return false;
            }
        }

        // 2. structural rules
        match self.kind {
            TwmNodeKind::Leaf => self.windows.is_empty(),
            TwmNodeKind::Stack => {
                self.stack_policy == TwmStackPolicy::Auto
                    && match self.max_stack_size {
                        Some(max) => self.windows.len() < max,
                        None => true, // unlimited stack
                    }
            }
            // these never accept directly
            TwmNodeKind::Vertical | TwmNodeKind::Horizontal => false,
        }
    }

    fn accepts_windows_on_overflow(&self, ctx: &TwmConditionContext) -> bool {
        match self.kind {
            TwmNodeKind::Stack => {
                if let Some(cond) = &self.condition {
                    if !cond.evaluate(ctx) {
                        return false;
                    }
                }

                self.stack_policy == TwmStackPolicy::AutoWhenOverflow
                    && match self.max_stack_size {
                        Some(max) => self.windows.len() < max,
                        None => true, // unlimited stack
                    }
            }
            _ => false,
        }
    }
}
