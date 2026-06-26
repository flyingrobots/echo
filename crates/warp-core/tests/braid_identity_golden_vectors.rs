// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Golden vectors for young braid/proof identity surfaces.

use warp_core::{
    make_strand_id, AuthorityDomainId, AuthorityDomainRef, BraidMemberRef, BraidShell,
    BraidShellError, BraidShellMember, BraidShellOutcome, CausalPosture, MemberVerdict, OriginId,
    ProofEnvelope, ProofError, ProofKind, ProvenanceRef, WorldlineId, WorldlineTick,
    BRAID_SHELL_VERSION,
};

type Hash = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompatibilityClass {
    PublicStable,
    E1Scaffold,
    TestOnlyFixture,
}

impl CompatibilityClass {
    const fn label(self) -> &'static str {
        match self {
            Self::PublicStable => "public stable identity",
            Self::E1Scaffold => "E1 scaffolding identity",
            Self::TestOnlyFixture => "test-only fixture identity",
        }
    }
}

struct VectorCase {
    name: &'static str,
    compatibility: CompatibilityClass,
    actual: Hash,
    expected_hex: &'static str,
}

fn assert_vector(case: VectorCase) {
    assert_eq!(
        hex::encode(case.actual),
        case.expected_hex,
        "{} [{}] drifted",
        case.name,
        case.compatibility.label()
    );
}

fn wl(byte: u8) -> WorldlineId {
    WorldlineId::from_bytes([byte; 32])
}

fn authority(origin: u8, domain: u8) -> AuthorityDomainRef {
    AuthorityDomainRef::new(
        OriginId::from_bytes([origin; 32]),
        AuthorityDomainId::from_bytes([domain; 32]),
    )
}

fn basis_ref() -> ProvenanceRef {
    ProvenanceRef {
        worldline_id: wl(0x41),
        worldline_tick: WorldlineTick::from_raw(7),
        commit_hash: [0x42; 32],
    }
}

fn revealed_member(label: &str, verdict: MemberVerdict, claim_byte: u8) -> BraidShellMember {
    BraidShellMember {
        member_ref: BraidMemberRef::Revealed(make_strand_id(label)),
        support_pin_digest: [0x21; 32],
        basis_digest: [0x22; 32],
        frontier_digest: [0x23; 32],
        footprint_digest: [0x24; 32],
        claim_digest: [claim_byte; 32],
        verdict,
        verdict_digest: [0x26; 32],
        posture: CausalPosture::AuthorOnly,
    }
}

fn sealed_member(
    blinded_commitment: Hash,
    authority: AuthorityDomainRef,
    verdict: MemberVerdict,
    claim_byte: u8,
) -> BraidShellMember {
    BraidShellMember {
        member_ref: BraidMemberRef::Sealed {
            blinded_commitment,
            authority,
        },
        support_pin_digest: [0x21; 32],
        basis_digest: [0x22; 32],
        frontier_digest: [0x23; 32],
        footprint_digest: [0x24; 32],
        claim_digest: [claim_byte; 32],
        verdict,
        verdict_digest: [0x26; 32],
        posture: CausalPosture::AuthorOnly,
    }
}

fn plural_outcome() -> BraidShellOutcome {
    BraidShellOutcome::Plural {
        alternative_ids: vec![[0x31; 32], [0x32; 32]],
    }
}

#[test]
fn compatibility_classes_are_explicit_vector_metadata() {
    let classes = [
        CompatibilityClass::PublicStable,
        CompatibilityClass::E1Scaffold,
        CompatibilityClass::TestOnlyFixture,
    ];

    assert_eq!(classes[0].label(), "public stable identity");
    assert_eq!(classes[1].label(), "E1 scaffolding identity");
    assert_eq!(classes[2].label(), "test-only fixture identity");
}

#[test]
fn replay_trace_proof_envelope_digest_vector_is_locked() {
    let expected_public_inputs = [0x44; 32];
    let proof = ProofEnvelope {
        kind: ProofKind::ReplayTrace,
        proof_bytes: b"gp2/replay-trace/evidence".to_vec(),
        public_inputs_hash: expected_public_inputs,
    };

    assert_eq!(proof.validate_shape(expected_public_inputs), Ok(()));
    assert_vector(VectorCase {
        name: "proof-envelope/replay-trace",
        compatibility: CompatibilityClass::E1Scaffold,
        actual: proof.digest(),
        expected_hex: "c4dc4d862a493cde6d8b62a83079463da56505358261fb3e7e7d190d6370f8c0",
    });

    for kind in [ProofKind::ZkSnark, ProofKind::VectorOpening] {
        let unsupported = ProofEnvelope {
            kind,
            proof_bytes: b"reserved-proof-kind".to_vec(),
            public_inputs_hash: expected_public_inputs,
        };

        assert_eq!(
            unsupported.validate_shape(expected_public_inputs),
            Err(ProofError::UnsupportedKind { kind })
        );
    }
}

