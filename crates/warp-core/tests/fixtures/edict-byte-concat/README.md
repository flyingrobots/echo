<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-produced bounded byte concatenation

Exact source, twelve literal expectations, executable package and separate
accepted report come from Jim evidence commit
`327ac11c71cc5a9a33c5552be7a6db533bf88f5d`, under
`edict/replace-range-probes/range-assembly/`. Source first appeared at
`e9ac7433c12d995743c109fd3cf6bcec46f9bca7` and remains unchanged.

Compiler: Edict `01dc5abb9a8a74f8fd8b0e0a6d1041d0adb86d6e`, tree
`74e571125a1da6ac5b2eac80944e343811708d51`, verified 449-file manifest;
CLI SHA-256 `e2700698829b03f437c66425ed9878b002ef7a34ac183a39bac4eaf6a758cc6d`.
Provider: Echo `49e9efb68001dfd78563d18bac9359a87671e431`, manifest SHA-256
`c5b9fb2fe3a0dc4dad282621a97413225c555be0071f3502b3272952069d42dc`.

| Artifact | Raw-byte SHA-256 |
| --- | --- |
| Executable package | `e889d4680435139fe76f45762f0529c090afcf3d73ef7d17f787d44a49bda534` |
| Verification report | `7eec90854e0663aa2346ec5005ff7d05eeb50fc2229b32b407dda3cfa0280077` |

The report binds domain-separated package digest
`ed480a332a9d5f84f7425e46ab0b08cca886c8a040e048145b88fba1e6e0415f`.
Hex files decode to the exact retained bytes. Their producer versions describe
historical public build evidence; package acceptance alone does not prove
runtime primitive support. The consumer test runs the compiled operation with
literal expected outputs and two input-bound refusals. It checks generic raw
byte order, including invalid UTF-8, without interpreting application names.

Malformed artifacts and renamed coordinates in test controls receive explicitly
test-only matching host pins. They are runtime defense/metamorphic checks, not
new authoring or verifier evidence. Existing Jim application and producer pins
remain unchanged. This fixture implements no traversal, rope, UTF-8 policy,
no-op classification, mutation, causal admission, receipt or WAL behavior.

Reproduce public production using the portable runner and exact source manifests
in the cited Jim commit, with its explicit package boundary and frozen provider.
Reuse the admitted shared Docker worker/target and fail-closed resource guard;
do not use historical build-in-image recipes or create another cache.
