use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    time::{Duration, SystemTime},
};

use nexus_rpc::{
    Headers, Link, OperationCancelContext, OperationStartContext, OperationToken,
    integration::RequestCanceller,
};

#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn assert_send_sync<T: Send + Sync>(_: &T) {}

fn assert_send<T: Send>(_: T) {}

#[test]
fn headers_normalize_names_and_replace_case_insensitively() {
    let mut headers = Headers::new();

    assert_eq!(headers.insert("Content-TYPE", "Application/JSON"), None);
    assert_eq!(headers.get("content-type"), Some("Application/JSON"));
    assert_eq!(headers.get("CONTENT-TYPE"), Some("Application/JSON"));
    assert!(headers.contains_key("CoNtEnT-TyPe"));

    assert_eq!(
        headers.insert("content-type", "Text/Plain"),
        Some(String::from("Application/JSON"))
    );
    assert_eq!(headers.len(), 1);
    assert_eq!(
        headers.iter().collect::<Vec<_>>(),
        vec![("content-type", "Text/Plain")]
    );
    assert_eq!(
        headers.remove("CONTENT-type"),
        Some(String::from("Text/Plain"))
    );
    assert!(headers.is_empty());
}

#[test]
fn context_builders_expose_all_fields_and_defaults() {
    let empty_start = OperationStartContext::builder("payments", "charge", "req-empty").build();
    assert_eq!(empty_start.service(), "payments");
    assert_eq!(empty_start.operation(), "charge");
    assert_eq!(empty_start.request_id(), "req-empty");
    assert!(empty_start.headers().is_empty());
    assert_eq!(empty_start.request_deadline(), None);
    assert!(!empty_start.request_cancellation().is_cancelled());
    assert_eq!(empty_start.callback_url(), None);
    assert!(empty_start.callback_headers().is_empty());
    assert!(empty_start.inbound_links().is_empty());
    assert!(empty_start.outbound_links_snapshot().is_empty());

    let deadline = SystemTime::UNIX_EPOCH + Duration::from_secs(12_345);
    let canceller = RequestCanceller::new();
    let inbound = Link::new("https://example.test/inbound", "example.inbound");
    let outbound = Link::new("https://example.test/outbound", "example.outbound");
    let start = OperationStartContext::builder("payments", "charge", "req-1")
        .header("Trace-ID", "TraceValue")
        .request_deadline(deadline)
        .request_cancellation(canceller.cancellation())
        .callback_url("https://callback.test/complete")
        .callback_header("Callback-Token", "CallbackSecret")
        .inbound_link(inbound.clone())
        .outbound_link(outbound.clone())
        .build();

    assert_eq!(start.service(), "payments");
    assert_eq!(start.operation(), "charge");
    assert_eq!(start.request_id(), "req-1");
    assert_eq!(start.headers().get("TRACE-id"), Some("TraceValue"));
    assert_eq!(start.request_deadline(), Some(deadline));
    assert_eq!(start.callback_url(), Some("https://callback.test/complete"));
    assert_eq!(
        start.callback_headers().get("CALLBACK-token"),
        Some("CallbackSecret")
    );
    assert_eq!(start.inbound_links(), &[inbound]);
    assert_eq!(start.outbound_links_snapshot(), vec![outbound]);

    let token = OperationToken::new("operation-secret");
    let empty_cancel = OperationCancelContext::builder("payments", "charge", token.clone()).build();
    assert_eq!(empty_cancel.service(), "payments");
    assert_eq!(empty_cancel.operation(), "charge");
    assert_eq!(empty_cancel.operation_token(), &token);
    assert!(empty_cancel.headers().is_empty());
    assert_eq!(empty_cancel.request_deadline(), None);
    assert!(!empty_cancel.request_cancellation().is_cancelled());

    let cancel = OperationCancelContext::builder("payments", "charge", token.clone())
        .header("Trace-ID", "TraceValue")
        .request_deadline(deadline)
        .request_cancellation(canceller.cancellation())
        .build();
    assert_eq!(cancel.headers().get("trace-id"), Some("TraceValue"));
    assert_eq!(cancel.request_deadline(), Some(deadline));
    assert_eq!(cancel.operation_token(), &token);
}

