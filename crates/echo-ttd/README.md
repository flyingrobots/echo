<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-ttd

Time-Travel Debugger compliance engine for Echo.

## Overview

This crate provides the compliance validation layer for the Echo TTD system.
It validates that recorded tick emissions conform to declared channel policies,
rule contracts, and determinism constraints.

## Features

- **Channel Policy Validation**: Verify `StrictSingle`, `Reduce`, and `Log` policies
- **Violation Tracking**: Structured error reporting with severity levels
- **Receipt Verification**: Hash chain and digest validation (future)

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
