// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! CBOR encoding and decoding for scene port types.
//!
//! Uses minicbor for efficient CBOR serialization.
//! Due to Rust's orphan rules, we use wrapper types for encoding/decoding.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use echo_scene_port::{
    CameraState, ColorRgba8, EdgeDef, EdgeKey, EdgeStyle, Hash, HighlightState, LabelAnchor,
    LabelDef, LabelKey, NodeDef, NodeKey, NodeShape, ProjectionKind, SceneDelta, SceneOp, MAX_OPS,
};
use minicbor::{Decoder, Encoder};

// ============================================================================
// Encoding helpers
// ============================================================================

fn encode_hash<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    hash: &Hash,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.bytes(hash)?;
    Ok(())
}

fn decode_hash(d: &mut Decoder<'_>) -> Result<Hash, minicbor::decode::Error> {
    let bytes = d.bytes()?;
    if bytes.len() != 32 {
        return Err(minicbor::decode::Error::message(format!(
            "Hash expected 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(bytes);
    Ok(hash)
}

fn encode_node_key<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    key: &NodeKey,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    encode_hash(e, &key.0)
}

fn decode_node_key(d: &mut Decoder<'_>) -> Result<NodeKey, minicbor::decode::Error> {
    Ok(NodeKey(decode_hash(d)?))
}

fn encode_edge_key<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    key: &EdgeKey,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    encode_hash(e, &key.0)
}

fn decode_edge_key(d: &mut Decoder<'_>) -> Result<EdgeKey, minicbor::decode::Error> {
    Ok(EdgeKey(decode_hash(d)?))
}

fn encode_label_key<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    key: &LabelKey,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    encode_hash(e, &key.0)
}

fn decode_label_key(d: &mut Decoder<'_>) -> Result<LabelKey, minicbor::decode::Error> {
    Ok(LabelKey(decode_hash(d)?))
}

fn encode_color<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    color: &ColorRgba8,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.bytes(color)?;
    Ok(())
}

fn decode_color(d: &mut Decoder<'_>) -> Result<ColorRgba8, minicbor::decode::Error> {
    let bytes = d.bytes()?;
    if bytes.len() != 4 {
        return Err(minicbor::decode::Error::message(format!(
            "ColorRgba8 expected 4 bytes, got {}",
            bytes.len()
        )));
    }
    let mut color = [0u8; 4];
    color.copy_from_slice(bytes);
    Ok(color)
}

fn encode_f32_array<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    arr: &[f32; 3],
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.array(3)?;
    e.f32(arr[0])?;
    e.f32(arr[1])?;
    e.f32(arr[2])?;
    Ok(())
}

fn decode_f32_array(d: &mut Decoder<'_>) -> Result<[f32; 3], minicbor::decode::Error> {
    let len = d
        .array()?
        .ok_or_else(|| minicbor::decode::Error::message("expected definite array for f32 array"))?;
    if len != 3 {
        return Err(minicbor::decode::Error::message(format!(
            "f32 array expected 3 elements, got {}",
            len
        )));
    }
    // Accept both f32 and f64 from wire for robustness
    Ok([
        decode_robust_f32(d)?,
        decode_robust_f32(d)?,
        decode_robust_f32(d)?,
    ])
}

fn decode_robust_f32(d: &mut Decoder<'_>) -> Result<f32, minicbor::decode::Error> {
    match d.datatype()? {
        minicbor::data::Type::F32 => d.f32(),
        minicbor::data::Type::F64 => Ok(d.f64()? as f32),
        t => Err(minicbor::decode::Error::message(format!(
            "expected float, got {:?}",
            t
        ))),
    }
}

// ============================================================================
// NodeShape
// ============================================================================

fn encode_node_shape<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    shape: &NodeShape,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.u8(*shape as u8)?;
    Ok(())
}

fn decode_node_shape(d: &mut Decoder<'_>) -> Result<NodeShape, minicbor::decode::Error> {
    match d.u8()? {
        0 => Ok(NodeShape::Sphere),
        1 => Ok(NodeShape::Cube),
        n => Err(minicbor::decode::Error::message(format!(
            "invalid NodeShape: {}",
            n
        ))),
    }
}

