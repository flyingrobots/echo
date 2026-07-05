// SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0

//! Echo Trace
//! 
//! Trace owns observation. It provides the bounded, append-only, canonical stream
//! of WSC-derived trace events.

/// Represents an execution tick identifier.
pub type TickId = u64;

/// Represents a sparse boolean control column identifier.
pub type SelectorId = u32;

/// A placeholder for the canonical WSC delta event.
pub struct CanonicalWscDelta {
    // To be implemented.
}

/// Metadata describing an execution trace run.
pub struct TraceRunMeta {
    /// The starting tick of the execution trace run.
    pub start_tick: TickId,
    // Add additional metadata (e.g. root hash) as needed.
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
    /// The tick recorded does not match the expected contiguous sequence.
    TickMismatch,
}

/// The trace boundary trait.
/// 
/// Echo execution emits trace events into this sink. It does not care whether
/// the sink is a human-readable JSON file, a canonical chunked stream, or a 
/// sparse Roaring-encoded accelerator.
pub trait TraceSink {
    /// Start a new trace run.
    fn begin_run(&mut self, meta: &TraceRunMeta) -> Result<(), TraceError>;

    /// Begin a new execution tick.
    fn begin_tick(&mut self, tick: TickId) -> Result<(), TraceError>;

    /// Record a canonical WSC delta within the current tick.
    fn record_wsc_delta(&mut self, delta: &CanonicalWscDelta) -> Result<(), TraceError>;

    /// Record that a specific sparse selector was activated at this tick.
    fn record_selector(&mut self, selector: SelectorId, tick: TickId) -> Result<(), TraceError>;

    /// End the current execution tick.
    fn end_tick(&mut self, tick: TickId) -> Result<(), TraceError>;

    /// Seal the current chunk and return its cryptographic receipt.
    fn seal_chunk(&mut self) -> Result<TraceChunkReceipt, TraceError>;

    /// Finish the entire trace run and return the final receipt.
    fn finish_run(&mut self) -> Result<TraceRunReceipt, TraceError>;
}

/// A baseline sink that incurs near-zero overhead and does nothing.
pub struct NullTraceSink;

impl TraceSink for NullTraceSink {
    fn begin_run(&mut self, _meta: &TraceRunMeta) -> Result<(), TraceError> { Ok(()) }
    fn begin_tick(&mut self, _tick: TickId) -> Result<(), TraceError> { Ok(()) }
    fn record_wsc_delta(&mut self, _delta: &CanonicalWscDelta) -> Result<(), TraceError> { Ok(()) }
    fn record_selector(&mut self, _selector: SelectorId, _tick: TickId) -> Result<(), TraceError> { Ok(()) }
    fn end_tick(&mut self, _tick: TickId) -> Result<(), TraceError> { Ok(()) }
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
