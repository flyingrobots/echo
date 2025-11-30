<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->

# Legal Overview

This document explains how licensing works in this repository. It is a summary
only; if anything here conflicts with the full license texts, those texts
control (`LICENSE-APACHE`, `LICENSE-MIND-UCAL`).

## 1) Code

- Applies to: Rust source, build scripts, shell/Python/JS tooling, binaries, Makefiles, configs used to build/run the code.
- License: **Apache License, Version 2.0** only.  
  See `LICENSE-APACHE`.
- SPDX for code files: `SPDX-License-Identifier: Apache-2.0`

## 2) Theory / Math / Documentation

- Applies to: `docs/`, `rmg-math/`, LaTeX sources, papers/notes, other written or mathematical materials.
- License options (your choice):
  - Apache License, Version 2.0 **OR**
  - MIND-UCAL License v1.0
- SPDX for these files: `SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0`
- If you do not wish to use MIND-UCAL, you may use all theory, math, and documentation under Apache 2.0 alone. No portion of this repository requires adopting MIND-UCAL.

## 3) SPDX Policy

- All tracked source and documentation files must carry an SPDX header.
- Enforcement:
  - `scripts/ensure_spdx.sh` (pre-commit): inserts missing headers into staged files, restages, and aborts so you can review.
  - `scripts/check_spdx.sh`: check-only helper (unused by default).
- Patterns:
  - Code: `Apache-2.0`
  - Docs/math: `Apache-2.0 OR MIND-UCAL-1.0`
- Exclusions: generated/binary assets (e.g., target/, node_modules/, PDFs, images) are not labeled.

## 4) NOTICE

See `NOTICE` for attribution. Apache 2.0 requires preservation of NOTICE content in redistributions that include NOTICE.

## 5) No additional terms

No extra terms or conditions beyond the licenses above. Unless required by law,
all material is provided “AS IS”, without warranties or conditions of any kind.
