<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-alloc-witness

Allocation witnesses for Echo's hot paths. A counting `#[global_allocator]` is
a `static`, and Echo's runtime crates admit no global state (`ban-globals`), so
these tests live in this test-only crate, which nothing in the runtime depends
on.

Each file in `tests/` is one binary with its own allocator and one
measurement. A witness states an unwanted scaling, such as commit cost growing
with untouched graph size, as a test that fails on the code that scaled and
passes on the fix. See #903.

Run them with `cargo test -p echo-alloc-witness`.
