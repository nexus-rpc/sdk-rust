pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationToken(String);

impl OperationToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Default)]
pub struct OperationStartContext;

#[derive(Clone, Debug)]
pub struct OperationCancelContext {
    token: OperationToken,
}

impl OperationCancelContext {
    pub fn new(token: OperationToken) -> Self {
        Self { token }
    }

    pub fn operation_token(&self) -> &OperationToken {
        &self.token
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationStartResult<O> {
    Sync(O),
    Async(OperationToken),
}

pub trait OperationHandler<I, O>: Send + Sync {
    fn start(
        &self,
        context: OperationStartContext,
        input: I,
    ) -> impl std::future::Future<Output = Result<OperationStartResult<O>, BoxError>> + Send;

    fn cancel(
        &self,
        context: OperationCancelContext,
    ) -> impl std::future::Future<Output = Result<(), BoxError>> + Send;
}

impl<I, O, H> OperationHandler<I, O> for &H
where
    H: OperationHandler<I, O> + ?Sized,
{
    fn start(
        &self,
        context: OperationStartContext,
        input: I,
    ) -> impl std::future::Future<Output = Result<OperationStartResult<O>, BoxError>> + Send {
        (**self).start(context, input)
    }

    fn cancel(
        &self,
        context: OperationCancelContext,
    ) -> impl std::future::Future<Output = Result<(), BoxError>> + Send {
        (**self).cancel(context)
    }
}

pub struct ServiceDefinition {
    pub name: &'static str,
    pub operations: &'static [integration::OperationDefinitionInfo],
}

impl ServiceDefinition {
    pub const fn new(
        name: &'static str,
        operations: &'static [integration::OperationDefinitionInfo],
    ) -> Self {
        Self { name, operations }
    }
}

pub struct OperationDefinition<I, O> {
    pub info: &'static integration::OperationDefinitionInfo,
    pub _marker: std::marker::PhantomData<fn(I) -> O>,
}

pub trait NexusInput: Send + 'static {}
impl<T> NexusInput for T where T: Send + 'static {}

pub trait NexusOutput: Send + 'static {}
impl<T> NexusOutput for T where T: Send + 'static {}

pub trait NexusService: 'static {
    fn name() -> &'static str;
}

pub trait NexusOperation: 'static {
    type Service: NexusService;
    type Input: NexusInput;
    type Output: NexusOutput;

    fn name(&self) -> &str;
}

impl<I, O> OperationDefinition<I, O> {
    pub const fn new(info: &'static integration::OperationDefinitionInfo) -> Self {
        Self {
            info,
            _marker: std::marker::PhantomData,
        }
    }
}

pub mod __macro_support {
    use super::{OperationDefinition, ServiceDefinition, integration::OperationDefinitionInfo};

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

pub trait ServiceHandler: Send + Sync + 'static {
    fn definition() -> &'static ServiceDefinition;

    fn visit_operations<V>(self, visitor: &mut V)
    where
        Self: Sized + integration::ServiceHandlerVisitor<V>,
        V: integration::OperationHandlerVisitor,
    {
        <Self as integration::ServiceHandlerVisitor<V>>::visit_service_handler(self, visitor);
    }
}

pub mod integration {
    use super::{
        NexusInput, NexusOutput, OperationDefinition, OperationHandler, ServiceHandler,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct OperationDefinitionInfo {
        pub rust_name: &'static str,
        pub wire_name: &'static str,
        pub input_type_name: &'static str,
        pub output_type_name: &'static str,
    }

    impl OperationDefinitionInfo {
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
    }

    pub trait OperationHandlerVisitor {}

    pub trait OperationHandlerVisitorFor<I, O>: OperationHandlerVisitor
    where
        I: NexusInput,
        O: NexusOutput,
    {
        fn visit<H>(&mut self, definition: &'static OperationDefinition<I, O>, handler: H)
        where
            H: OperationHandler<I, O> + 'static;
    }

    pub trait ServiceHandlerVisitor<V>: ServiceHandler
    where
        V: OperationHandlerVisitor,
    {
        fn visit_service_handler(self, visitor: &mut V);
    }
}
