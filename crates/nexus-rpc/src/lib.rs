#![forbid(unsafe_code)]
#![doc = include_str!("../../../README.md")]

extern crate self as nexus_rpc;

mod error;
mod handler;
mod operation;
mod service;

/// Implementation details used by generated code.
#[doc(hidden)]
pub mod __macro_support {
    use crate::{OperationDefinition, ServiceDefinition, service::OperationDefinitionInfo};

    pub const fn operation_definition_info(
        rust_name: &'static str,
        wire_name: &'static str,
        input_type_name: &'static str,
        output_type_name: &'static str,
    ) -> OperationDefinitionInfo {
        OperationDefinitionInfo::new(rust_name, wire_name, input_type_name, output_type_name)
    }

    pub const fn service_definition(
        name: &'static str,
        operations: &'static [OperationDefinitionInfo],
    ) -> ServiceDefinition {
        ServiceDefinition::new(name, operations)
    }

    pub const fn operation_definition<I, O>(
        info: &'static OperationDefinitionInfo,
    ) -> OperationDefinition<I, O> {
        OperationDefinition::new(info)
    }
}

pub use error::{
    BoxError, HandlerError, HandlerErrorRetryBehavior, HandlerErrorType, OperationError,
    OperationErrorState,
};
pub use handler::ServiceHandler;
pub use nexus_rpc_macros::{
    nexus_operation, nexus_operation_handler, nexus_service, nexus_service_handler, operation,
    service,
};
pub use operation::{
    Headers, Link, OperationCancelContext, OperationCancelContextBuilder, OperationHandler,
    OperationStartContext, OperationStartContextBuilder, OperationStartResult, OperationToken,
    RequestCancellation,
};
pub use service::{
    NexusInput, NexusOperation, NexusOutput, NexusService, OperationDefinition, ServiceDefinition,
};

/// Adapter-facing types for worker integrations.
pub mod integration {
    pub use crate::handler::{
        ClassifiedHandlerError, OperationHandlerVisitor, OperationHandlerVisitorFor,
        ServiceHandlerVisitor, classify_handler_error,
    };
    pub use crate::operation::RequestCanceller;
    pub use crate::service::OperationDefinitionInfo;
}