// ============================================================================
// EdgeStyle
// ============================================================================

fn encode_edge_style<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    style: &EdgeStyle,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.u8(*style as u8)?;
    Ok(())
}

fn decode_edge_style(d: &mut Decoder<'_>) -> Result<EdgeStyle, minicbor::decode::Error> {
    match d.u8()? {
        0 => Ok(EdgeStyle::Solid),
        1 => Ok(EdgeStyle::Dashed),
        n => Err(minicbor::decode::Error::message(format!(
            "invalid EdgeStyle: {}",
            n
        ))),
    }
}

// ============================================================================
// ProjectionKind
// ============================================================================

fn encode_projection_kind<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    kind: &ProjectionKind,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    match kind {
        ProjectionKind::Perspective => e.u8(0)?,
        ProjectionKind::Orthographic => e.u8(1)?,
    };
    Ok(())
}

fn decode_projection_kind(d: &mut Decoder<'_>) -> Result<ProjectionKind, minicbor::decode::Error> {
    match d.u8()? {
        0 => Ok(ProjectionKind::Perspective),
        1 => Ok(ProjectionKind::Orthographic),
        n => Err(minicbor::decode::Error::message(format!(
            "invalid ProjectionKind: {}",
            n
        ))),
    }
}

// ============================================================================
// NodeDef
// ============================================================================

fn encode_node_def<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    node: &NodeDef,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.array(5)?;
    encode_node_key(e, &node.key)?;
    encode_f32_array(e, &node.position)?;
    e.f32(node.radius)?;
    encode_node_shape(e, &node.shape)?;
    encode_color(e, &node.color)?;
    Ok(())
}

fn decode_node_def(d: &mut Decoder<'_>) -> Result<NodeDef, minicbor::decode::Error> {
    let len = d
        .array()?
        .ok_or_else(|| minicbor::decode::Error::message("expected definite array for NodeDef"))?;
    if len != 5 {
        return Err(minicbor::decode::Error::message(format!(
            "NodeDef expected 5 fields, got {}",
            len
        )));
    }
    Ok(NodeDef {
        key: decode_node_key(d)?,
        position: decode_f32_array(d)?,
        radius: d.f32()?,
        shape: decode_node_shape(d)?,
        color: decode_color(d)?,
    })
}

// ============================================================================
// EdgeDef
// ============================================================================

fn encode_edge_def<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    edge: &EdgeDef,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.array(6)?;
    encode_edge_key(e, &edge.key)?;
    encode_node_key(e, &edge.a)?;
    encode_node_key(e, &edge.b)?;
    e.f32(edge.width)?;
    encode_edge_style(e, &edge.style)?;
    encode_color(e, &edge.color)?;
    Ok(())
}

fn decode_edge_def(d: &mut Decoder<'_>) -> Result<EdgeDef, minicbor::decode::Error> {
    let len = d
        .array()?
        .ok_or_else(|| minicbor::decode::Error::message("expected definite array for EdgeDef"))?;
    if len != 6 {
        return Err(minicbor::decode::Error::message(format!(
            "EdgeDef expected 6 fields, got {}",
            len
        )));
    }
    Ok(EdgeDef {
        key: decode_edge_key(d)?,
        a: decode_node_key(d)?,
        b: decode_node_key(d)?,
        width: d.f32()?,
        style: decode_edge_style(d)?,
        color: decode_color(d)?,
    })
}

// ============================================================================
// LabelAnchor
// ============================================================================

fn encode_label_anchor<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    anchor: &LabelAnchor,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    match anchor {
        LabelAnchor::Node { key } => {
            e.array(2)?;
            e.u8(0)?;
            encode_node_key(e, key)?;
        }
        LabelAnchor::World { position } => {
            e.array(2)?;
            e.u8(1)?;
            encode_f32_array(e, position)?;
        }
    }
    Ok(())
}

