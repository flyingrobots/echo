// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Versioned effect installation evidence; invocation authority remains separate.
use super::package::hash;
use crate::echo_operation::{
    installation_error, invalid_structure, map_value, recover_installation_v1,
};
use crate::edict_pure::values::{bytes, exact_fields, field, require_text, text};
use crate::AdmittedExecutableOperationPackage;
use crate::EchoOperationAdmissionPolicy;
use crate::ExecutableOperationPackage;
use crate::{
    EchoOperationArtifactErrorV1, EchoOperationInstallationErrorKindV1,
    EchoOperationInstallationErrorV1, EchoOperationPackageIdV1, InstalledEchoOperationV1,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode, CanonicalValueV1 as V,
};
use std::collections::BTreeMap;
/// Exact installation evidence; registry membership remains runtime-owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledEffectOperationV2 {
    admitted: AdmittedExecutableOperationPackage,
    id: [u8; 32],
}
impl InstalledEffectOperationV2 {
    /// Identity of the current package definition.
    #[must_use]
    pub const fn package_identity(&self) -> [u8; 32] {
        self.admitted.package_identity()
    }
    /// Identity of the current program definition.
    #[must_use]
    pub const fn program_identity(&self) -> [u8; 32] {
        self.admitted.program_identity()
    }
    /// Exact current publication bytes retained by installation.
    #[must_use]
    pub fn canonical_package_bytes(&self) -> &[u8] {
        self.admitted.canonical_package_bytes()
    }

