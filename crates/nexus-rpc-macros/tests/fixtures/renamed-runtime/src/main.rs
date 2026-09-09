use nr::{
    BoxError, OperationCancelContext, OperationHandler, OperationStartContext,
    OperationStartResult, ServiceHandler,
};

#[allow(dead_code)]
#[nr::nexus_service]
trait RenamedService {
    #[nr::nexus_operation(name = "renamed-operation")]
    fn operation(input: ()) -> ();
}

struct Handler;

impl OperationHandler<(), ()> for Handler {
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

struct RenamedHandler;

#[nr::nexus_service_handler(service = RenamedService)]
impl RenamedHandler {
    #[nr::nexus_operation_handler]
    fn operation(&self) -> Handler {
        Handler
    }
}

fn main() {
    let _ = <RenamedHandler as ServiceHandler>::definition();
}
