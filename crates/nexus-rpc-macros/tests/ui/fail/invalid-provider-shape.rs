#![allow(unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{
    nexus_operation, nexus_operation_handler, nexus_service, nexus_service_handler,
};

#[nexus_service]
trait ExampleService {
    #[nexus_operation]
    fn ping(input: ()) -> ();
}

struct PingHandler;

impl OperationHandler<(), ()> for PingHandler {
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

struct ExampleHandlers {
    ping: PingHandler,
}

#[nexus_service_handler(service = ExampleService)]
impl ExampleHandlers {
    #[nexus_operation_handler]
    fn ping(&self, _unused: ()) -> &PingHandler {
        &self.ping
    }
}

fn main() {}
