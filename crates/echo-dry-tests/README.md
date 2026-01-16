# echo-dry-tests

Shared test doubles and fixtures for Echo crates.

## Purpose

This crate provides reusable test utilities including:

- `FakeConfigStore`: An in-memory configuration store for testing
- Engine builder helpers for determinism testing
- Common test fixtures and setup utilities

## Usage

Add as a dev-dependency in your crate's `Cargo.toml`:

```toml
[dev-dependencies]
echo-dry-tests = { workspace = true }
```
