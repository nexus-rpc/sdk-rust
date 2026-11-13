#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate self as nexus_rpc;

mod error;
mod service;

/// Implementation details used by generated code.
#[doc(hidden)]
pub mod __macro_support {
    use crate::{OperationDefinitionInfo, ServiceDefinition};

    pub const fn operation_definition_info(wire_name: &'static str) -> OperationDefinitionInfo {
        OperationDefinitionInfo::new(wire_name)
    }

    pub const fn service_definition(
        name: &'static str,
        operations: &'static [OperationDefinitionInfo],
    ) -> ServiceDefinition {
        ServiceDefinition::new(name, operations)
    }
}

pub use error::{
    BoxError, FailureInfo, HandlerError, HandlerErrorRetryBehavior, HandlerErrorType,
    OperationError, OperationErrorState,
};
pub use nexus_rpc_macros::{operation, service};
pub use service::{
    NexusInput, NexusOperation, NexusOutput, NexusService, NexusServiceMetadata,
    OperationDefinitionInfo, ServiceDefinition,
};
