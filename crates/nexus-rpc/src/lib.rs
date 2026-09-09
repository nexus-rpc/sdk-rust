#![forbid(unsafe_code)]
#![doc = include_str!("../../../README.md")]
#![doc = r#"
Minimal Nexus authoring primitives with a transport-free registration seam.

# Examples

Implement an operation handler with ordinary `async fn` methods:

```rust
use nexus_rpc::{
    BoxError, OperationCancelContext, OperationHandler, OperationStartContext,
    OperationStartResult,
};

struct Echo;

impl OperationHandler<String, String> for Echo {
    async fn start(
        &self,
        _context: OperationStartContext,
        input: String,
    ) -> Result<OperationStartResult<String>, BoxError> {
        Ok(OperationStartResult::Sync(input))
    }

    async fn cancel(
        &self,
        _context: OperationCancelContext,
    ) -> Result<(), BoxError> {
        Ok(())
    }
}
```

Manually bind a typed service definition to a downstream registrar:

```rust
use nexus_rpc::{
    BoxError, OperationCancelContext, OperationDefinition, OperationHandler,
    OperationStartContext, OperationStartResult, ServiceDefinition,
    ServiceHandler,
};
use nexus_rpc::integration::{OperationDefinitionInfo, OperationRegistrar};

static ECHO_INFO: OperationDefinitionInfo = OperationDefinitionInfo::new(
    "echo",
    "echo",
    "alloc::string::String",
    "alloc::string::String",
);
static ECHO_DEFINITION: OperationDefinition<String, String> =
    OperationDefinition::new(&ECHO_INFO);
static SERVICE_DEFINITION: ServiceDefinition =
    ServiceDefinition::new("echo-service", &[ECHO_INFO]);

struct Echo;

impl OperationHandler<String, String> for Echo {
    async fn start(
        &self,
        _context: OperationStartContext,
        input: String,
    ) -> Result<OperationStartResult<String>, BoxError> {
        Ok(OperationStartResult::Sync(input))
    }

    async fn cancel(
        &self,
        _context: OperationCancelContext,
    ) -> Result<(), BoxError> {
        Ok(())
    }
}

struct EchoService;

impl ServiceHandler for EchoService {
    fn definition() -> &'static ServiceDefinition {
        &SERVICE_DEFINITION
    }

    fn register<R>(self, registrar: &mut R)
    where
        Self: Sized,
        R: OperationRegistrar,
    {
        registrar.register(&ECHO_DEFINITION, Echo);
    }
}

struct NoopRegistrar;

impl OperationRegistrar for NoopRegistrar {
    fn register<I, O, H>(
        &mut self,
        _definition: &'static OperationDefinition<I, O>,
        _handler: H,
    )
    where
        I: Send + 'static,
        O: Send + 'static,
        H: OperationHandler<I, O> + 'static,
    {
    }
}

let mut registrar = NoopRegistrar;
EchoService.register(&mut registrar);
```
"#]

extern crate self as nexus_rpc;

mod error;
mod handler;
mod operation;
mod service;

pub use error::{
    BoxError, HandlerError, HandlerErrorRetryBehavior, HandlerErrorType, OperationError,
    OperationErrorState,
};
pub use handler::ServiceHandler;
pub use nexus_rpc_macros::{
    nexus_operation, nexus_operation_handler, nexus_service, nexus_service_handler,
};
pub use operation::{
    Headers, Link, OperationCancelContext, OperationCancelContextBuilder, OperationHandler,
    OperationStartContext, OperationStartContextBuilder, OperationStartResult, OperationToken,
    RequestCancellation,
};
pub use service::{OperationDefinition, ServiceDefinition};

/// Adapter-facing types for worker integrations.
pub mod integration {
    pub use crate::handler::{ClassifiedHandlerError, OperationRegistrar, classify_handler_error};
    pub use crate::operation::RequestCanceller;
    pub use crate::service::OperationDefinitionInfo;
}