    pub(crate) fn from_admitted(
        admitted: AdmittedExecutableOperationPackage,
    ) -> Result<Self, EchoOperationArtifactErrorV1> {
        if admitted.package().program().as_effect_program().is_none() {
            return Err(invalid_structure(
                "effect catalog requires an authored-effect program",
            ));
        }
        let package = admitted.package();
        let mut h = blake3::Hasher::new();
        h.update(b"echo:operation-installation:v2\0");
        h.update(&admitted.package_identity());
        h.update(&admitted.identity());
        h.update(&(package.operation_coordinate().len() as u64).to_le_bytes());
        h.update(package.operation_coordinate().as_bytes());
        h.update(&admitted.program_identity());
        h.update(&(admitted.canonical_package_bytes().len() as u64).to_le_bytes());
        h.update(admitted.canonical_package_bytes());
        Ok(Self {
            admitted,
            id: h.finalize().into(),
        })
    }
    /// Exact publication named by this installation.
    #[must_use]
    pub const fn package(&self) -> &ExecutableOperationPackage {
        self.admitted.package()
    }
    /// Versioned installation identity, without invocation rights.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.id
    }
    /// Canonical retained installation evidence.
    /// # Errors
    /// Refuses invalid canonical material.
    pub fn retained_bytes(&self) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
        encode(&map_value([
            ("schema", V::Text("echo.operation-installation/v2".into())),
            (
                "admission_policy",
                self.admitted.admission_policy().to_value(),
            ),
            (
                "admission_policy_id",
                V::Bytes(self.admitted.admission_policy_identity().to_vec()),
            ),
            ("package_id", V::Bytes(self.package_identity().to_vec())),
            (
                "package_bytes",
                V::Bytes(self.canonical_package_bytes().to_vec()),
            ),
            (
                "package_admission_id",
                V::Bytes(self.admitted.identity().to_vec()),
            ),
            ("installed_operation_id", V::Bytes(self.id.to_vec())),
        ]))
        .map_err(|_| invalid_structure("invalid V2 installation encoding"))
    }
    /// Reconstructs exact retained evidence; this does not install it in an engine.
    /// # Errors
    /// Refuses altered policy, publication, identity or version evidence.
    pub fn from_retained_bytes(canonical: &[u8]) -> Result<Self, EchoOperationArtifactErrorV1> {
        let value =
            decode(canonical).map_err(|_| invalid_structure("invalid V2 retained bytes"))?;
        let read = || {
            exact_fields(
                &value,
                &[
                    "schema",
                    "admission_policy",
                    "admission_policy_id",
                    "package_id",
                    "package_bytes",
                    "package_admission_id",
                    "installed_operation_id",
                ],
            )?;
            require_text(&value, "schema", "echo.operation-installation/v2")?;
            Ok::<_, crate::edict_pure::EvaluationError>((
                field(&value, "admission_policy")?,
                hash(field(&value, "admission_policy_id")?)?,
                hash(field(&value, "package_id")?)?,
                bytes(field(&value, "package_bytes")?)?,
                hash(field(&value, "package_admission_id")?)?,
                hash(field(&value, "installed_operation_id")?)?,
            ))
        };
        let (policy, policy_id, package_id, package_bytes, admission_id, installed_id) =
            read().map_err(|_| invalid_structure("invalid V2 installation fields"))?;
        let policy = EchoOperationAdmissionPolicy::from_value(policy.clone())?;
        let admitted =
            crate::echo_operation::admit_operation_package(&policy, package_bytes.to_vec())
                .map_err(|_| invalid_structure("V2 retained package no longer admits"))?;
        if policy.identity() != policy_id
            || admitted.package_identity() != package_id
            || admitted.identity() != admission_id
        {
            return Err(invalid_structure("V2 retained admission identities differ"));
        }
        let installed = Self::from_admitted(admitted)?;
        if installed.id != installed_id || installed.retained_bytes()? != canonical {
            return Err(invalid_structure(
                "V2 retained installation identity differs",
            ));
        }
        Ok(installed)
    }
}
pub(crate) enum RecoveredOperationInstallation {
    V1(Box<InstalledEchoOperationV1>),
    V2(Box<InstalledEffectOperationV2>),
}
impl RecoveredOperationInstallation {
    pub(crate) fn package_identity(&self) -> [u8; 32] {
        match self {
            Self::V1(v) => v.package_id().as_hash(),
            Self::V2(v) => v.package_identity(),
        }
    }
}
pub(crate) fn recover_versioned_installation(
    canonical: &[u8],
) -> Result<RecoveredOperationInstallation, EchoOperationArtifactErrorV1> {
    let value =
        decode(canonical).map_err(|_| invalid_structure("invalid retained installation"))?;
    match field(&value, "schema")
        .and_then(text)
        .map_err(|_| invalid_structure("missing installation version"))?
    {
        "echo.operation-installation/v1" => recover_installation_v1(canonical)
            .map(|installed| RecoveredOperationInstallation::V1(Box::new(installed))),
        "echo.operation-installation/v2" => {
            InstalledEffectOperationV2::from_retained_bytes(canonical)
                .map(|installed| RecoveredOperationInstallation::V2(Box::new(installed)))
        }
        _ => Err(invalid_structure("unsupported installation version")),
    }
}
pub(crate) fn restore_effect_catalog_entry(
    packages: &mut BTreeMap<[u8; 32], InstalledEffectOperationV2>,
    operations: &mut BTreeMap<String, [u8; 32]>,
    v1_operations: &BTreeMap<String, EchoOperationPackageIdV1>,
    installed: InstalledEffectOperationV2,
) -> Result<(), EchoOperationInstallationErrorV1> {
    let pin = installed.package_identity();
    let coordinate = installed.package().operation_coordinate();
    if let Some(existing) = packages.get(&pin) {
        if existing == &installed {
            return Ok(());
        }
        return Err(installation_error(
            EchoOperationInstallationErrorKindV1::PackageIdentityConflict,
            "V2 installation policy conflicts with existing package",
        ));
    }
    if v1_operations.contains_key(coordinate)
        || operations.get(coordinate).is_some_and(|id| id != &pin)
    {
        return Err(installation_error(
            EchoOperationInstallationErrorKindV1::OperationCoordinateConflict,
            "V2 installation operation coordinate already exists",
        ));
    }
    operations.insert(coordinate.to_owned(), pin);
    packages.insert(pin, installed);
    Ok(())
}
