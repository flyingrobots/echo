<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Method drift check as pre-push hook

Once `cargo xtask method drift` exists, wire it into
`scripts/hooks/pre-push` so playback questions are checked against
test descriptions before code leaves the machine.
