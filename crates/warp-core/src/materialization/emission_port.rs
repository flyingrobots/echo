// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Driven port for rule emissions (hexagonal architecture).
//!
//! Rules emit to channels via the [`EmissionPort`] trait. The engine provides
//! a scoped implementation ([`ScopedEmitter`]) that automatically constructs
//! [`EmitKey`]s from execution context, preventing rules from forging keys.
//!
//! # Hexagonal Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              Rule Executor              │
//! │         (depends on trait only)         │
//! └─────────────────────────────────────────┘
//!                       │
//!                       ▼ EmissionPort
//!              ┌────────────────────┐
//!              │   ScopedEmitter    │  ← Adapter
//!              └────────────────────┘
//!                       │
//!                       ▼
//!              ┌────────────────────┐
//!              │ MaterializationBus │  ← Internal
//!              └────────────────────┘
//! ```
//!
//! This separation allows rules to be tested with mock ports while the engine
//! uses the real bus.

use super::bus::DuplicateEmission;
use super::channel::ChannelId;

/// Driven port for rule emissions.
///
/// Rules emit to channels via this trait. The implementation handles
/// [`EmitKey`](super::EmitKey) construction—callers only provide channel and payload.
///
/// # Error Handling
///
/// All methods return `Result<(), DuplicateEmission>`. Duplicate emissions
/// (same channel + same derived [`EmitKey`](super::EmitKey)) are structural
/// errors that indicate bugs in the rule (e.g., iterating a `HashMap` without
/// proper subkey differentiation).
pub trait EmissionPort {
    /// Emit data to a channel.
    ///
    /// The implementation handles [`EmitKey`](super::EmitKey) construction from
    /// execution context.
    /// Callers only provide channel and payload.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateEmission`] if this rule has already emitted to
    /// this channel with the same derived key (subkey = 0).
    fn emit(&self, channel: ChannelId, data: Vec<u8>) -> Result<(), DuplicateEmission>;

    /// Emit with explicit subkey (for multi-emission rules).
    ///
    /// Use when a single rule invocation needs to emit multiple values to
    /// the same channel. The subkey disambiguates emissions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Rule that emits multiple particles
    /// for (i, particle) in particles.iter().enumerate() {
    ///     port.emit_with_subkey(channel, i as u32, particle.encode())?;
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateEmission`] if this `(channel, subkey)` pair has
    /// already been emitted by this rule.
    fn emit_with_subkey(
        &self,
        channel: ChannelId,
        subkey: u32,
        data: Vec<u8>,
    ) -> Result<(), DuplicateEmission>;
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Mock port for testing rules in isolation.
    struct MockEmissionPort {
        emissions: RefCell<Vec<(ChannelId, u32, Vec<u8>)>>,
    }

    impl MockEmissionPort {
        fn new() -> Self {
            Self {
                emissions: RefCell::new(Vec::new()),
            }
        }

        #[allow(dead_code)]
        fn emissions(&self) -> Vec<(ChannelId, u32, Vec<u8>)> {
            self.emissions.borrow().clone()
        }
    }

    impl EmissionPort for MockEmissionPort {
        fn emit(&self, channel: ChannelId, data: Vec<u8>) -> Result<(), DuplicateEmission> {
            self.emit_with_subkey(channel, 0, data)
        }

        fn emit_with_subkey(
            &self,
            channel: ChannelId,
            subkey: u32,
            data: Vec<u8>,
        ) -> Result<(), DuplicateEmission> {
            self.emissions.borrow_mut().push((channel, subkey, data));
            Ok(())
        }
    }

    #[test]
    fn mock_port_collects_emissions() {
        use super::super::channel::make_channel_id;

        let port = MockEmissionPort::new();
        let ch = make_channel_id("test:mock");

        port.emit(ch, vec![1, 2, 3]).expect("emit");
        port.emit_with_subkey(ch, 42, vec![4, 5]).expect("emit");

        let emissions = port.emissions();
        assert_eq!(emissions.len(), 2);
        assert_eq!(emissions[0], (ch, 0, vec![1, 2, 3]));
        assert_eq!(emissions[1], (ch, 42, vec![4, 5]));
    }
}
