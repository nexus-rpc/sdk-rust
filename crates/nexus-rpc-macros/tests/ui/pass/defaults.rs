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
struct Visitor;

impl integration::OperationHandlerVisitor for Visitor {}

impl<I, O> integration::OperationHandlerVisitorFor<I, O> for Visitor
where
    I: NexusInput,
    O: NexusOutput,
{
    fn visit<H>(&mut self, _definition: &'static OperationDefinition<I, O>, _handler: H)
    where
        H: OperationHandler<I, O> + 'static,
    {
    }
}

fn main() {
    let mut visitor = Visitor;
    <GreetingServiceHandler as ServiceHandler>::visit_operations(
        GreetingServiceHandler {
            greet: GreetingHandler,
        },
        &mut visitor,
    );
    let definition = <GreetingServiceHandler as ServiceHandler>::definition();
    assert_eq!(definition.name, "GreetingService");
    assert_eq!(definition.operations[0].wire_name, "greet");
}
