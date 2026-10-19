// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Owner installation, exact retained evidence and fresh-host WAL recovery.
use super::*;
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as V};
use std::sync::atomic::{AtomicU64, Ordering};
use warp_core::edict_effects::InstalledEffectOperationV2;
use warp_core::TrustedRuntimeWalConfig;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct WalDir(std::path::PathBuf);
impl WalDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "echo-v2-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).expect("unique owned WAL directory");
        Self(p)
    }
}
impl Drop for WalDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove owned WAL fixture");
    }
}
fn admitted(host: &TrustedRuntimeHost) -> warp_core::edict_effects::AdmittedEffectPackageV2 {
    let bytes = encoded(&package(&program()));
    host.admit_echo_operation_package_v2(
        &policy(
            &bytes,
            "app.update@1",
            [8; 32],
            EchoOperationBudgetV1::new(1, 0, 0),
        ),
        bytes,
    )
    .expect("admit")
}
#[test]
fn owner_installation_is_idempotent_and_fresh_filesystem_recovery_restores_exact_evidence() {
    let dir = WalDir::new();
    let mut owner = host();
    owner
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
        .expect("activate empty WAL");
    let token = admitted(&owner);
    let installed = owner
        .install_admitted_echo_operation_package_v2(token.clone())
        .expect("install");
    let mut identity_oracle = blake3::Hasher::new();
    identity_oracle.update(b"echo:operation-installation:v2\0");
    identity_oracle.update(&token.package().identity());
    identity_oracle.update(&token.identity());
    identity_oracle.update(&12u64.to_le_bytes());
    identity_oracle.update(b"app.update@1");
    identity_oracle.update(&token.package().program_identity());
    identity_oracle.update(&(token.package().canonical_bytes().len() as u64).to_le_bytes());
    identity_oracle.update(token.package().canonical_bytes());
    assert_eq!(installed.identity(), *identity_oracle.finalize().as_bytes());
    let first_frames = owner.runtime_wal().expect("wal").frames().len();
    assert_eq!(
        owner
            .engine()
            .installed_echo_operation_package_v2(installed.package().identity()),
        Some(&installed)
    );
    assert_eq!(
        owner
            .install_admitted_echo_operation_package_v2(token)
            .expect("idempotent"),
        installed
    );
    assert_eq!(
        owner.runtime_wal().expect("wal").frames().len(),
        first_frames
    );
    let bytes = installed.retained_bytes().expect("retained");
    assert_eq!(
        InstalledEffectOperationV2::from_retained_bytes(&bytes).expect("reconstruct"),
        installed
    );
    let recovered = owner
        .runtime_wal()
        .expect("wal")
        .recover_read_only()
        .expect("recover");
    assert_eq!(
        recovered.installed_echo_effect_operations,
        vec![installed.clone()]
    );
    drop(owner);
    let mut fresh = host();
    fresh
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
        .expect("fresh engine recovery");
    assert_eq!(
        fresh
            .engine()
            .installed_echo_operation_package_v2(installed.package().identity()),
        Some(&installed)
    );
}
#[test]
fn retained_installation_refuses_substituted_package_policy_and_evidence_ids() {
    let mut owner = host();
    let token = admitted(&owner);
    let installed = owner
        .install_admitted_echo_operation_package_v2(token)
        .expect("install");
    let bytes = installed.retained_bytes().expect("bytes");
    for key in [
        "package_id",
        "admission_policy_id",
        "package_admission_id",
        "installed_operation_id",
    ] {
        let mut value = decode(&bytes).expect("canonical");
        set(&mut value, key, id(99));
        assert!(InstalledEffectOperationV2::from_retained_bytes(&encoded(&value)).is_err());
    }
    for schema in [
        "echo.operation-installation/v1",
        "echo.operation-installation/v3",
    ] {
        let mut value = decode(&bytes).expect("canonical");
        set(&mut value, "schema", text(schema));
        assert!(InstalledEffectOperationV2::from_retained_bytes(&encoded(&value)).is_err());
    }
    let mut value = decode(&bytes).expect("canonical");
    set(
        &mut value,
        "package_bytes",
        V::Bytes(encoded(&V::Integer(0))),
    );
    assert!(InstalledEffectOperationV2::from_retained_bytes(&encoded(&value)).is_err());
    let mut value = decode(&bytes).expect("canonical");
    add(&mut value, "extra", V::Integer(0));
    assert!(InstalledEffectOperationV2::from_retained_bytes(&encoded(&value)).is_err());
    let mut value = decode(&bytes).expect("canonical");
    let V::Map(fields) = &mut value else {
        panic!("map")
    };
    let policy = &mut fields
        .iter_mut()
        .find(|(k, _)| k == &text("admission_policy"))
        .expect("policy")
        .1;
    set(policy, "expected_authority_profile_identity", id(99));
    assert!(InstalledEffectOperationV2::from_retained_bytes(&encoded(&value)).is_err());
}
#[test]
fn conflicting_coordinate_or_policy_cannot_replace_an_installed_meaning_or_append_wal() {
    let mut owner = host();
    owner.enable_in_memory_runtime_wal().expect("wal");
    let installed = owner
        .install_admitted_echo_operation_package_v2(admitted(&owner))
        .expect("install");
    let before = owner.runtime_wal().expect("wal").frames().len();
    let mut p = program();
    set(&mut p, "compiled_program", V::Bytes(vec![0xa1]));
    let bytes = encoded(&package(&p));
    let token = owner
        .admit_echo_operation_package_v2(
            &policy(
                &bytes,
                "app.update@1",
                [8; 32],
                EchoOperationBudgetV1::new(1, 0, 0),
            ),
            bytes,
        )
        .expect("new pinned meaning");
    assert!(owner
        .install_admitted_echo_operation_package_v2(token)
        .is_err());
    let bytes = encoded(&package(&program()));
    let token = owner
        .admit_echo_operation_package_v2(
            &policy(
                &bytes,
                "app.update@1",
                [8; 32],
                EchoOperationBudgetV1::new(2, 0, 0),
            ),
            bytes,
        )
        .expect("new policy");
    assert!(owner
        .install_admitted_echo_operation_package_v2(token)
        .is_err());
    assert_eq!(
        owner
            .engine()
            .installed_echo_operation_package_v2(installed.package().identity()),
        Some(&installed)
    );
    assert_eq!(owner.runtime_wal().expect("wal").frames().len(), before);
}
#[test]
fn activating_empty_wal_cannot_attest_an_earlier_volatile_v2_installation() {
    let mut owner = host();
    owner
        .install_admitted_echo_operation_package_v2(admitted(&owner))
        .expect("volatile install");
    assert!(owner.enable_in_memory_runtime_wal().is_err());
}
fn install_v1(
    owner: &mut TrustedRuntimeHost,
    coordinate: &str,
) -> Result<warp_core::InstalledEchoOperationV1, warp_core::TrustedRuntimeHostError> {
    use warp_core::{
        EchoOperationAdmissionPolicyV1, EchoOperationProgramV1, EchoOperationSemanticClosureV1,
        ExecutableOperationPackageV1,
    };
    let budget = EchoOperationBudgetV1::new(16, 4096, 4096);
    let legacy = ExecutableOperationPackageV1::new(
        coordinate,
        "app.LegacyRefusal",
        EchoOperationSemanticClosureV1::new(
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            "app.Schema",
            [5; 32],
            "app.Lawpack",
            [6; 32],
        ),
        warp_core::echo_operation_target_profile_identity_v1(),
        [8; 32],
        budget,
        EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
            make_type_id("legacy-node"),
            make_type_id("legacy-atom"),
            1024,
        ),
    );
    let bytes = legacy.to_canonical_bytes().expect("V1 fixture");
    let policy = EchoOperationAdmissionPolicyV1::exact(
        warp_core::echo_operation_package_id_v1(&bytes),
        coordinate,
        [8; 32],
        budget,
    );
    let admitted = owner
        .admit_echo_operation_package_v1(&policy, bytes)
        .expect("V1 policy");
    owner.install_admitted_echo_operation_package_v1(admitted)
}
#[test]
fn v1_and_v2_share_coordinate_exclusion_in_both_installation_orders() {
    for v2_first in [false, true] {
        let mut owner = host();
        owner.enable_in_memory_runtime_wal().expect("wal");
        if v2_first {
            owner
                .install_admitted_echo_operation_package_v2(admitted(&owner))
                .expect("V2 first");
            let before = owner.runtime_wal().expect("wal").frames().len();
            assert!(install_v1(&mut owner, "app.update@1").is_err());
            assert_eq!(owner.runtime_wal().expect("wal").frames().len(), before);
        } else {
            install_v1(&mut owner, "app.update@1").expect("V1 first");
            let before = owner.runtime_wal().expect("wal").frames().len();
            assert!(owner
                .install_admitted_echo_operation_package_v2(admitted(&owner))
                .is_err());
            assert_eq!(owner.runtime_wal().expect("wal").frames().len(), before);
        }
    }
}
#[test]
fn mixed_catalog_recovers_both_versions_and_certificate_binds_v2_installations() {
    let dir = WalDir::new();
    let mut owner = host();
    owner
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
        .expect("wal");
    let legacy = install_v1(&mut owner, "app.legacy@1").expect("V1");
    let effect = owner
        .install_admitted_echo_operation_package_v2(admitted(&owner))
        .expect("V2");
    let mut recovered = owner
        .runtime_wal()
        .expect("wal")
        .recover_read_only()
        .expect("mixed recovery");
    assert_eq!(recovered.installed_echo_operations, vec![legacy.clone()]);
    assert_eq!(
        recovered.recomputed_indexes_root().expect("root"),
        recovered.certificate.recovered_indexes_root
    );
    recovered.installed_echo_effect_operations.clear();
    assert_ne!(
        recovered.recomputed_indexes_root().expect("altered root"),
        recovered.certificate.recovered_indexes_root
    );
    drop(owner);
    let mut fresh = host();
    fresh
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
        .expect("mixed fresh recovery");
    assert_eq!(
        fresh
            .engine()
            .installed_echo_operation_package_v1(legacy.package_id()),
        Some(&legacy)
    );
    assert_eq!(
        fresh
            .engine()
            .installed_echo_operation_package_v2(effect.package().identity()),
        Some(&effect)
    );
}
#[cfg(feature = "host_test")]
#[test]
fn failed_wal_append_and_commit_do_not_publish_v2_registry_entries() {
    use warp_core::causal_wal::{FilesystemWalFaultPlan, FilesystemWalFaultTarget};
    for target in [
        FilesystemWalFaultTarget::AppendFrame,
        FilesystemWalFaultTarget::FlushCommit,
    ] {
        let dir = WalDir::new();
        let mut owner = host();
        owner
            .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
            .expect("wal");
        let token = admitted(&owner);
        let pin = token.package().identity();
        owner
            .inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(target))
            .expect("fault");
        assert!(owner
            .install_admitted_echo_operation_package_v2(token)
            .is_err());
        assert!(owner
            .engine()
            .installed_echo_operation_package_v2(pin)
            .is_none());
        drop(owner);
        let mut fresh = host();
        fresh
            .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
            .expect("recover incomplete transaction");
        assert!(fresh
            .engine()
            .installed_echo_operation_package_v2(pin)
            .is_none());
    }
}
#[test]
fn self_consistent_retained_hashes_cannot_bypass_independent_policy_readmission() {
    let mut owner = host();
    let token = admitted(&owner);
    let installed = owner
        .install_admitted_echo_operation_package_v2(token.clone())
        .expect("install");
    let mut value = decode(&installed.retained_bytes().expect("bytes")).expect("value");
    let V::Map(fields) = &mut value else {
        panic!("map")
    };
    set(
        &mut fields
            .iter_mut()
            .find(|(k, _)| k == &text("admission_policy"))
            .expect("policy")
            .1,
        "expected_authority_profile_identity",
        id(99),
    );
    let pin = token.package().identity();
    let mut h = blake3::Hasher::new();
    h.update(b"echo:operation-admission-policy:v1\0");
    h.update(&pin);
    h.update(&12u64.to_le_bytes());
    h.update(b"app.update@1");
    h.update(&[99; 32]);
    h.update(&1u64.to_le_bytes());
    h.update(&0u64.to_le_bytes());
    h.update(&0u64.to_le_bytes());
    let policy_id = *h.finalize().as_bytes();
    set(
        &mut value,
        "admission_policy_id",
        V::Bytes(policy_id.to_vec()),
    );
    let mut h = blake3::Hasher::new();
    h.update(b"echo:operation-package-admission:v2\0");
    h.update(&pin);
    h.update(&policy_id);
    let admission_id = *h.finalize().as_bytes();
    set(
        &mut value,
        "package_admission_id",
        V::Bytes(admission_id.to_vec()),
    );
    let mut h = blake3::Hasher::new();
    h.update(b"echo:operation-installation:v2\0");
    h.update(&pin);
    h.update(&admission_id);
    h.update(&12u64.to_le_bytes());
    h.update(b"app.update@1");
    h.update(&token.package().program_identity());
    h.update(&(token.package().canonical_bytes().len() as u64).to_le_bytes());
    h.update(token.package().canonical_bytes());
    set(
        &mut value,
        "installed_operation_id",
        V::Bytes(h.finalize().as_bytes().to_vec()),
    );
    assert!(InstalledEffectOperationV2::from_retained_bytes(&encoded(&value)).is_err());
}
#[cfg(feature = "host_test")]
#[test]
fn readable_unsynced_installation_is_not_published_until_a_fresh_store_reopens() {
    use warp_core::causal_wal::{FilesystemWalFaultPlan, FilesystemWalFaultTarget};
    let dir = WalDir::new();
    let mut owner = host();
    owner
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
        .expect("wal");
    let token = admitted(&owner);
    let pin = token.package().identity();
    let package = token.package().clone();
    owner
        .inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
            FilesystemWalFaultTarget::CommitMarkerSync,
        ))
        .expect("sync fault");
    assert!(owner
        .install_admitted_echo_operation_package_v2(token)
        .is_err());
    assert!(owner
        .engine()
        .installed_echo_operation_package_v2(pin)
        .is_none());
    assert!(
        owner
            .runtime_wal()
            .expect("wal")
            .recover_read_only()
            .is_err(),
        "readable bytes alone cannot attest durability"
    );
    drop(owner);
    let mut fresh = host();
    fresh
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&dir.0))
        .expect("fresh store establishes durable recovered history");
    assert_eq!(
        fresh
            .engine()
            .installed_echo_operation_package_v2(pin)
            .expect("recovered installation")
            .package(),
        &package
    );
}
