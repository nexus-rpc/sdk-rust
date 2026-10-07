# Nexus Rust SDK

Service definitions and typed operation markers for Nexus clients and integrations.

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
describes wire names and declared Rust types.
It is available through the optional `NexusServiceMetadata` trait; the service
identity contract itself requires only `NexusService::name()`.
`nexus_service` and `nexus_operation` remain aliases for the definition macros.

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
cargo +1.92.0 check --workspace --all-targets
```
