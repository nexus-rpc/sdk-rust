# Nexus Rust SDK

Service definitions and typed operation markers for Nexus clients and integrations.

- [`nexus-rpc`](crates/nexus-rpc/README.md): public API, examples, and installation.
- [`nexus-rpc-macros`](crates/nexus-rpc-macros/README.md): procedural macros re-exported
  by `nexus-rpc`.
- [Releasing](RELEASING.md): initial crates.io publication and subsequent release setup.

## Verification Commands

Tests exercise typed markers, wire names, and matching metadata through the
public API. Compile-fail fixtures check type safety and invalid operation
declarations.

Run the full validation suite from the repository root:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo +1.88 check --workspace --all-targets
cargo publish -p nexus-rpc-macros -p nexus-rpc --dry-run
```
