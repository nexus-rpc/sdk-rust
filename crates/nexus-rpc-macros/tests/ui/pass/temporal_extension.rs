#![allow(dead_code, unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{nexus_operation, nexus_service, nexus_service_handler};
use temporal_extension::temporal_operation;

#[derive(Default)]
struct DummyTemporalContext;

#[derive(Default)]
struct DummyTemporalClient;

#[nexus_service]
trait PaymentService {
    #[nexus_operation]
    fn charge(input: String) -> String;
}

struct PaymentHandlers;

#[nexus_service_handler(service = PaymentService)]
impl PaymentHandlers {
    #[temporal_operation]
    async fn charge(
        &self,
        _context: DummyTemporalContext,
        _client: DummyTemporalClient,
        input: String,
    ) -> Result<OperationStartResult<String>, BoxError> {
        Ok(OperationStartResult::Sync(input))
    }
}

fn main() {
    let definition = <PaymentHandlers as ServiceHandler>::definition();
    assert_eq!(definition.operations[0].wire_name, "charge");
}
