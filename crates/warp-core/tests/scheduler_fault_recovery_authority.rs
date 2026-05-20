// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Trusted-runtime API witness for scheduler fault recovery authority.

use warp_core::SchedulerFaultRecoveryAuthority;

#[test]
fn trusted_runtime_feature_exposes_scheduler_fault_recovery_authority() {
    let _authority = SchedulerFaultRecoveryAuthority::assume_runtime_owner();
}
