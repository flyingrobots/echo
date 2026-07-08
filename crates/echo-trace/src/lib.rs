// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Echo Trace
//!
//! Graph is truth. Delta log is transport. Roaring is index.
//! Matrix is projection. Receipt is proof.
//!
//! This crate defines the canonical causal delta stream boundary.

/// Represents an execution tick identifier.
pub type TickId = u64;

/// A digest representing a causal frontier in the WARP graph.
pub type FrontierDigest = [u8; 32];

/// The core graph delta, encapsulating the exact WSC rows appended in a tick.
/// Parameterized over the row types to avoid circular dependencies with `warp-core`.
pub struct CanonicalGraphDelta<'a, N, E, A> {
    /// The tick at which this delta occurred.
    pub tick: TickId,
    /// The causal frontier digest before this delta.
    pub causal_frontier_before: FrontierDigest,
    /// The canonical NodeRows appended.
    pub node_rows: &'a [N],
    /// The canonical EdgeRows appended.
    pub edge_rows: &'a [E],
    /// The canonical AttRows appended.
    pub att_rows: &'a [A],
    /// The causal frontier digest after this delta.
    pub causal_frontier_after: FrontierDigest,
}

/// Metadata describing an execution trace run.
pub struct TraceRunMeta {
    /// The starting tick of the execution trace run.
    pub start_tick: TickId,
    /// The root causal frontier digest.
    pub root_frontier: FrontierDigest,
}

/// A receipt for a sealed trace chunk.
pub struct TraceChunkReceipt {
    /// The starting tick of this chunk.
    pub start_tick: TickId,
    /// The total number of ticks contained in this chunk.
    pub tick_count: u64,
    /// The cryptographic hash of the prior chunk.
    pub prior_chunk_hash: [u8; 32],
    /// The cryptographic hash of the current chunk.
    pub chunk_hash: [u8; 32],
}

/// A receipt for a completed trace run.
pub struct TraceRunReceipt {
    /// The total number of chunks generated in the run.
    pub chunks: usize,
    /// The final cryptographic hash representing the entire trace run.
    pub final_hash: [u8; 32],
}

/// A generic error type for trace operations.
#[derive(Debug)]
pub enum TraceError {
    /// An I/O error occurred while writing or reading the trace sink.
    IoError,
    /// The trace chunk could not be sealed.
    SealError,
    /// A causal boundary violation occurred.
    BoundaryMismatch,
}

/// The trace boundary trait.
///
/// Consumes canonical graph deltas, bounded in chunked streams.
pub trait TraceSink<N, E, A> {
    /// Start a new trace run.
    fn begin_run(&mut self, meta: &TraceRunMeta) -> Result<(), TraceError>;

    /// Append a canonical graph delta to the stream.
    fn append_delta(&mut self, delta: &CanonicalGraphDelta<'_, N, E, A>) -> Result<(), TraceError>;

    /// Seal the current chunk and return its cryptographic receipt.
    fn seal_chunk(&mut self) -> Result<TraceChunkReceipt, TraceError>;

    /// Finish the entire trace run and return the final receipt.
    fn finish_run(&mut self) -> Result<TraceRunReceipt, TraceError>;
}

/// A baseline sink that incurs near-zero overhead and does nothing.
pub struct NullTraceSink;

impl<N, E, A> TraceSink<N, E, A> for NullTraceSink {
    fn begin_run(&mut self, _meta: &TraceRunMeta) -> Result<(), TraceError> {
        Ok(())
    }
    fn append_delta(
        &mut self,
        _delta: &CanonicalGraphDelta<'_, N, E, A>,
    ) -> Result<(), TraceError> {
        Ok(())
    }
    fn seal_chunk(&mut self) -> Result<TraceChunkReceipt, TraceError> {
        Ok(TraceChunkReceipt {
            start_tick: 0,
            tick_count: 0,
            prior_chunk_hash: [0; 32],
            chunk_hash: [0; 32],
        })
    }
    fn finish_run(&mut self) -> Result<TraceRunReceipt, TraceError> {
        Ok(TraceRunReceipt {
            chunks: 0,
            final_hash: [0; 32],
        })
    }
}
