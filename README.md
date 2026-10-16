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
describes service and operation wire names.
It is available through the optional `NexusServiceMetadata` trait; the service
identity contract itself requires only `NexusService::name()`.

## Errors

`HandlerError` describes a handler-level protocol failure and its retry policy.
`OperationError` describes a terminal failed or canceled operation instead.
Both retain application causes through `std::error::Error::source` without flattening them.
Use `HandlerError::from_raw_type` and `raw_error_type()` to preserve unrecognized
protocol codes; `error_type()` maps these to `HandlerErrorType::Unknown`.
`HandlerErrorType::from_wire_name` recognizes only canonical uppercase names.
Both error kinds accept an explicit stack trace through `with_stack_trace` and an
original wire failure through `with_original_failure`. The root-exported, serde-enabled
`FailureInfo` retains opaque metadata and details, a stack trace, and nested causes
without interpreting them or replacing the application source chain.

```rust
use nexus_rpc::{HandlerError, HandlerErrorRetryBehavior, HandlerErrorType, OperationError};

let handler_error = HandlerError::new(HandlerErrorType::Unavailable, "storage unavailable")
    .with_retry_behavior(HandlerErrorRetryBehavior::Retryable)
    .with_source(std::io::Error::other("connection lost"));
assert_eq!(handler_error.error_type().to_string(), "UNAVAILABLE");

let operation_error = OperationError::failed("payment declined");
assert_eq!(operation_error.message(), "payment declined");
```

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