#[test]
fn link_is_a_raw_transport_independent_value() {
    let link = Link::new("not necessarily a URL", "custom/type");

    assert_eq!(link.url(), "not necessarily a URL");
    assert_eq!(link.link_type(), "custom/type");
}

#[test]
fn cloned_start_context_shares_live_state_but_snapshots_are_independent() {
    let canceller = RequestCanceller::new();
    let first = Link::new("https://example.test/1", "example");
    let second = Link::new("https://example.test/2", "example");
    let replacement = Link::new("https://example.test/replacement", "example");
    let context = OperationStartContext::builder("service", "operation", "request")
        .request_cancellation(canceller.cancellation())
        .outbound_link(first.clone())
        .build();
    let clone = context.clone();

    context.extend_outbound_links([second.clone(), second.clone()]);
    assert_eq!(
        clone.outbound_links_snapshot(),
        vec![first, second.clone(), second]
    );

    let mut snapshot = clone.outbound_links_snapshot();
    snapshot.clear();
    assert_eq!(context.outbound_links_snapshot().len(), 3);

    clone.set_outbound_links([replacement.clone()]);
    assert_eq!(context.outbound_links_snapshot(), vec![replacement]);

    assert!(canceller.cancel("worker shutdown"));
    assert!(context.request_cancellation().is_cancelled());
    assert_eq!(
        clone.request_cancellation().reason().as_deref(),
        Some("worker shutdown")
    );

    let cancel_canceller = RequestCanceller::new();
    let cancel_context = OperationCancelContext::builder(
        "service",
        "operation",
        OperationToken::new("operation-token"),
    )
    .request_cancellation(cancel_canceller.cancellation())
    .build();
    let cancel_clone = cancel_context.clone();
    assert!(cancel_canceller.cancel("cancel request disconnected"));
    assert_eq!(
        cancel_clone.request_cancellation().reason().as_deref(),
        Some("cancel request disconnected")
    );
}

#[test]
fn public_context_and_cancellation_types_have_thread_safe_traits() {
    let start = OperationStartContext::builder("service", "operation", "request").build();
    let cancel =
        OperationCancelContext::builder("service", "operation", OperationToken::new("token"))
            .build();
    let canceller = RequestCanceller::new();
    let cancellation = canceller.cancellation();

    assert_send_sync(&start);
    assert_send_sync(&cancel);
    assert_send_sync(&cancellation);
    assert_send_sync(&canceller);
    assert_send_sync(&Headers::new());
    assert_send_sync(&Link::new("url", "type"));
    assert_send(cancellation.cancelled());
    let _ = start.clone();
    let _ = cancel.clone();
    let _ = format!("{start:?} {cancel:?} {cancellation:?}");
}

#[test]
fn context_debug_redacts_header_callback_link_and_token_values() {
    let start = OperationStartContext::builder("service", "operation", "request")
        .header("secret-header-name", "secret-header-value")
        .callback_url("https://secret.callback.test")
        .callback_header("secret-callback-name", "secret-callback-value")
        .inbound_link(Link::new(
            "https://secret.inbound.test",
            "secret-inbound-type",
        ))
        .outbound_link(Link::new(
            "https://secret.outbound.test",
            "secret-outbound-type",
        ))
        .build();
    let cancel = OperationCancelContext::builder(
        "service",
        "operation",
        OperationToken::new("secret-operation-token"),
    )
    .header("secret-cancel-name", "secret-cancel-value")
    .build();

    let start_debug = format!("{start:?}");
    let cancel_debug = format!("{cancel:?}");
    for secret in [
        "secret-header-name",
        "secret-header-value",
        "https://secret.callback.test",
        "secret-callback-name",
        "secret-callback-value",
        "https://secret.inbound.test",
        "secret-inbound-type",
        "https://secret.outbound.test",
        "secret-outbound-type",
    ] {
        assert!(!start_debug.contains(secret), "debug leaked {secret}");
    }
    for secret in [
        "secret-operation-token",
        "secret-cancel-name",
        "secret-cancel-value",
    ] {
        assert!(!cancel_debug.contains(secret), "debug leaked {secret}");
    }
}