fn decode_label_anchor(d: &mut Decoder<'_>) -> Result<LabelAnchor, minicbor::decode::Error> {
    let len = d.array()?.ok_or_else(|| {
        minicbor::decode::Error::message("expected definite array for LabelAnchor")
    })?;
    if len != 2 {
        return Err(minicbor::decode::Error::message(format!(
            "LabelAnchor expected 2 fields, got {}",
            len
        )));
    }
    match d.u8()? {
        0 => Ok(LabelAnchor::Node {
            key: decode_node_key(d)?,
        }),
        1 => Ok(LabelAnchor::World {
            position: decode_f32_array(d)?,
        }),
        n => Err(minicbor::decode::Error::message(format!(
            "invalid LabelAnchor tag: {}",
            n
        ))),
    }
}

// ============================================================================
// LabelDef
// ============================================================================

fn encode_label_def<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    label: &LabelDef,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.array(6)?;
    encode_label_key(e, &label.key)?;
    e.str(&label.text)?;
    e.f32(label.font_size)?;
    encode_color(e, &label.color)?;
    encode_label_anchor(e, &label.anchor)?;
    encode_f32_array(e, &label.offset)?;
    Ok(())
}

fn decode_label_def(d: &mut Decoder<'_>) -> Result<LabelDef, minicbor::decode::Error> {
    let len = d
        .array()?
        .ok_or_else(|| minicbor::decode::Error::message("expected definite array for LabelDef"))?;
    if len != 6 {
        return Err(minicbor::decode::Error::message(format!(
            "LabelDef expected 6 fields, got {}",
            len
        )));
    }
    Ok(LabelDef {
        key: decode_label_key(d)?,
        text: String::from(d.str()?),
        font_size: d.f32()?,
        color: decode_color(d)?,
        anchor: decode_label_anchor(d)?,
        offset: decode_f32_array(d)?,
    })
}

// ============================================================================
// SceneOp
// ============================================================================

fn encode_scene_op<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    op: &SceneOp,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    match op {
        SceneOp::UpsertNode(def) => {
            e.array(2)?;
            e.u8(0)?;
            encode_node_def(e, def)?;
        }
        SceneOp::RemoveNode { key } => {
            e.array(2)?;
            e.u8(1)?;
            encode_node_key(e, key)?;
        }
        SceneOp::UpsertEdge(def) => {
            e.array(2)?;
            e.u8(2)?;
            encode_edge_def(e, def)?;
        }
        SceneOp::RemoveEdge { key } => {
            e.array(2)?;
            e.u8(3)?;
            encode_edge_key(e, key)?;
        }
        SceneOp::UpsertLabel(def) => {
            e.array(2)?;
            e.u8(4)?;
            encode_label_def(e, def)?;
        }
        SceneOp::RemoveLabel { key } => {
            e.array(2)?;
            e.u8(5)?;
            encode_label_key(e, key)?;
        }
        SceneOp::Clear => {
            e.array(1)?;
            e.u8(6)?;
        }
    }
    Ok(())
}

fn decode_scene_op(d: &mut Decoder<'_>) -> Result<SceneOp, minicbor::decode::Error> {
    let len = d
        .array()?
        .ok_or_else(|| minicbor::decode::Error::message("expected definite array for SceneOp"))?;
    let tag = d.u8()?;
    match tag {
        0 => {
            if len != 2 {
                return Err(minicbor::decode::Error::message(
                    "UpsertNode expected 2 fields",
                ));
            }
            Ok(SceneOp::UpsertNode(decode_node_def(d)?))
        }
        1 => {
            if len != 2 {
                return Err(minicbor::decode::Error::message(
                    "RemoveNode expected 2 fields",
                ));
            }
            Ok(SceneOp::RemoveNode {
                key: decode_node_key(d)?,
            })
        }
        2 => {
            if len != 2 {
                return Err(minicbor::decode::Error::message(
                    "UpsertEdge expected 2 fields",
                ));
            }
            Ok(SceneOp::UpsertEdge(decode_edge_def(d)?))
        }
        3 => {
            if len != 2 {
                return Err(minicbor::decode::Error::message(
                    "RemoveEdge expected 2 fields",
                ));
            }
            Ok(SceneOp::RemoveEdge {
                key: decode_edge_key(d)?,
            })
        }
        4 => {
            if len != 2 {
                return Err(minicbor::decode::Error::message(
                    "UpsertLabel expected 2 fields",
                ));
            }
            Ok(SceneOp::UpsertLabel(decode_label_def(d)?))
        }
        5 => {
            if len != 2 {
                return Err(minicbor::decode::Error::message(
                    "RemoveLabel expected 2 fields",
                ));
            }
            Ok(SceneOp::RemoveLabel {
                key: decode_label_key(d)?,
            })
        }
        6 => {
            if len != 1 {
                return Err(minicbor::decode::Error::message("Clear expected 1 field"));
            }
            Ok(SceneOp::Clear)
        }
        n => Err(minicbor::decode::Error::message(format!(
            "invalid SceneOp tag: {}",
            n
        ))),
    }
}

