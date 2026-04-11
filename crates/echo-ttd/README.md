<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-ttd

Runtime-side compliance and receipt validation for Echo.

## Overview

This crate provides the compliance validation layer around Echo runtime truth.
It validates that recorded tick emissions and channel outputs conform to
declared channel policies, rule contracts, and determinism constraints.

It is not the debugger product. Its job is to provide runtime-side checking and
structured violations that browser adapters and `warp-ttd` can consume.

## Features

- **Channel Policy Validation**: Verify `StrictSingle`, `Reduce`, and `Log` policies
- **Violation Tracking**: Structured error reporting with severity levels
- **Receipt Verification**: Hash chain and digest validation (future)
- **Runtime-side witness support**: structured violations suitable for adapter
  or protocol lifting

## Usage

```rust
use echo_ttd::compliance::{PolicyChecker, Violation};
use warp_core::materialization::{ChannelPolicy, FinalizedChannel};

let checker = PolicyChecker::new();
let violations = checker.check_channel_policies(&channels, &policies);

if violations.is_empty() {
    println!("All policies satisfied!");
} else {
    for v in &violations {
        eprintln!("{}: {}", v.severity, v.message);
    }
}
```

## License

Apache-2.0
