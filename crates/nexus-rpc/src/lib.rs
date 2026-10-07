#![forbid(unsafe_code)]
#![doc = include_str!("../../../README.md")]

extern crate self as nexus_rpc;

mod service;

/// Implementation details used by generated code.
#[doc(hidden)]
pub mod __macro_support {
    use crate::{OperationDefinition, OperationDefinitionInfo, ServiceDefinition};

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

pub use nexus_rpc_macros::{nexus_operation, nexus_service, operation, service};
pub use service::{
    NexusInput, NexusOperation, NexusOutput, NexusService, NexusServiceMetadata,
    OperationDefinition, OperationDefinitionInfo, ServiceDefinition,
};
