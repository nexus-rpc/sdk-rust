use std::{
    any::Any,
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};

use nexus_rpc::{
    BoxError, NexusInput, NexusOutput, OperationCancelContext, OperationDefinition,
    OperationHandler, OperationStartContext, OperationStartResult, OperationToken,
    ServiceDefinition, ServiceHandler,
    integration::{
        OperationDefinitionInfo, OperationHandlerVisitor, OperationHandlerVisitorFor,
        ServiceHandlerVisitor,
    },
};

static PING_INFO: OperationDefinitionInfo =
    nexus_rpc::__macro_support::operation_definition_info("ping", "ping-wire", "u32", "u32");
static EMPTY_INFO: OperationDefinitionInfo =
    nexus_rpc::__macro_support::operation_definition_info("empty", "empty", "()", "()");
static PING_DEFINITION: OperationDefinition<u32, u32> =
    nexus_rpc::__macro_support::operation_definition(&PING_INFO);
static EMPTY_DEFINITION: OperationDefinition<(), ()> =
    nexus_rpc::__macro_support::operation_definition(&EMPTY_INFO);
static SERVICE_DEFINITION: ServiceDefinition =
    nexus_rpc::__macro_support::service_definition("test-service", &[PING_INFO, EMPTY_INFO]);

fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn start_context(operation: &str) -> OperationStartContext {
    OperationStartContext::builder("test-service", operation, "request-id").build()
}

fn cancel_context(operation: &str, token: &str) -> OperationCancelContext {
    OperationCancelContext::builder("test-service", operation, OperationToken::new(token)).build()
}

#[derive(Clone)]
struct PingHandler {
    started: Arc<AtomicBool>,
    canceled: Arc<AtomicBool>,
}

impl OperationHandler<u32, u32> for PingHandler {
    async fn start(
        &self,
        _context: OperationStartContext,
        input: u32,
    ) -> Result<OperationStartResult<u32>, BoxError> {
        self.started.store(true, Ordering::SeqCst);
        Ok(OperationStartResult::Sync(input + 1))
    }

    async fn cancel(&self, _context: OperationCancelContext) -> Result<(), BoxError> {
        self.canceled.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct EmptyHandler;

impl OperationHandler<(), ()> for EmptyHandler {
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

struct TestService {
    started: Arc<AtomicBool>,
    canceled: Arc<AtomicBool>,
}

impl TestService {
    fn ping(&self) -> impl OperationHandler<u32, u32> + '_ {
        PingHandler {
            started: Arc::clone(&self.started),
            canceled: Arc::clone(&self.canceled),
        }
    }

    fn empty(&self) -> impl OperationHandler<(), ()> + '_ {
        EmptyHandler
    }
}

struct PingRegistration {
    service: Arc<TestService>,
}

impl OperationHandler<u32, u32> for PingRegistration {
    async fn start(
        &self,
        context: OperationStartContext,
        input: u32,
    ) -> Result<OperationStartResult<u32>, BoxError> {
        self.service.ping().start(context, input).await
    }

    async fn cancel(&self, context: OperationCancelContext) -> Result<(), BoxError> {
        self.service.ping().cancel(context).await
    }
}

struct EmptyRegistration {
    service: Arc<TestService>,
}

impl OperationHandler<(), ()> for EmptyRegistration {
    async fn start(
        &self,
        context: OperationStartContext,
        input: (),
    ) -> Result<OperationStartResult<()>, BoxError> {
        self.service.empty().start(context, input).await
    }

    async fn cancel(&self, context: OperationCancelContext) -> Result<(), BoxError> {
        self.service.empty().cancel(context).await
    }
}

impl ServiceHandler for TestService {
    fn definition() -> &'static ServiceDefinition {
        &SERVICE_DEFINITION
    }
}

impl<V> ServiceHandlerVisitor<V> for TestService
where
    V: OperationHandlerVisitorFor<u32, u32> + OperationHandlerVisitorFor<(), ()>,
{
    fn visit_service_handler(self, visitor: &mut V) {
        let service = Arc::new(self);
        visitor.visit(
            &PING_DEFINITION,
            PingRegistration {
                service: Arc::clone(&service),
            },
        );
        visitor.visit(&EMPTY_DEFINITION, EmptyRegistration { service });
    }
}

struct StoredOperation<I: 'static, O: 'static, H> {
    definition: &'static OperationDefinition<I, O>,
    handler: H,
}

struct FakeVisitor {
    wire_names: Vec<&'static str>,
    operations: Vec<Box<dyn Any + Send + Sync>>,
}

impl FakeVisitor {
    fn new() -> Self {
        Self {
            wire_names: Vec::new(),
            operations: Vec::new(),
        }
    }
}

impl OperationHandlerVisitor for FakeVisitor {}

impl<I, O> OperationHandlerVisitorFor<I, O> for FakeVisitor
where
    I: NexusInput,
    O: NexusOutput,
{
    fn visit<H>(&mut self, definition: &'static OperationDefinition<I, O>, handler: H)
    where
        H: OperationHandler<I, O> + 'static,
    {
        self.wire_names.push(definition.info().wire_name());
        self.operations.push(Box::new(StoredOperation {
            definition,
            handler,
        }));
    }
}

#[test]
fn service_definition_exposes_erased_metadata() {
    let definition = <TestService as ServiceHandler>::definition();

    assert_eq!(definition.name(), "test-service");
    assert_eq!(definition.operations().len(), 2);
    assert_eq!(definition.operations()[0].rust_name(), "ping");
    assert_eq!(definition.operations()[0].wire_name(), "ping-wire");
    assert_eq!(definition.operations()[1].input_type_name(), "()");
    assert_eq!(definition.operations()[1].output_type_name(), "()");
}

#[test]
fn visitor_receives_each_operation_once() {
    let service = TestService {
        started: Arc::new(AtomicBool::new(false)),
        canceled: Arc::new(AtomicBool::new(false)),
    };
    let mut visitor = FakeVisitor::new();

    service.visit_operations(&mut visitor);

    assert_eq!(visitor.wire_names, vec!["ping-wire", "empty"]);
    assert_eq!(visitor.operations.len(), 2);
}

#[test]
fn visited_wrappers_remain_callable_after_service_is_moved() {
    let started = Arc::new(AtomicBool::new(false));
    let canceled = Arc::new(AtomicBool::new(false));
    let service = TestService {
        started: Arc::clone(&started),
        canceled: Arc::clone(&canceled),
    };
    let mut visitor = FakeVisitor::new();

    service.visit_operations(&mut visitor);

    let ping = visitor.operations[0]
        .downcast_ref::<StoredOperation<u32, u32, PingRegistration>>()
        .expect("missing ping registration");
    let empty = visitor.operations[1]
        .downcast_ref::<StoredOperation<(), (), EmptyRegistration>>()
        .expect("missing empty registration");

    assert_eq!(ping.definition.info().wire_name(), "ping-wire");
    assert_eq!(empty.definition.info().wire_name(), "empty");

    let ping_result =
        block_on(ping.handler.start(start_context("ping-wire"), 41)).expect("ping start failed");
    assert_eq!(ping_result, OperationStartResult::Sync(42));
    assert!(started.load(Ordering::SeqCst));

    block_on(
        ping.handler
            .cancel(cancel_context("ping-wire", "cancel-token")),
    )
    .expect("ping cancel failed");
    assert!(canceled.load(Ordering::SeqCst));

    let empty_result =
        block_on(empty.handler.start(start_context("empty"), ())).expect("empty start failed");
    assert_eq!(empty_result, OperationStartResult::Sync(()));
}