// ============================================================================
// SceneDelta
// ============================================================================

fn encode_scene_delta_inner<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    delta: &SceneDelta,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.array(5)?;
    e.u8(1)?; // Wire version
    encode_hash(e, &delta.session_id)?;
    encode_hash(e, &delta.cursor_id)?;
    e.u64(delta.epoch)?;
    e.array(delta.ops.len() as u64)?;
    for op in &delta.ops {
        encode_scene_op(e, op)?;
    }
    Ok(())
}

fn decode_scene_delta_inner(d: &mut Decoder<'_>) -> Result<SceneDelta, minicbor::decode::Error> {
    let len = d.array()?.ok_or_else(|| {
        minicbor::decode::Error::message("expected definite array for SceneDelta")
    })?;
    if len != 5 {
        return Err(minicbor::decode::Error::message(format!(
            "SceneDelta expected 5 fields, got {}",
            len
        )));
    }
    let version = d.u8()?;
    if version != 1 {
        return Err(minicbor::decode::Error::message(format!(
            "unsupported SceneDelta version: {}",
            version
        )));
    }
    let session_id = decode_hash(d)?;
    let cursor_id = decode_hash(d)?;
    let epoch = d.u64()?;
    let ops_len = d
        .array()?
        .ok_or_else(|| minicbor::decode::Error::message("expected definite array for ops"))?;
    if ops_len as usize > MAX_OPS {
        return Err(minicbor::decode::Error::message(format!(
            "SceneDelta ops count {} exceeds MAX_OPS {}",
            ops_len, MAX_OPS
        )));
    }
    let mut ops = Vec::with_capacity(ops_len as usize);
    for _ in 0..ops_len {
        ops.push(decode_scene_op(d)?);
    }
    Ok(SceneDelta {
        session_id,
        cursor_id,
        epoch,
        ops,
    })
}

// ============================================================================
// CameraState
// ============================================================================

fn encode_camera_state_inner<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    camera: &CameraState,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.array(9)?;
    e.u8(1)?; // Wire version
    encode_f32_array(e, &camera.position)?;
    encode_f32_array(e, &camera.target)?;
    encode_f32_array(e, &camera.up)?;
    encode_projection_kind(e, &camera.projection)?;
    e.f32(camera.fov_y_radians)?;
    e.f32(camera.ortho_scale)?;
    e.f32(camera.near)?;
    e.f32(camera.far)?;
    Ok(())
}

fn decode_camera_state_inner(d: &mut Decoder<'_>) -> Result<CameraState, minicbor::decode::Error> {
    let len = d.array()?.ok_or_else(|| {
        minicbor::decode::Error::message("expected definite array for CameraState")
    })?;
    if len != 9 {
        return Err(minicbor::decode::Error::message(format!(
            "CameraState expected 9 fields, got {}",
            len
        )));
    }
    let version = d.u8()?;
    if version != 1 {
        return Err(minicbor::decode::Error::message(format!(
            "unsupported CameraState version: {}",
            version
        )));
    }
    Ok(CameraState {
        position: decode_f32_array(d)?,
        target: decode_f32_array(d)?,
        up: decode_f32_array(d)?,
        projection: decode_projection_kind(d)?,
        fov_y_radians: d.f32()?,
        ortho_scale: d.f32()?,
        near: d.f32()?,
        far: d.f32()?,
    })
}

