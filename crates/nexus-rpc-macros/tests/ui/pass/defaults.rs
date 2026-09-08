#![allow(dead_code, unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{
    nexus_operation, nexus_operation_handler, nexus_service, nexus_service_handler,
};

#[nexus_service]
trait GreetingService {
    #[nexus_operation]
    fn greet(input: ()) -> ();
}

struct GreetingHandler;

impl OperationHandler<(), ()> for GreetingHandler {
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

struct GreetingServiceHandler {
    greet: GreetingHandler,
}

#[nexus_service_handler(service = GreetingService)]
impl GreetingServiceHandler {
    #[nexus_operation_handler]
    fn greet(&self) -> &GreetingHandler {
        &self.greet
    }
}

#[derive(Default)]
struct Registrar;

impl integration::OperationRegistrar for Registrar {
    fn register<I, O, H>(&mut self, _definition: &'static OperationDefinition<I, O>, _handler: H)
    where
        I: Send + 'static,
        O: Send + 'static,
        H: OperationHandler<I, O> + 'static,
    {
    }
}

fn main() {
    let mut registrar = Registrar;
    <GreetingServiceHandler as ServiceHandler>::register(
        GreetingServiceHandler {
            greet: GreetingHandler,
        },
        &mut registrar,
    );
    let definition = <GreetingServiceHandler as ServiceHandler>::definition();
    assert_eq!(definition.name, "GreetingService");
    assert_eq!(definition.operations[0].wire_name, "greet");
}
