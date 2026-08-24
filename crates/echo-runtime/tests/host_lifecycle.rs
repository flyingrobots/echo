// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! External-style lifecycle witnesses for the sealed runtime facade.

use echo_runtime::{RuntimeHost, RuntimeHostConfig, RuntimeHostErrorKind, RuntimeWal};

#[test]
fn sealed_host_opens_without_native_rule_bootstrap() -> Result<(), Box<dyn std::error::Error>> {
    let config = RuntimeHostConfig::new(
        "public-facade-test",
        "examples.public-facade-root/v1",
        RuntimeWal::InMemory,
    )?;
    let host = RuntimeHost::open(config)?;
    let snapshot = host.snapshot()?;

    assert_ne!(snapshot.worldline_id(), [0; 32]);
    assert_ne!(snapshot.state_root(), [0; 32]);
    assert_eq!(snapshot.pending_submission_count(), 0);
    Ok(())
}

#[test]
fn invalid_configuration_fails_before_host_construction() {
    let outcome = RuntimeHostConfig::new("", "examples.root/v1", RuntimeWal::InMemory);

    assert!(matches!(
        outcome,
        Err(failure) if failure.kind() == RuntimeHostErrorKind::InvalidConfiguration
    ));
}

#[test]
fn filesystem_host_reopens_the_same_genesis_state() -> Result<(), Box<dyn std::error::Error>> {
    let wal = tempfile::tempdir()?;
    let config = RuntimeHostConfig::new(
        "filesystem-recovery-test",
        "examples.filesystem-root/v1",
        RuntimeWal::filesystem(wal.path()),
    )?;

    let first = RuntimeHost::open(config.clone()).and_then(|host| host.snapshot())?;
    let recovered = RuntimeHost::open(config).and_then(|host| host.snapshot())?;

    assert_eq!(recovered, first);
    Ok(())
}
