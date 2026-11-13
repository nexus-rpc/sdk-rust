# Nexus Rust SDK macros

Procedural macros for Nexus service definitions and typed operation markers.
Requires Rust 1.88 or newer.

Applications should depend on [`nexus-rpc`](https://crates.io/crates/nexus-rpc),
which re-exports the `service` and `operation` macros. A separate dependency on
`nexus-rpc-macros` is not needed.

See the [Nexus Rust SDK documentation](https://docs.rs/nexus-rpc) for usage.
