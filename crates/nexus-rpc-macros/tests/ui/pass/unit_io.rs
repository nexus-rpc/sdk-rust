#![allow(dead_code, unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{
    nexus_operation, nexus_operation_handler, nexus_service, nexus_service_handler,
};

#[nexus_service]
trait UnitService {
    #[nexus_operation]
    fn nop(input: ());
}

struct NopHandler;

impl OperationHandler<(), ()> for NopHandler {
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

struct UnitHandlers;

#[nexus_service_handler(service = UnitService)]
impl UnitHandlers {
    #[nexus_operation_handler]
    fn nop(&self) -> NopHandler {
        NopHandler
    }
}

fn main() {}
