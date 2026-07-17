// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-MSRV compile witness for the checked generated provider helper.
#![allow(clippy::expect_used)]
#![allow(dead_code)]

#[rustfmt::skip]
#[path = "../../echo-edict-provider-lowerer/tests/fixtures/generated_echo_dpo.rs"]
mod checked_generated_helper;

use checked_generated_helper::echo_dpo as generated;

#[test]
fn checked_generated_provider_helper_compiles_in_echo_rust_1_90_graph() {
    let operation_id: u32 = generated::OPERATION_ID;
    assert_eq!(operation_id, 3_389_142_194);
    let input = generated::Input::new("echo-msrv").expect("fixture input is within its bound");
    assert_eq!(input.id(), "echo-msrv");
}
