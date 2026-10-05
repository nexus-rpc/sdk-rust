#![allow(dead_code, unused_imports, clippy::unused_unit)]

extern crate self as nexus_rpc;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use nexus_rpc_macros::{nexus_operation_handler, nexus_service_handler, operation, service};

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
    use super::{NexusInput, NexusOutput, OperationDefinition, OperationHandler, ServiceHandler};

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

#[service(name = "payments")]
trait PaymentService {
    #[operation(name = "charge-card")]
    fn charge(input: ()) -> ();

    #[operation]
    fn ping(input: ()) -> ();
}

#[service]
trait IdentifierService {
    fn r#type(input: ()) -> ();
    fn foo(input: ()) -> ();
    #[allow(non_snake_case)]
    fn FOO(input: ()) -> ();
}

#[test]
fn supports_raw_and_case_distinct_operation_identifiers() {
    let names: Vec<_> = __nexus_service_metadata_for_IdentifierService::__NEXUS_OPERATION_INFOS
        .iter()
        .map(|info| info.rust_name)
        .collect();

    assert_eq!(names, ["type", "foo", "FOO"]);
}

struct CountingHandler {
    starts: Arc<AtomicUsize>,
    cancels: Arc<AtomicUsize>,
}

impl OperationHandler<(), ()> for CountingHandler {
    async fn start(
        &self,
        _context: OperationStartContext,
        _input: (),
    ) -> Result<OperationStartResult<()>, BoxError> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        Ok(OperationStartResult::Sync(()))
    }

    async fn cancel(&self, context: OperationCancelContext) -> Result<(), BoxError> {
        assert_eq!(context.operation_token().as_str(), "token");
        self.cancels.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct PaymentServiceHandler {
    charge: CountingHandler,
    ping: CountingHandler,
}

#[nexus_service_handler(service = PaymentService)]
impl PaymentServiceHandler {
    #[nexus_operation_handler]
    fn charge(&self) -> &CountingHandler {
        &self.charge
    }

    #[nexus_operation_handler]
    fn ping(&self) -> &CountingHandler {
        &self.ping
    }
}

#[derive(Default)]
struct RecordingVisitor {
    operations: Vec<(&'static str, &'static str)>,
}

impl integration::OperationHandlerVisitor for RecordingVisitor {}

impl<I, O> integration::OperationHandlerVisitorFor<I, O> for RecordingVisitor
where
    I: NexusInput,
    O: NexusOutput,
{
    fn visit<H>(&mut self, definition: &'static OperationDefinition<I, O>, _handler: H)
    where
        H: OperationHandler<I, O> + 'static,
    {
        self.operations
            .push((definition.info.rust_name, definition.info.wire_name));
    }
}

#[test]
fn generates_service_metadata_and_registration_wrappers() {
    let charge_starts = Arc::new(AtomicUsize::new(0));
    let charge_cancels = Arc::new(AtomicUsize::new(0));
    let ping_starts = Arc::new(AtomicUsize::new(0));
    let ping_cancels = Arc::new(AtomicUsize::new(0));

    let service = PaymentServiceHandler {
        charge: CountingHandler {
            starts: Arc::clone(&charge_starts),
            cancels: Arc::clone(&charge_cancels),
        },
        ping: CountingHandler {
            starts: Arc::clone(&ping_starts),
            cancels: Arc::clone(&ping_cancels),
        },
    };

    let definition = <PaymentServiceHandler as ServiceHandler>::definition();
    assert_eq!(definition.name, "payments");
    assert_eq!(definition.operations.len(), 2);
    assert_eq!(definition.operations[0].rust_name, "charge");
    assert_eq!(definition.operations[0].wire_name, "charge-card");
    assert_eq!(definition.operations[1].wire_name, "ping");

    let mut visitor = RecordingVisitor::default();
    <PaymentServiceHandler as ServiceHandler>::visit_operations(service, &mut visitor);

    assert_eq!(
        visitor.operations,
        vec![("charge", "charge-card"), ("ping", "ping")]
    );
    assert_eq!(charge_starts.load(Ordering::SeqCst), 0);
    assert_eq!(charge_cancels.load(Ordering::SeqCst), 0);
    assert_eq!(ping_starts.load(Ordering::SeqCst), 0);
    assert_eq!(ping_cancels.load(Ordering::SeqCst), 0);
}