// ============================================================================
// HighlightState
// ============================================================================

fn encode_highlight_state_inner<W: minicbor::encode::Write>(
    e: &mut Encoder<W>,
    highlight: &HighlightState,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.array(5)?;
    e.u8(1)?; // Wire version
              // selected_nodes
    e.array(highlight.selected_nodes.len() as u64)?;
    for key in &highlight.selected_nodes {
        encode_node_key(e, key)?;
    }
    // selected_edges
    e.array(highlight.selected_edges.len() as u64)?;
    for key in &highlight.selected_edges {
        encode_edge_key(e, key)?;
    }
    // hovered_node
    match &highlight.hovered_node {
        Some(key) => {
            e.array(1)?;
            encode_node_key(e, key)?;
        }
        None => {
            e.array(0)?;
        }
    }
    // hovered_edge
    match &highlight.hovered_edge {
        Some(key) => {
            e.array(1)?;
            encode_edge_key(e, key)?;
        }
        None => {
            e.array(0)?;
        }
    }
    Ok(())
}

fn decode_highlight_state_inner(
    d: &mut Decoder<'_>,
) -> Result<HighlightState, minicbor::decode::Error> {
    let len = d.array()?.ok_or_else(|| {
        minicbor::decode::Error::message("expected definite array for HighlightState")
    })?;
    if len != 5 {
        return Err(minicbor::decode::Error::message(format!(
            "HighlightState expected 5 fields, got {}",
            len
        )));
    }
    let version = d.u8()?;
    if version != 1 {
        return Err(minicbor::decode::Error::message(format!(
            "unsupported HighlightState version: {}",
            version
        )));
    }
    // selected_nodes
    let nodes_len = d.array()?.ok_or_else(|| {
        minicbor::decode::Error::message("expected definite array for selected_nodes")
    })?;
    if nodes_len as usize > MAX_OPS {
        return Err(minicbor::decode::Error::message("too many selected nodes"));
    }
    let mut selected_nodes = Vec::with_capacity(nodes_len as usize);
    for _ in 0..nodes_len {
        selected_nodes.push(decode_node_key(d)?);
    }
    // selected_edges
    let edges_len = d.array()?.ok_or_else(|| {
        minicbor::decode::Error::message("expected definite array for selected_edges")
    })?;
    if edges_len as usize > MAX_OPS {
        return Err(minicbor::decode::Error::message("too many selected edges"));
    }
    let mut selected_edges = Vec::with_capacity(edges_len as usize);
    for _ in 0..edges_len {
        selected_edges.push(decode_edge_key(d)?);
    }
    // hovered_node
    let hovered_node_len = d.array()?.ok_or_else(|| {
        minicbor::decode::Error::message("expected definite array for hovered_node")
    })?;
    if hovered_node_len > 1 {
        return Err(minicbor::decode::Error::message(
            "expected array of length 0 or 1 for hovered_node",
        ));
    }
    let hovered_node = if hovered_node_len == 0 {
        None
    } else {
        Some(decode_node_key(d)?)
    };
    // hovered_edge
    let hovered_edge_len = d.array()?.ok_or_else(|| {
        minicbor::decode::Error::message("expected definite array for hovered_edge")
    })?;
    if hovered_edge_len > 1 {
        return Err(minicbor::decode::Error::message(
            "expected array of length 0 or 1 for hovered_edge",
        ));
    }
    let hovered_edge = if hovered_edge_len == 0 {
        None
    } else {
        Some(decode_edge_key(d)?)
    };
    Ok(HighlightState {
        selected_nodes,
        selected_edges,
        hovered_node,
        hovered_edge,
    })
}

// ============================================================================
// Public encode/decode functions
// ============================================================================

/// Encode a SceneDelta to CBOR bytes.
pub fn encode_scene_delta(delta: &SceneDelta) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut encoder = Encoder::new(&mut buf);
    encode_scene_delta_inner(&mut encoder, delta).expect("encoding should not fail");
    buf
}

