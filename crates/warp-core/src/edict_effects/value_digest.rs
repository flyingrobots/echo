// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! One bridge atom-value digest recipe for read evidence and mutation comparisons.
pub(super) fn atom(atom: &crate::AtomPayload) -> crate::Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(b"echo.bridge.attachment-value.v1\0");
    hash.update(&[0]);
    hash.update(&atom.type_id.0);
    hash.update(&(atom.bytes.len() as u64).to_le_bytes());
    hash.update(&atom.bytes);
    hash.finalize().into()
}