#[test]
fn supporting_type_debug_redacts_raw_values() {
    let mut headers = Headers::new();
    headers.insert("secret-header-name", "secret-header-value");
    let link = Link::new("https://secret.link.test", "secret-link-type");
    let token = OperationToken::new("secret-operation-token");

    let debug = format!("{headers:?} {link:?} {token:?}");
    for secret in [
        "secret-header-name",
        "secret-header-value",
        "https://secret.link.test",
        "secret-link-type",
        "secret-operation-token",
    ] {
        assert!(!debug.contains(secret), "debug leaked {secret}");
    }
}

#[test]
fn cancellation_wakes_a_pending_waiter_and_is_immediate_afterward() {
    let canceller = RequestCanceller::new();
    let cancellation = canceller.cancellation();
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut task_context = Context::from_waker(&waker);
    let mut pending = pin!(cancellation.cancelled());

    assert_eq!(pending.as_mut().poll(&mut task_context), Poll::Pending);
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);
    assert!(canceller.cancel("request deadline exceeded"));
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 1);
    assert_eq!(pending.as_mut().poll(&mut task_context), Poll::Ready(()));
    assert!(cancellation.is_cancelled());
    assert_eq!(
        cancellation.cancellation_reason().as_deref(),
        Some("request deadline exceeded")
    );

    let mut already_cancelled = pin!(cancellation.cancelled());
    assert_eq!(
        already_cancelled.as_mut().poll(&mut task_context),
        Poll::Ready(())
    );
}

#[test]
fn cancellation_wakes_multiple_waiters() {
    let canceller = RequestCanceller::new();
    let first_cancellation = canceller.cancellation();
    let second_cancellation = canceller.cancellation();
    let first_counter = Arc::new(WakeCounter::default());
    let second_counter = Arc::new(WakeCounter::default());
    let first_waker = Waker::from(Arc::clone(&first_counter));
    let second_waker = Waker::from(Arc::clone(&second_counter));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    let mut first = pin!(first_cancellation.cancelled());
    let mut second = pin!(second_cancellation.cancelled());

    assert_eq!(first.as_mut().poll(&mut first_context), Poll::Pending);
    assert_eq!(second.as_mut().poll(&mut second_context), Poll::Pending);
    assert!(canceller.cancel("disconnected"));
    assert_eq!(first_counter.0.load(Ordering::SeqCst), 1);
    assert_eq!(second_counter.0.load(Ordering::SeqCst), 1);
    assert_eq!(first.as_mut().poll(&mut first_context), Poll::Ready(()));
    assert_eq!(second.as_mut().poll(&mut second_context), Poll::Ready(()));
}

#[test]
fn dropped_cancellation_waiter_releases_its_waker() {
    let canceller = RequestCanceller::new();
    let cancellation = canceller.cancellation();
    let wake_counter = Arc::new(WakeCounter::default());
    let weak_counter = Arc::downgrade(&wake_counter);

    {
        let waker = Waker::from(Arc::clone(&wake_counter));
        let mut task_context = Context::from_waker(&waker);
        let mut pending = pin!(cancellation.cancelled());
        assert_eq!(pending.as_mut().poll(&mut task_context), Poll::Pending);
    }
    drop(wake_counter);

    assert!(weak_counter.upgrade().is_none());
    assert!(canceller.cancel("cleanup"));
}

#[test]
fn first_cancellation_reason_wins_even_under_a_concurrent_race() {
    const THREADS: usize = 16;
    let canceller = RequestCanceller::new();
    let cancellation = canceller.cancellation();
    let barrier = Arc::new(Barrier::new(THREADS));
    let mut threads = Vec::new();

    for index in 0..THREADS {
        let thread_canceller = canceller.clone();
        let thread_barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            thread_barrier.wait();
            let reason = format!("reason-{index}");
            (reason.clone(), thread_canceller.cancel(reason))
        }));
    }

    let results: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().expect("cancellation thread panicked"))
        .collect();
    let winners: Vec<_> = results
        .iter()
        .filter(|(_, won)| *won)
        .map(|(reason, _)| reason.as_str())
        .collect();

    assert_eq!(winners.len(), 1);
    assert_eq!(cancellation.reason().as_deref(), winners.first().copied());
    assert!(!canceller.cancel("too late"));
    assert_eq!(cancellation.reason().as_deref(), winners.first().copied());
}
