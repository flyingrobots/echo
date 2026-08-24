// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Sealed host facade for the Echo causal runtime.
//!
//! The facade owns trusted runtime construction and WAL activation without
//! exposing native rule registration or constructors for receipts, authority
//! epochs, or committed outcomes. It remains `publish = false` while the
//! release boundary is qualified.
//!
//! Runtime internals are not part of this facade:
//!
//! ```compile_fail
//! use echo_runtime::{RewriteRule, TrustedRuntimeHost};
//! ```

use std::path::{Path, PathBuf};

use blake3::Hasher;
use thiserror::Error;
use warp_core::{
    make_head_id, make_node_id, make_type_id, make_warp_id, EngineBuilder, GraphStore, InboxPolicy,
    NodeRecord, PlaybackMode, SchedulerKind, TrustedRuntimeHost, TrustedRuntimeWalConfig,
    WorldlineId, WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

/// Stable failure category for opening a local Echo runtime host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeHostErrorKind {
    /// The supplied configuration is not a valid bounded host description.
    InvalidConfiguration,
    /// The initial worldline could not be constructed or registered.
    WorldlineBootstrap,
    /// The trusted runtime host could not be constructed.
    HostConstruction,
    /// The configured WAL could not be activated or recovered.
    WalActivation,
}

/// Failure to construct or recover a sealed local Echo runtime host.
#[derive(Debug, Error)]
#[error("{kind:?}: {detail}")]
pub struct RuntimeHostError {
    kind: RuntimeHostErrorKind,
    detail: String,
}

impl RuntimeHostError {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> RuntimeHostErrorKind {
        self.kind
    }
}

/// Persistence posture for a local runtime host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeWal {
    /// Ephemeral WAL retained only for the life of this process.
    InMemory,
    /// Filesystem WAL rooted at the supplied host-owned directory.
    Filesystem(PathBuf),
}

impl RuntimeWal {
    /// Selects a filesystem-backed WAL directory.
    #[must_use]
    pub fn filesystem(path: impl AsRef<Path>) -> Self {
        Self::Filesystem(path.as_ref().to_path_buf())
    }
}

/// Explicit bootstrap description for one local runtime worldline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeHostConfig {
    basis_label: String,
    root_type_coordinate: String,
    wal: RuntimeWal,
}

impl RuntimeHostConfig {
    /// Creates one bounded local runtime configuration.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeHostErrorKind::InvalidConfiguration`] when either
    /// identity-bearing string is empty or exceeds 1,024 UTF-8 bytes.
    pub fn new(
        basis_label: impl Into<String>,
        root_type_coordinate: impl Into<String>,
        wal: RuntimeWal,
    ) -> Result<Self, RuntimeHostError> {
        let basis_label = basis_label.into();
        let root_type_coordinate = root_type_coordinate.into();
        for (name, value) in [
            ("basis label", basis_label.as_str()),
            ("root type coordinate", root_type_coordinate.as_str()),
        ] {
            if value.is_empty() || value.len() > 1_024 {
                return Err(error(
                    RuntimeHostErrorKind::InvalidConfiguration,
                    format!("{name} must contain 1..=1024 UTF-8 bytes"),
                ));
            }
        }
        Ok(Self {
            basis_label,
            root_type_coordinate,
            wal,
        })
    }
}

/// Read-only identity and state evidence for one opened host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeHostSnapshot {
    worldline_id: [u8; 32],
    state_root: [u8; 32],
    pending_submission_count: usize,
}

impl RuntimeHostSnapshot {
    /// Returns the authoritative worldline identity.
    #[must_use]
    pub const fn worldline_id(&self) -> [u8; 32] {
        self.worldline_id
    }

    /// Returns the current worldline state root.
    #[must_use]
    pub const fn state_root(&self) -> [u8; 32] {
        self.state_root
    }

    /// Returns the count of accepted submissions awaiting settlement.
    #[must_use]
    pub const fn pending_submission_count(&self) -> usize {
        self.pending_submission_count
    }
}

