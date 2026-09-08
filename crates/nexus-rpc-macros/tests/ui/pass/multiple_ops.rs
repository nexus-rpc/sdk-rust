#![allow(dead_code, unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{
    nexus_operation, nexus_operation_handler, nexus_service, nexus_service_handler,
};

#[nexus_service]
trait UtilityService {
    #[nexus_operation]
    fn ping(input: ()) -> ();

    #[nexus_operation]
    fn status(input: ()) -> ();
}

struct PingHandler;
struct StatusHandler;

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

impl OperationHandler<(), ()> for StatusHandler {
    async fn start(
        &self,
        _context: OperationStartContext,
        _input: (),
    ) -> Result<OperationStartResult<()>, BoxError> {
        Ok(OperationStartResult::Async(OperationToken::new("status")))
    }

    async fn cancel(&self, _context: OperationCancelContext) -> Result<(), BoxError> {
        Ok(())
    }
}

struct UtilityHandlers {
    ping: PingHandler,
    status: StatusHandler,
}

#[nexus_service_handler(service = UtilityService)]
impl UtilityHandlers {
    #[nexus_operation_handler]
    fn ping(&self) -> &PingHandler {
        &self.ping
    }

    #[nexus_operation_handler]
    fn status(&self) -> &StatusHandler {
        &self.status
    }
}

fn main() {
    let definition = <UtilityHandlers as ServiceHandler>::definition();
    assert_eq!(definition.operations.len(), 2);
}
