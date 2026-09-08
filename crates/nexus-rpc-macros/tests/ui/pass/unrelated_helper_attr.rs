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
}

struct PingHandler;

impl OperationHandler<(), ()> for PingHandler {
    fn start(
        &self,
        _context: OperationStartContext,
        _input: (),
    ) -> impl std::future::Future<Output = Result<OperationStartResult<()>, BoxError>> + Send {
        async { Ok(OperationStartResult::Sync(())) }
    }

    fn cancel(
        &self,
        _context: OperationCancelContext,
    ) -> impl std::future::Future<Output = Result<(), BoxError>> + Send {
        async { Ok(()) }
    }
}

struct UtilityHandlers {
    ping: PingHandler,
}

#[nexus_service_handler(service = UtilityService)]
impl UtilityHandlers {
    #[nexus_operation_handler]
    fn ping(&self) -> &PingHandler {
        &self.ping
    }

    #[inline]
    fn helper(&self) -> usize {
        42
    }
}

fn main() {
    let handlers = UtilityHandlers { ping: PingHandler };
    assert_eq!(handlers.helper(), 42);
}
