// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Test-side ContentAddressV1 preimages, assembled field by field without the
//! production module, with readable diagnostics when an identity disagrees.
//!
//! Diagnostics show field names, lengths, type ids and digests. Raw atom bytes
//! appear only when a test explicitly opts in with [`Preimage::show_atom`].
#![allow(dead_code)]

/// One labelled field of a preimage.
pub struct Field {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    /// Atom contents are summarized unless the preimage opts in.
    pub atom: bool,
}

/// A labelled preimage whose BLAKE3 hash is an identity.
pub struct Preimage {
    pub fields: Vec<Field>,
    show_atom: bool,
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Preimage {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            show_atom: false,
        }
    }
    pub fn field(mut self, name: &'static str, bytes: impl Into<Vec<u8>>) -> Self {
        self.fields.push(Field {
            name,
            bytes: bytes.into(),
            atom: false,
        });
        self
    }
    fn atom_field(mut self, name: &'static str, bytes: &[u8]) -> Self {
        self.fields.push(Field {
            name,
            bytes: bytes.to_vec(),
            atom: true,
        });
        self
    }
    /// Opt in to raw atom bytes in diagnostics.
    #[must_use]
    pub fn show_atom(mut self) -> Self {
        self.show_atom = true;
        self
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.fields.iter().flat_map(|f| f.bytes.clone()).collect()
    }
    pub fn hash(&self) -> [u8; 32] {
        blake3::hash(&self.bytes()).into()
    }
    /// `domain | warp | ...` with each field's length and value or digest.
    pub fn describe(&self) -> String {
        self.fields
            .iter()
            .map(|f| {
                let value = if f.atom && !self.show_atom {
                    format!("blake3 {}", hex(blake3::hash(&f.bytes).as_bytes()))
                } else if f.name.contains("domain") {
                    format!("{:?}", String::from_utf8_lossy(&f.bytes))
                } else {
                    hex(&f.bytes)
                };
                format!("{} [{} bytes] {}", f.name, f.bytes.len(), value)
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }
    /// Panics with a field-by-field explanation unless `actual` is this
    /// preimage's hash.
    pub fn assert_identity(&self, actual: [u8; 32], what: &str) {
        assert!(
            actual == self.hash(),
            "{what}: identity {} does not match the expected preimage {}\n  {}",
            hex(&actual),
            hex(&self.hash()),
            self.describe()
        );
    }
}

/// `echo.bridge.attachment-value.v1\0`, atom tag 0, atom type, length u64 LE, bytes.
pub fn attachment(atom_type: [u8; 32], bytes: &[u8]) -> Preimage {
    Preimage::new()
        .field(
            "attachment domain",
            b"echo.bridge.attachment-value.v1\0".to_vec(),
        )
        .field("atom tag", vec![0])
        .field("atom type", atom_type.to_vec())
        .field(
            "atom length (u64 LE)",
            (bytes.len() as u64).to_le_bytes().to_vec(),
        )
        .atom_field("atom bytes", bytes)
}

/// Edges are listed as given; callers sort by (edge type, to) to match the law.
pub fn node(
    warp: [u8; 32],
    node_type: [u8; 32],
    attachment_digest: [u8; 32],
    edges: &[([u8; 32], [u8; 32])],
) -> Preimage {
    let mut preimage = Preimage::new()
        .field("node domain", b"echo.bridge.node-id.v1\0".to_vec())
        .field("warp", warp.to_vec())
        .field("node type", node_type.to_vec())
        .field("attachment digest", attachment_digest.to_vec())
        .field(
            "edge count (u64 LE)",
            (edges.len() as u64).to_le_bytes().to_vec(),
        );
    for (edge_type, to) in edges {
        preimage = preimage
            .field("edge type", edge_type.to_vec())
            .field("edge to", to.to_vec());
    }
    preimage
}

pub fn content_edge(warp: [u8; 32], from: [u8; 32], edge_type: [u8; 32], to: [u8; 32]) -> Preimage {
    Preimage::new()
        .field("content edge domain", b"echo.bridge.edge-id.v1\0".to_vec())
        .field("warp", warp.to_vec())
        .field("from", from.to_vec())
        .field("edge type", edge_type.to_vec())
        .field("to", to.to_vec())
}

pub fn slot_edge(warp: [u8; 32], from: [u8; 32], edge_type: [u8; 32]) -> Preimage {
    Preimage::new()
        .field(
            "slot edge domain",
            b"echo.bridge.slot-edge-id.v1\0".to_vec(),
        )
        .field("warp", warp.to_vec())
        .field("from", from.to_vec())
        .field("edge type", edge_type.to_vec())
}
