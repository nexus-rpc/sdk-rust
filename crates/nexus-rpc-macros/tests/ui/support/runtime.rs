include!("../../../../nexus-rpc/src/service.rs");

pub mod __macro_support {
    use super::{OperationDefinition, OperationDefinitionInfo, ServiceDefinition};

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
