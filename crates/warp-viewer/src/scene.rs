// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Scene construction and lightweight simulation for the viewer.

use blake3::Hasher;
use ciborium::de::from_reader;
use echo_graph::RenderGraph as WireGraph;
use glam::Vec3;
use serde::Deserialize;
use warp_core::{
    make_edge_id, make_node_id, make_type_id, EdgeRecord, GraphStore, NodeRecord, TypeId,
};

#[derive(Clone, Debug)]
pub struct RenderNode {
    #[allow(dead_code)]
    pub ty: TypeId,
    pub color: [f32; 3],
    pub pos: Vec3,
    pub vel: Vec3,
}

#[derive(Clone, Debug, Default)]
pub struct RenderGraph {
    pub nodes: Vec<RenderNode>,
    pub edges: Vec<(usize, usize)>,
    #[allow(dead_code)]
    pub max_depth: usize,
}

impl RenderGraph {
    pub fn step_layout(&mut self, dt: f32) {
        let n = self.nodes.len();
        if n == 0 {
            return;
        }
        let mut forces = vec![Vec3::ZERO; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let delta = self.nodes[i].pos - self.nodes[j].pos;
                let dist2 = delta.length_squared().max(9.0);
                let f = delta.normalize_or_zero() * (2400.0 / dist2);
                forces[i] += f;
                forces[j] -= f;
            }
        }
        for &(a, b) in &self.edges {
            let delta = self.nodes[b].pos - self.nodes[a].pos;
            let dist = delta.length().max(1.0);
            let dir = delta / dist;
            let target = 140.0;
            let f = dir * ((dist - target) * 0.08);
            forces[a] += f;
            forces[b] -= f;
        }
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.vel += forces[i] * dt;
            node.vel *= 0.9;
            node.pos += node.vel * dt;
        }
    }

    pub fn bounding_radius(&self) -> f32 {
        self.nodes
            .iter()
            .map(|n| n.pos.length())
            .fold(0.0, f32::max)
            .max(1.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct HistoryNode {
    #[allow(dead_code)]
    pub graph: WireGraph,
    #[allow(dead_code)]
    pub revision: u64,
    pub next: Option<Box<HistoryNode>>,
}

#[derive(Clone, Debug, Default)]
pub struct History {
    pub head: Option<Box<HistoryNode>>,
    pub tail_rev: u64,
    pub len: usize,
}

impl History {
    pub fn append(&mut self, graph: WireGraph, revision: u64) {
        let node = Box::new(HistoryNode {
            graph,
            revision,
            next: None,
        });
        match self.head.as_mut() {
            None => {
                self.tail_rev = revision;
                self.head = Some(node);
                self.len = 1;
            }
            Some(head) => {
                let mut cur = head;
                while cur.next.is_some() {
                    let next = cur.next.as_mut().unwrap();
                    cur = next;
                }
                cur.next = Some(node);
                self.tail_rev = revision;
                self.len += 1;
            }
        }
    }
}

fn id_to_u64(bytes: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    let take = bytes.len().min(8);
    arr[..take].copy_from_slice(&bytes[..take]);
    u64::from_le_bytes(arr)
}

fn radial_pos_u64(id: u64) -> Vec3 {
    let mut h = Hasher::new();
    h.update(&id.to_le_bytes());
    let bytes = h.finalize();
    let theta = u32::from_le_bytes(bytes.as_bytes()[0..4].try_into().unwrap()) as f32
        / u32::MAX as f32
        * std::f32::consts::TAU;
    let phi = u32::from_le_bytes(bytes.as_bytes()[4..8].try_into().unwrap()) as f32
        / u32::MAX as f32
        * std::f32::consts::PI
        - std::f32::consts::FRAC_PI_2;
    let r = 200.0;
    Vec3::new(
        r * phi.cos() * theta.cos(),
        r * phi.sin(),
        r * phi.cos() * theta.sin(),
    )
}

fn hash_color_u64(id: u64) -> [f32; 3] {
    let h = blake3::hash(&id.to_be_bytes());
    let b = h.as_bytes();
    [
        b[0] as f32 / 255.0,
        b[1] as f32 / 255.0,
        b[2] as f32 / 255.0,
    ]
}

fn compute_depth(edges: &[(usize, usize)], n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut adj = vec![Vec::new(); n];
    for &(a, b) in edges {
        if a < n && b < n {
            adj[a].push(b);
        }
    }
    let mut depth = vec![0usize; n];
    let mut stack = vec![0usize];
    let mut visited = vec![false; n];
    while let Some(v) = stack.pop() {
        visited[v] = true;
        let d = depth[v] + 1;
        for &m in &adj[v] {
            depth[m] = depth[m].max(d);
            if !visited[m] {
                stack.push(m);
            }
        }
    }
    depth.into_iter().max().unwrap_or(0)
}

/// Build a renderable graph from wire-format graph data.
pub fn scene_from_wire(w: &WireGraph) -> RenderGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct Payload {
        #[serde(default)]
        pos: Option<[f32; 3]>,
        #[serde(default)]
        color: Option<[f32; 3]>,
    }

    let mut id_to_idx = HashMap::new();
    for (i, n) in w.nodes.iter().enumerate() {
        id_to_idx.insert(n.id, i);
        let mut pos = radial_pos_u64(i as u64);
        let mut color = hash_color_u64(n.id);

        let payload: Option<Payload> = from_reader(&n.data.raw[..])
            .ok()
            .or_else(|| serde_json::from_slice(&n.data.raw).ok());
        if let Some(p) = payload {
            if let Some(pv) = p.pos {
                pos = Vec3::from_array(pv);
            }
            if let Some(cv) = p.color {
                color = cv;
            }
        }

        nodes.push(RenderNode {
            ty: make_type_id("node"),
            color,
            pos,
            vel: Vec3::ZERO,
        });
    }

    for e in &w.edges {
        if let (Some(a), Some(b)) = (id_to_idx.get(&e.src), id_to_idx.get(&e.dst)) {
            edges.push((*a, *b));
        }
    }

    let max_depth = compute_depth(&edges, nodes.len());
    RenderGraph {
        nodes,
        edges,
        max_depth,
    }
}

pub fn sample_wire_graph() -> WireGraph {
    let store = build_sample_graph();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (id, node) in store.iter_nodes() {
        nodes.push(echo_graph::RenderNode {
            id: id_to_u64(&id.0),
            kind: echo_graph::NodeKind::Generic,
            data: echo_graph::NodeData { raw: Vec::new() },
        });
        let _ = node;
    }
    for (from, outs) in store.iter_edges() {
        for e in outs {
            edges.push(echo_graph::RenderEdge {
                id: id_to_u64(&e.id.0),
                src: id_to_u64(&from.0),
                dst: id_to_u64(&e.to.0),
                kind: echo_graph::EdgeKind::Generic,
                data: echo_graph::EdgeData { raw: Vec::new() },
            });
        }
    }
    WireGraph { nodes, edges }
}

/// Placeholder sample graph until connected to live Echo stream.
fn build_sample_graph() -> GraphStore {
    let mut store = GraphStore::default();
    let world_ty = make_type_id("world");
    let region_ty = make_type_id("region");
    let leaf_ty = make_type_id("leaf");
    let worm_ty = make_type_id("wormhole");

    let world = make_node_id("world");
    store.insert_node(world, NodeRecord { ty: world_ty });

    for i in 0..8u8 {
        let id = make_node_id(&format!("region-{i}"));
        store.insert_node(id, NodeRecord { ty: region_ty });
        store.insert_edge(
            world,
            EdgeRecord {
                id: make_edge_id(&format!("world-region-{i}")),
                from: world,
                to: id,
                ty: region_ty,
            },
        );
        for j in 0..3u8 {
            let leaf = make_node_id(&format!("leaf-{i}-{j}"));
            store.insert_node(leaf, NodeRecord { ty: leaf_ty });
            store.insert_edge(
                id,
                EdgeRecord {
                    id: make_edge_id(&format!("edge-{i}-{j}")),
                    from: id,
                    to: leaf,
                    ty: leaf_ty,
                },
            );
        }
    }
    for pair in [(0, 3), (2, 6), (5, 7)] {
        let (a, b) = pair;
        let a_id = make_node_id(&format!("region-{a}"));
        let b_id = make_node_id(&format!("region-{b}"));
        store.insert_edge(
            a_id,
            EdgeRecord {
                id: make_edge_id(&format!("worm-{a}-{b}")),
                from: a_id,
                to: b_id,
                ty: worm_ty,
            },
        );
        store.insert_edge(
            b_id,
            EdgeRecord {
                id: make_edge_id(&format!("worm-{b}-{a}")),
                from: b_id,
                to: a_id,
                ty: worm_ty,
            },
        );
    }
    store
}
