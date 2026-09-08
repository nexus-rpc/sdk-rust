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

impl<I, O> OperationDefinition<I, O> {
    pub const fn new(info: &'static integration::OperationDefinitionInfo) -> Self {
        Self {
            info,
            _marker: std::marker::PhantomData,
        }
    }
}

pub trait ServiceHandler: Send + Sync + 'static {
    fn definition() -> &'static ServiceDefinition;

    fn register<R>(self, registrar: &mut R)
    where
        Self: Sized,
        R: integration::OperationRegistrar;
}

pub mod integration {
    use super::{OperationDefinition, OperationHandler};

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

    pub trait OperationRegistrar {
        fn register<I, O, H>(&mut self, definition: &'static OperationDefinition<I, O>, handler: H)
        where
            I: Send + 'static,
            O: Send + 'static,
            H: OperationHandler<I, O> + 'static;
    }
}
