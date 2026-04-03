<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RED/GREEN can't be separate commits

Clippy denies `todo!()` and `unimplemented!()` in production code.
This means the RED phase (failing tests with stub implementations)
can't be committed separately from the GREEN phase (real code).

The discipline is preserved (tests are written first) but the git
history doesn't show it. Options:

- Allow `todo!()` in a `#[cfg(test)]`-gated stub module
- Use a `method-dev` clippy profile that allows stubs
- Accept it as a documentation-only friction (retro notes it)
- Use `unreachable!("not yet implemented")` which clippy allows
  (but is semantically wrong)

Not urgent — the retro documents it — but worth resolving if it
keeps coming up.