/// Decode a SceneDelta from CBOR bytes.
pub fn decode_scene_delta(bytes: &[u8]) -> Result<SceneDelta, minicbor::decode::Error> {
    let mut decoder = Decoder::new(bytes);
    let delta = decode_scene_delta_inner(&mut decoder)?;
    if decoder.position() < bytes.len() {
        return Err(minicbor::decode::Error::message(
            "trailing bytes in SceneDelta",
        ));
    }
    Ok(delta)
}

/// Encode a CameraState to CBOR bytes.
pub fn encode_camera_state(camera: &CameraState) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut encoder = Encoder::new(&mut buf);
    encode_camera_state_inner(&mut encoder, camera).expect("encoding should not fail");
    buf
}

/// Decode a CameraState from CBOR bytes.
pub fn decode_camera_state(bytes: &[u8]) -> Result<CameraState, minicbor::decode::Error> {
    let mut decoder = Decoder::new(bytes);
    let state = decode_camera_state_inner(&mut decoder)?;
    if decoder.position() < bytes.len() {
        return Err(minicbor::decode::Error::message(
            "trailing bytes in CameraState",
        ));
    }
    Ok(state)
}

/// Encode a HighlightState to CBOR bytes.
pub fn encode_highlight_state(highlight: &HighlightState) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut encoder = Encoder::new(&mut buf);
    encode_highlight_state_inner(&mut encoder, highlight).expect("encoding should not fail");
    buf
}

