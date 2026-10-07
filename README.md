# Nexus Rust SDK

Service definitions and typed operation markers for Nexus clients and integrations.
This crate defines the service contract only: it does not author or run operation
handlers.

## Example

```rust
use nexus_rpc::{NexusOperation, NexusService, service};

#[service(name = "payments")]
pub trait PaymentService {
    #[nexus_rpc::operation(name = "charge-card")]
    fn charge(input: String) -> String;
}

fn accepts_charge<Op>(operation: Op)
where
    Op: NexusOperation<Service = PaymentService, Input = String, Output = String>,
{
    assert_eq!(operation.name(), "charge-card");
}

fn main() {
    accepts_charge(PaymentService::charge);
    assert_eq!(PaymentService::name(), "payments");
}
```

The `service` macro replaces a declaration with a zero-sized service marker and
associated operation markers such as `PaymentService::charge`. `NexusOperation`
preserves the declaring service, input, and output types. Generated metadata
describes wire names and declared Rust types without binding any handlers.
It is available through the optional `NexusServiceMetadata` trait; the service
identity contract itself requires only `NexusService::name()`.
`nexus_service` and `nexus_operation` remain aliases for the definition macros.

## Handler ownership

Temporal owns operation authoring, handler registration, start and cancel
contexts, cancellation, operation completion, and handler errors. Use Temporal's
SDK APIs for those capabilities; Nexus has no handler or context authoring API.
The dependency is one-way: Temporal consumes Nexus service definitions and typed
operation markers. Nexus does not depend on Temporal.

## Verification Commands

Run the full validation suite from the repository root:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo +1.92.0 check --workspace --all-targets
```
