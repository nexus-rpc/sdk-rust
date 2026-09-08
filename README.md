# Nexus Rust SDK

Minimal generic authoring primitives for Nexus services in Rust.

## Example

The snippet below shows the generic authoring model implemented by this workspace.

```rust
use nexus_rpc::{
    BoxError, OperationCancelContext, OperationHandler, OperationStartContext,
    OperationStartResult, nexus_service, nexus_service_handler,
};

#[nexus_service(name = "payments")]
pub trait PaymentService {
    #[nexus_operation(name = "charge-card")]
    fn charge(input: String) -> String;
}

struct ChargeHandler;

impl OperationHandler<String, String> for ChargeHandler {
    async fn start(
        &self,
        _context: OperationStartContext,
        input: String,
    ) -> Result<OperationStartResult<String>, BoxError> {
        Ok(OperationStartResult::Sync(input))
    }

    async fn cancel(&self, _context: OperationCancelContext) -> Result<(), BoxError> {
        Ok(())
    }
}

struct PaymentServiceHandler {
    charge: ChargeHandler,
}

#[nexus_service_handler(service = PaymentService)]
impl PaymentServiceHandler {
    #[nexus_operation_handler]
    fn charge(&self) -> impl OperationHandler<String, String> + '_ {
        &self.charge
    }
}
```

Async completion uses the same model: return `OperationStartResult::Async(OperationToken::new(...))` from `start`, and receive that token again in `OperationCancelContext`.

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
