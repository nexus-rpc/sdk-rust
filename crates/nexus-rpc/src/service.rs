use std::marker::PhantomData;

/// Erased immutable operation metadata stored by a service definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationDefinitionInfo {
    rust_name: &'static str,
    wire_name: &'static str,
    input_type_name: &'static str,
    output_type_name: &'static str,
}

impl OperationDefinitionInfo {
    /// Creates an erased operation definition record.
    pub const fn new(
        rust_name: &'static str,
        wire_name: &'static str,
        input_type_name: &'static str,
        output_type_name: &'static str,
    ) -> Self {
        Self {
            rust_name,
            wire_name,
            input_type_name,
            output_type_name,
        }
    }

    /// Returns the Rust method name used for handler association.
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the Nexus wire name for the operation.
    pub const fn wire_name(&self) -> &'static str {
        self.wire_name
    }

    /// Returns the diagnostic Rust type name of the operation input.
    pub const fn input_type_name(&self) -> &'static str {
        self.input_type_name
    }

    /// Returns the diagnostic Rust type name of the operation output.
    pub const fn output_type_name(&self) -> &'static str {
        self.output_type_name
    }
}

/// Immutable service metadata.
#[derive(Debug)]
pub struct ServiceDefinition {
    name: &'static str,
    operations: &'static [OperationDefinitionInfo],
}

impl ServiceDefinition {
    /// Creates an immutable service definition.
    pub const fn new(name: &'static str, operations: &'static [OperationDefinitionInfo]) -> Self {
        Self { name, operations }
    }

    /// Returns the Nexus service wire name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the immutable erased operation metadata slice.
    pub const fn operations(&self) -> &'static [OperationDefinitionInfo] {
        self.operations
    }
}

/// Typed operation metadata used during downstream registration.
#[derive(Debug)]
pub struct OperationDefinition<I, O> {
    info: &'static OperationDefinitionInfo,
    _marker: PhantomData<fn(I) -> O>,
}

impl<I, O> OperationDefinition<I, O> {
    /// Creates a typed operation definition from an erased metadata record.
    pub const fn new(info: &'static OperationDefinitionInfo) -> Self {
        Self {
            info,
            _marker: PhantomData,
        }
    }

    /// Returns the shared erased operation metadata.
    pub const fn info(&self) -> &'static OperationDefinitionInfo {
        self.info
    }
}
