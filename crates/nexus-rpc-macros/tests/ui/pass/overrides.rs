#![allow(dead_code, unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{
    nexus_operation, nexus_operation_handler, nexus_service, nexus_service_handler,
};

#[nexus_service(name = "payments")]
pub trait PaymentService {
    #[nexus_operation(name = "charge-card")]
    fn charge(input: ()) -> ();
}

struct ChargeHandler;

impl OperationHandler<(), ()> for ChargeHandler {
    async fn start(
        &self,
        _context: OperationStartContext,
        _input: (),
    ) -> Result<OperationStartResult<()>, BoxError> {
        Ok(OperationStartResult::Sync(()))
    }

    async fn cancel(&self, _context: OperationCancelContext) -> Result<(), BoxError> {
        Ok(())
    }
}

pub struct PaymentHandlers {
    charge: ChargeHandler,
}

#[nexus_service_handler(service = PaymentService)]
impl PaymentHandlers {
    #[nexus_operation_handler]
    fn charge(&self) -> &ChargeHandler {
        &self.charge
    }
}

fn main() {
    let definition = <PaymentHandlers as ServiceHandler>::definition();
    assert_eq!(definition.name, "payments");
    assert_eq!(definition.operations[0].rust_name, "charge");
    assert_eq!(definition.operations[0].wire_name, "charge-card");
}
