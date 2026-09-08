use nexus_rpc::{
    BoxError, OperationCancelContext, OperationDefinition, OperationHandler, OperationStartContext,
    OperationStartResult, ServiceHandler, integration::OperationRegistrar, nexus_service,
    nexus_service_handler,
};

#[nexus_service(name = "echo-service")]
#[allow(dead_code)]
trait EchoService {
    #[nexus_operation(name = "echo-message")]
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
    #[nexus_operation_handler]
    fn echo(&self) -> impl OperationHandler<String, String> + '_ {
        &self.echo
    }
}

#[derive(Default)]
struct RecordingRegistrar {
    operations: Vec<(&'static str, &'static str)>,
}

impl OperationRegistrar for RecordingRegistrar {
    fn register<I, O, H>(&mut self, definition: &'static OperationDefinition<I, O>, _handler: H)
    where
        I: Send + 'static,
        O: Send + 'static,
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

    let mut registrar = RecordingRegistrar::default();
    EchoServiceHandler { echo: EchoHandler }.register(&mut registrar);

    assert_eq!(registrar.operations, vec![("echo", "echo-message")]);
}
