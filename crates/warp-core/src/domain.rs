// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Domain separation prefixes for hashing.

/// Prefix for canonical state hashes (Stage B1).
pub const STATE_ROOT_V1: &[u8] = b"echo:state_root:v1\0";

/// Prefix for tick patch digests.
pub const PATCH_DIGEST_V1: &[u8] = b"echo:patch_digest:v1\0";

/// Prefix for commit identifiers (V2).
pub const COMMIT_ID_V2: &[u8] = b"echo:commit_id:v2\0";

/// Prefix for `RenderGraph` canonical bytes (T-1-1-2).
pub const RENDER_GRAPH_V1: &[u8] = b"echo:render_graph:v1\0";