/// Decode a HighlightState from CBOR bytes.
pub fn decode_highlight_state(bytes: &[u8]) -> Result<HighlightState, minicbor::decode::Error> {
    let mut decoder = Decoder::new(bytes);
    let state = decode_highlight_state_inner(&mut decoder)?;
    if decoder.position() < bytes.len() {
        return Err(minicbor::decode::Error::message(
            "trailing bytes in HighlightState",
        ));
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_hash(seed: u8) -> Hash {
        let mut hash = [0u8; 32];
        for (i, byte) in hash.iter_mut().enumerate() {
            *byte = seed.wrapping_add(i as u8);
        }
        hash
    }

    #[test]
    fn test_node_def_roundtrip() {
        let node = NodeDef {
            key: NodeKey(make_test_hash(1)),
            position: [1.0, 2.0, 3.0],
            radius: 0.5,
            shape: NodeShape::Sphere,
            color: [255, 128, 64, 255],
        };
        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        encode_node_def(&mut encoder, &node).unwrap();

        let mut decoder = Decoder::new(&buf);
        let decoded = decode_node_def(&mut decoder).unwrap();
        assert_eq!(node, decoded);
    }

    #[test]
    fn test_edge_def_roundtrip() {
        let edge = EdgeDef {
            key: EdgeKey(make_test_hash(1)),
            a: NodeKey(make_test_hash(2)),
            b: NodeKey(make_test_hash(3)),
            width: 0.1,
            style: EdgeStyle::Dashed,
            color: [100, 150, 200, 128],
        };
        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        encode_edge_def(&mut encoder, &edge).unwrap();

        let mut decoder = Decoder::new(&buf);
        let decoded = decode_edge_def(&mut decoder).unwrap();
        assert_eq!(edge, decoded);
    }

    #[test]
    fn test_label_anchor_node_roundtrip() {
        let anchor = LabelAnchor::Node {
            key: NodeKey(make_test_hash(5)),
        };
        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        encode_label_anchor(&mut encoder, &anchor).unwrap();

        let mut decoder = Decoder::new(&buf);
        let decoded = decode_label_anchor(&mut decoder).unwrap();
        assert_eq!(anchor, decoded);
    }

    #[test]
    fn test_label_anchor_world_roundtrip() {
        let anchor = LabelAnchor::World {
            position: [10.0, 20.0, 30.0],
        };
        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        encode_label_anchor(&mut encoder, &anchor).unwrap();

        let mut decoder = Decoder::new(&buf);
        let decoded = decode_label_anchor(&mut decoder).unwrap();
        assert_eq!(anchor, decoded);
    }

    #[test]
    fn test_label_def_roundtrip() {
        let label = LabelDef {
            key: LabelKey(make_test_hash(1)),
            text: "Hello World".into(),
            font_size: 14.0,
            color: [255, 255, 255, 255],
            anchor: LabelAnchor::Node {
                key: NodeKey(make_test_hash(2)),
            },
            offset: [0.0, 1.0, 0.0],
        };
        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        encode_label_def(&mut encoder, &label).unwrap();

        let mut decoder = Decoder::new(&buf);
        let decoded = decode_label_def(&mut decoder).unwrap();
        assert_eq!(label, decoded);
    }

    #[test]
    fn test_scene_op_roundtrip() {
        let ops = vec![
            SceneOp::UpsertNode(NodeDef {
                key: NodeKey(make_test_hash(1)),
                position: [0.0, 0.0, 0.0],
                radius: 1.0,
                shape: NodeShape::Cube,
                color: [255, 0, 0, 255],
            }),
            SceneOp::RemoveNode {
                key: NodeKey(make_test_hash(2)),
            },
            SceneOp::Clear,
        ];
        for op in &ops {
            let mut buf = Vec::new();
            let mut encoder = Encoder::new(&mut buf);
            encode_scene_op(&mut encoder, op).unwrap();

            let mut decoder = Decoder::new(&buf);
            let decoded = decode_scene_op(&mut decoder).unwrap();
            assert_eq!(*op, decoded);
        }
    }

    #[test]
    fn test_scene_delta_roundtrip() {
        let delta = SceneDelta {
            session_id: make_test_hash(1),
            cursor_id: make_test_hash(2),
            epoch: 42,
            ops: vec![
                SceneOp::UpsertNode(NodeDef {
                    key: NodeKey(make_test_hash(10)),
                    position: [1.0, 2.0, 3.0],
                    radius: 0.5,
                    shape: NodeShape::Sphere,
                    color: [128, 128, 128, 255],
                }),
                SceneOp::Clear,
            ],
        };
        let bytes = encode_scene_delta(&delta);
        let decoded = decode_scene_delta(&bytes).unwrap();
        assert_eq!(delta, decoded);
    }

    #[test]
    fn test_camera_state_roundtrip() {
        let camera = CameraState {
            position: [0.0, 5.0, 10.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            projection: ProjectionKind::Orthographic,
            fov_y_radians: 1.0,
            ortho_scale: 15.0,
            near: 0.1,
            far: 1000.0,
        };
        let bytes = encode_camera_state(&camera);
        let decoded = decode_camera_state(&bytes).unwrap();
        assert_eq!(camera, decoded);
    }

    #[test]
    fn test_highlight_state_roundtrip() {
        let highlight = HighlightState {
            selected_nodes: vec![NodeKey(make_test_hash(1)), NodeKey(make_test_hash(2))],
            selected_edges: vec![EdgeKey(make_test_hash(3))],
            hovered_node: Some(NodeKey(make_test_hash(4))),
            hovered_edge: None,
        };
        let bytes = encode_highlight_state(&highlight);
        let decoded = decode_highlight_state(&bytes).unwrap();
        assert_eq!(highlight, decoded);
    }

    #[test]
    fn test_highlight_state_empty_roundtrip() {
        let highlight = HighlightState::default();
        let bytes = encode_highlight_state(&highlight);
        let decoded = decode_highlight_state(&bytes).unwrap();
        assert_eq!(highlight, decoded);
    }

    #[test]
    fn reject_trailing_garbage() {
        let delta = SceneDelta {
            session_id: make_test_hash(1),
            cursor_id: make_test_hash(2),
            epoch: 42,
            ops: vec![SceneOp::Clear],
        };
        let mut bytes = encode_scene_delta(&delta);
        bytes.push(0xFF); // Trailing garbage

        let result = decode_scene_delta(&bytes);
        assert!(
            result.is_err(),
            "Decoder should reject trailing garbage bytes"
        );
    }

    #[test]
    fn reject_exceeding_max_ops() {
        // Minimal CBOR header for SceneDelta
        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        encoder.array(5).unwrap();
        encoder.u8(1).unwrap(); // Version
        encoder.bytes(&make_test_hash(1)).unwrap(); // session
        encoder.bytes(&make_test_hash(2)).unwrap(); // cursor
        encoder.u64(0).unwrap(); // epoch
        encoder.array((MAX_OPS + 1) as u64).unwrap(); // ops array header

        let result = decode_scene_delta(&buf);
        assert!(
            result.is_err(),
            "Decoder should reject ops count exceeding MAX_OPS"
        );
        let err = result.err().unwrap().to_string();
        assert!(err.contains("exceeds MAX_OPS"));
    }

    #[test]
    fn reject_invalid_version() {
        let mut buf = Vec::new();
        let mut encoder = Encoder::new(&mut buf);
        encoder.array(5).unwrap();
        encoder.u8(99).unwrap(); // Unsupported version
        encoder.bytes(&make_test_hash(1)).unwrap(); // session
        encoder.bytes(&make_test_hash(2)).unwrap(); // cursor
        encoder.u64(0).unwrap(); // epoch
        encoder.array(0).unwrap(); // empty ops

        let result = decode_scene_delta(&buf);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("version"));
    }

    #[test]
    fn reject_invalid_enum_tags() {
        let mut buf = Vec::new();

        // NodeShape: allowed 0..=1
        let mut encoder = Encoder::new(&mut buf);
        encoder.u8(2).unwrap();
        assert!(decode_node_shape(&mut Decoder::new(&buf)).is_err());

        // EdgeStyle: allowed 0..=1
        buf.clear();
        let mut encoder = Encoder::new(&mut buf);
        encoder.u8(2).unwrap();
        assert!(decode_edge_style(&mut Decoder::new(&buf)).is_err());

        // ProjectionKind: allowed 0..=1
        buf.clear();
        let mut encoder = Encoder::new(&mut buf);
        encoder.u8(2).unwrap();
        assert!(decode_projection_kind(&mut Decoder::new(&buf)).is_err());

        // LabelAnchor tag: allowed 0..=1
        buf.clear();
        let mut encoder = Encoder::new(&mut buf);
        encoder.array(2).unwrap();
        encoder.u8(2).unwrap(); // Invalid tag
        assert!(decode_label_anchor(&mut Decoder::new(&buf)).is_err());
    }

    #[test]
    fn drill_truncated_cbor() {
        let delta = SceneDelta {
            session_id: make_test_hash(1),
            cursor_id: make_test_hash(2),
            epoch: 42,
            ops: vec![SceneOp::UpsertNode(NodeDef {
                key: NodeKey(make_test_hash(10)),
                position: [1.0, 2.0, 3.0],
                radius: 0.5,
                shape: NodeShape::Sphere,
                color: [128, 128, 128, 255],
            })],
        };
        let full_bytes = encode_scene_delta(&delta);

        // Try decoding every possible truncated prefix
        for len in 0..full_bytes.len() - 1 {
            let truncated = &full_bytes[..len];
            let result = decode_scene_delta(truncated);
            assert!(
                result.is_err(),
                "Decoding should fail for truncated input of length {}",
                len
            );
        }
    }

    #[test]
    #[cfg(feature = "std")]
    fn stress_concurrent_decode() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let delta = SceneDelta {
            session_id: make_test_hash(1),
            cursor_id: make_test_hash(2),
            epoch: 42,
            ops: vec![SceneOp::Clear],
        };
        let bytes = encode_scene_delta(&delta);
        let bytes_arc = Arc::new(bytes);

        let success_count = Arc::new(Mutex::new(0));
        let mut handles = Vec::new();

        for _ in 0..10 {
            let b = Arc::clone(&bytes_arc);
            let s = Arc::clone(&success_count);
            handles.push(thread::spawn(move || {
                let decoded = decode_scene_delta(&b).unwrap();
                if decoded.epoch == 42 {
                    let mut count = s.lock().unwrap();
                    *count += 1;
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(*success_count.lock().unwrap(), 10);
    }
}
