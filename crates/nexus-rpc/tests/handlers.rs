use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

use nexus_rpc::{
    BoxError, OperationCancelContext, OperationHandler, OperationStartContext,
    OperationStartResult, OperationToken,
};

#[derive(Default)]
struct CountingHandler {
    starts: AtomicUsize,
    cancels: AtomicUsize,
    seen_token_len: AtomicUsize,
}

impl OperationHandler<String, String> for CountingHandler {
    async fn start(
        &self,
        _context: OperationStartContext,
        input: String,
    ) -> Result<OperationStartResult<String>, BoxError> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        if input == "async" {
            Ok(OperationStartResult::Async(OperationToken::new("tok-1")))
        } else {
            Ok(OperationStartResult::Sync(input))
        }
    }

    async fn cancel(&self, context: OperationCancelContext) -> Result<(), BoxError> {
        self.cancels.fetch_add(1, Ordering::SeqCst);
        self.seen_token_len
            .fetch_add(context.operation_token().as_str().len(), Ordering::SeqCst);
        Ok(())
    }
}

struct UnitHandler;

impl OperationHandler<(), ()> for UnitHandler {
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

fn assert_send<T: Send>(value: T) -> T {
    value
}

fn start_context() -> OperationStartContext {
    OperationStartContext::builder("test-service", "test-operation", "request-id").build()
}

fn cancel_context(token: impl Into<String>) -> OperationCancelContext {
    OperationCancelContext::builder("test-service", "test-operation", OperationToken::new(token))
        .build()
}

#[test]
fn sync_and_async_start_results_round_trip() {
    let handler = CountingHandler::default();

    let sync = block_on(handler.start(start_context(), "sync".into())).expect("sync start failed");
    let async_result =
        block_on(handler.start(start_context(), "async".into())).expect("async start failed");

    assert_eq!(sync, OperationStartResult::Sync(String::from("sync")));
    assert_eq!(
        async_result,
        OperationStartResult::Async(OperationToken::new("tok-1"))
    );
    assert_eq!(handler.starts.load(Ordering::SeqCst), 2);
}

#[test]
fn cancel_context_exposes_operation_token() {
    let handler = CountingHandler::default();
    let context = cancel_context("cancel-me");

    block_on(handler.cancel(context)).expect("cancel failed");

    assert_eq!(handler.cancels.load(Ordering::SeqCst), 1);
    assert_eq!(
        handler.seen_token_len.load(Ordering::SeqCst),
        "cancel-me".len()
    );
}

#[test]
fn unit_input_and_output_are_supported() {
    let handler = UnitHandler;

    let result = block_on(handler.start(start_context(), ())).expect("unit start failed");

    assert_eq!(result, OperationStartResult::Sync(()));
    block_on(handler.cancel(cancel_context("unit-token"))).expect("unit cancel failed");
}

#[test]
fn handler_futures_are_send() {
    let handler = Arc::new(CountingHandler::default());

    let start_future = assert_send(handler.start(start_context(), String::from("sendable")));
    let cancel_future = assert_send(handler.cancel(cancel_context("sendable-token")));

    let _ = block_on(start_future).expect("start future failed");
    block_on(cancel_future).expect("cancel future failed");
}

#[test]
fn ordinary_async_fn_impls_compile_without_helper_macros() {
    struct AsyncOnly;

    impl OperationHandler<u8, u8> for AsyncOnly {
        async fn start(
            &self,
            _context: OperationStartContext,
            input: u8,
        ) -> Result<OperationStartResult<u8>, BoxError> {
            Ok(OperationStartResult::Sync(input + 1))
        }

        async fn cancel(&self, _context: OperationCancelContext) -> Result<(), BoxError> {
            Ok(())
        }
    }

    let handler = AsyncOnly;
    let result = block_on(handler.start(start_context(), 41)).expect("async impl failed");

    assert_eq!(result, OperationStartResult::Sync(42));
}

#[test]
fn references_forward_to_underlying_handler() {
    let handler = CountingHandler::default();
    let borrowed = &handler;

    let result = block_on(borrowed.start(start_context(), String::from("borrowed")))
        .expect("borrowed start failed");

    assert_eq!(result, OperationStartResult::Sync(String::from("borrowed")));
}