/// Sealed trusted runtime host.
///
/// This wrapper intentionally exposes neither the underlying engine nor raw
/// runtime-owner constructors.
pub struct RuntimeHost {
    inner: TrustedRuntimeHost,
    worldline_id: WorldlineId,
}

impl RuntimeHost {
    /// Constructs a host, activates its WAL, and recovers retained runtime
    /// history before returning.
    ///
    /// # Errors
    ///
    /// Returns a typed host error if bootstrap, construction, or WAL activation
    /// fails.
    pub fn open(config: RuntimeHostConfig) -> Result<Self, RuntimeHostError> {
        let lane_label = format!("echo:public-runtime:{}", config.basis_label);
        let warp_id = make_warp_id(&lane_label);
        let root_id = make_node_id(&format!("{lane_label}:root"));
        let mut store = GraphStore::new(warp_id);
        store.insert_node(
            root_id,
            NodeRecord {
                ty: make_type_id(&config.root_type_coordinate),
            },
        );
        let state = WorldlineState::from_root_store(store, root_id).map_err(|failure| {
            error(
                RuntimeHostErrorKind::WorldlineBootstrap,
                failure.to_string(),
            )
        })?;
        let worldline_id = WorldlineId::from_bytes(domain_hash(
            b"echo:public-runtime-worldline:v1\0",
            config.basis_label.as_bytes(),
        ));
        let head = WriterHeadKey {
            worldline_id,
            head_id: make_head_id(&format!("{lane_label}:head")),
        };
        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(worldline_id, state)
            .map_err(|failure| {
                error(
                    RuntimeHostErrorKind::WorldlineBootstrap,
                    failure.to_string(),
                )
            })?;
        runtime
            .register_writer_head(WriterHead::with_routing(
                head,
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .map_err(|failure| {
                error(
                    RuntimeHostErrorKind::WorldlineBootstrap,
                    failure.to_string(),
                )
            })?;

        let mut engine_store = GraphStore::default();
        let engine_root = make_node_id("echo:public-runtime-engine-root:v1");
        engine_store.insert_node(
            engine_root,
            NodeRecord {
                ty: make_type_id("echo.public-runtime.engine-root/v1"),
            },
        );
        let engine = EngineBuilder::new(engine_store, engine_root)
            .scheduler(SchedulerKind::Radix)
            .workers(1)
            .build();
        let mut inner = TrustedRuntimeHost::new(runtime, engine).map_err(|failure| {
            error(RuntimeHostErrorKind::HostConstruction, failure.to_string())
        })?;
        let wal = match config.wal {
            RuntimeWal::InMemory => TrustedRuntimeWalConfig::in_memory(),
            RuntimeWal::Filesystem(root) => TrustedRuntimeWalConfig::filesystem(root),
        };
        inner
            .enable_runtime_wal(wal)
            .map_err(|failure| error(RuntimeHostErrorKind::WalActivation, failure.to_string()))?;
        Ok(Self {
            inner,
            worldline_id,
        })
    }

    /// Returns read-only identity and state evidence for the opened host.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeHostErrorKind::WorldlineBootstrap`] if the configured
    /// worldline is no longer available.
    pub fn snapshot(&self) -> Result<RuntimeHostSnapshot, RuntimeHostError> {
        let frontier = self
            .inner
            .runtime()
            .worldlines()
            .get(&self.worldline_id)
            .ok_or_else(|| {
                error(
                    RuntimeHostErrorKind::WorldlineBootstrap,
                    "configured worldline is unavailable",
                )
            })?;
        Ok(RuntimeHostSnapshot {
            worldline_id: *self.worldline_id.as_bytes(),
            state_root: frontier.state().state_root(),
            pending_submission_count: self.inner.runtime().pending_witnessed_submission_count(),
        })
    }
}

fn error(kind: RuntimeHostErrorKind, detail: impl Into<String>) -> RuntimeHostError {
    RuntimeHostError {
        kind,
        detail: detail.into(),
    }
}

fn domain_hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}
