#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
S=/private/tmp/claude-501/-Users-j-git-reader/a1c895b9-b432-44d0-8f03-86a8cf7ca98f/scratchpad
WT=/Users/j/git/echo-claude-stack; R=$S/m0e1/run.sh
$R $WT stack-fmt fmt --all -- --check
$R $WT stack-clippy clippy --locked -p warp-core -p warp-benches --all-targets --features warp-core/native_rule_bootstrap,warp-core/trusted_runtime,warp-core/host_test,warp-core/delta_validate,warp-core/application_root_profile -- -D warnings
$R $WT stack-clippy-legacy clippy --locked -p warp-core --lib -- -D warnings -D missing_docs
cd $WT && scripts/check_spdx.sh --check --all > /dev/null && echo "SPDX ok"
python3 scripts/check-feature-gated-tests.py | tail -1
