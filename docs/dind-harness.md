# DIND Harness (Deterministic Ironclad Nightmare Drills)

The DIND harness is the deterministic verification runner for Echo/WARP. It replays canonical intent transcripts and asserts that state hashes and intermediate outputs are identical across runs, platforms, and build profiles.

Location:
- `crates/echo-dind-harness`

## Quickstart

```bash
cargo run -p echo-dind-harness -- help
```

Examples (commands depend on the harness CLI):

```bash
cargo run -p echo-dind-harness -- torture
cargo run -p echo-dind-harness -- converge
cargo run -p echo-dind-harness -- repro <scenario>
```

## Determinism Guardrails

Echo ships guard scripts to enforce determinism in core crates:

- `scripts/ban-globals.sh`
- `scripts/ban-nondeterminism.sh`
- `scripts/ban-unordered-abi.sh`

Run them locally or wire them into CI for strict enforcement.
