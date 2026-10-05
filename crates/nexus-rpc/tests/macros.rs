use nexus_rpc::{
    BoxError, NexusInput, NexusOutput, OperationCancelContext, OperationDefinition,
    OperationHandler, OperationStartContext, OperationStartResult, ServiceHandler,
    integration::{OperationHandlerVisitor, OperationHandlerVisitorFor},
    nexus_service_handler, service,
};

#[service(name = "echo-service")]
trait EchoService {
    #[nexus_rpc::operation(name = "echo-message")]
    fn echo(input: String) -> String;
}

struct EchoHandler;

impl OperationHandler<String, String> for EchoHandler {
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

struct EchoServiceHandler {
    echo: EchoHandler,
}

#[nexus_service_handler(service = EchoService)]
impl EchoServiceHandler {
    #[nexus_rpc::nexus_operation_handler]
    fn echo(&self) -> impl OperationHandler<String, String> + '_ {
        &self.echo
    }
}

#[derive(Default)]
struct RecordingVisitor {
    operations: Vec<(&'static str, &'static str)>,
}

impl OperationHandlerVisitor for RecordingVisitor {}

impl<I, O> OperationHandlerVisitorFor<I, O> for RecordingVisitor
where
    I: NexusInput,
    O: NexusOutput,
{
    fn visit<H>(&mut self, definition: &'static OperationDefinition<I, O>, _handler: H)
    where
        H: OperationHandler<I, O> + 'static,
    {
        let info = definition.info();
        self.operations.push((info.rust_name(), info.wire_name()));
    }
}

#[test]
fn reexported_macros_generate_runtime_registration_contract() {
    let definition = <EchoServiceHandler as ServiceHandler>::definition();
    assert_eq!(definition.name(), "echo-service");
    assert_eq!(definition.operations().len(), 1);
    assert_eq!(definition.operations()[0].wire_name(), "echo-message");

    let mut visitor = RecordingVisitor::default();
    EchoServiceHandler { echo: EchoHandler }.visit_operations(&mut visitor);

    assert_eq!(visitor.operations, vec![("echo", "echo-message")]);
}