#[test]
fn proofless_and_proof_bearing_shell_digest_vectors_are_locked() -> Result<(), BraidShellError> {
    let members = vec![
        revealed_member("gp2/member-a", MemberVerdict::Plural, 0x25),
        revealed_member("gp2/member-b", MemberVerdict::Derived, 0x35),
    ];
    let proofless_shell = BraidShell::assemble(
        wl(0x40),
        basis_ref(),
        members.clone(),
        [0x5E; 32],
        plural_outcome(),
        CausalPosture::AuthorOnly,
    )?;
    let proof = ProofEnvelope {
        kind: ProofKind::ReplayTrace,
        proof_bytes: b"gp2/shell/replay-trace".to_vec(),
        public_inputs_hash: proofless_shell.witness_digest,
    };
    let proof_digest = proof.digest();
    let proof_bearing_shell = BraidShell::assemble_with_proof(
        wl(0x40),
        basis_ref(),
        members,
        [0x5E; 32],
        plural_outcome(),
        CausalPosture::AuthorOnly,
        Some(proof),
    )?;

    assert_eq!(proofless_shell.version, BRAID_SHELL_VERSION);
    assert_eq!(proof_bearing_shell.version, BRAID_SHELL_VERSION);
    assert_ne!(proofless_shell.digest, proof_bearing_shell.digest);
    assert_vector(VectorCase {
        name: "braid-shell/proofless",
        compatibility: CompatibilityClass::E1Scaffold,
        actual: proofless_shell.digest,
        expected_hex: "61acd40a01a32ef2771b7a98d11b3d6734e1064ca19393dba7b8dd8e40195d37",
    });
    assert_vector(VectorCase {
        name: "proof-envelope/shell-replay-trace",
        compatibility: CompatibilityClass::E1Scaffold,
        actual: proof_digest,
        expected_hex: "c1c6e5ef0a01cd29e8230124b42868045b69c474a33cca7f4b6a7fb24ace4c4c",
    });
    assert_vector(VectorCase {
        name: "braid-shell/proof-bearing",
        compatibility: CompatibilityClass::E1Scaffold,
        actual: proof_bearing_shell.digest,
        expected_hex: "7bd76a3efbcbd944ce3b1b52f09cc7bbb9a5fae633f65ad361a3376d8a9cbbbe",
    });

    Ok(())
}

#[test]
fn revealed_and_sealed_member_reference_vectors_are_locked() {
    let revealed = revealed_member("gp2/revealed-member", MemberVerdict::Plural, 0x25);
    let strand_id = make_strand_id("gp2/sealed-member");
    let child_worldline_id = wl(0x88);
    let member_authority = authority(0xA1, 0xB1);
    let first_blinding = [0xA5; 32];
    let second_blinding = [0xB6; 32];
    let first_commitment = BraidMemberRef::seal(strand_id, child_worldline_id, first_blinding);
    let second_commitment = BraidMemberRef::seal(strand_id, child_worldline_id, second_blinding);
    let sealed = sealed_member(
        first_commitment,
        member_authority,
        MemberVerdict::Plural,
        0x27,
    );

    assert_ne!(first_commitment, second_commitment);
    assert_vector(VectorCase {
        name: "braid-member/revealed-member-digest",
        compatibility: CompatibilityClass::E1Scaffold,
        actual: revealed.member_digest(),
        expected_hex: "f11e5fdea68591570c852b2119d3f1e57d63959e0ea3412a0a5e3a223fc4cf07",
    });
    assert_vector(VectorCase {
        name: "braid-member/sealed-commitment/first-blinding",
        compatibility: CompatibilityClass::E1Scaffold,
        actual: first_commitment,
        expected_hex: "7a34ac5e0f683028d96c75a495f41629124cb5064a29dcc53dc8f8605ab408e4",
    });
    assert_vector(VectorCase {
        name: "braid-member/sealed-commitment/second-blinding",
        compatibility: CompatibilityClass::E1Scaffold,
        actual: second_commitment,
        expected_hex: "62b203a994819eeac54fed71360728487e4960ce38ea19cf1b0ea0dea8ab3d0d",
    });
    assert_vector(VectorCase {
        name: "braid-member/sealed-member-digest",
        compatibility: CompatibilityClass::E1Scaffold,
        actual: sealed.member_digest(),
        expected_hex: "6af9058f530b0edc611cee34f716f74d2df32c2e4307020a44d0b30c1ae75ee7",
    });
}
