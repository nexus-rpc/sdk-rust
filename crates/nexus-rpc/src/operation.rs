use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Formatter},
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll, Waker},
    time::SystemTime,
};

use crate::BoxError;

/// Request or callback headers with case-insensitive names.
///
/// Names are normalized to ASCII lowercase when inserted. Inserting a name
/// that differs from an existing name only by ASCII case replaces its value.
/// Header values are preserved unchanged.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Headers {
    values: BTreeMap<String, String>,
}

impl Debug for Headers {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Headers")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

impl Headers {
    /// Creates an empty header collection.
    pub const fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Inserts a header and returns the previous value, if present.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.values
            .insert(normalize_header_name(name.into()), value.into())
    }

    /// Returns a header value without regard to ASCII case in its name.
    pub fn get(&self, name: impl AsRef<str>) -> Option<&str> {
        self.values
            .get(&normalize_header_name(name.as_ref().to_owned()))
            .map(String::as_str)
    }

    /// Returns whether a header exists without regard to ASCII case in its name.
    pub fn contains_key(&self, name: impl AsRef<str>) -> bool {
        self.values
            .contains_key(&normalize_header_name(name.as_ref().to_owned()))
    }

    /// Removes a header without regard to ASCII case in its name.
    pub fn remove(&mut self, name: impl AsRef<str>) -> Option<String> {
        self.values
            .remove(&normalize_header_name(name.as_ref().to_owned()))
    }

    /// Returns the number of headers.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true when there are no headers.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Iterates over normalized header names and their unchanged values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }
}

impl<K, V> Extend<(K, V)> for Headers
where
    K: Into<String>,
    V: Into<String>,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (K, V)>,
    {
        for (name, value) in iter {
            self.insert(name, value);
        }
    }
}

impl<K, V> FromIterator<(K, V)> for Headers
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        let mut headers = Self::new();
        headers.extend(iter);
        headers
    }
}

impl<'a> IntoIterator for &'a Headers {
    type Item = (&'a str, &'a str);
    type IntoIter = std::iter::Map<
        std::collections::btree_map::Iter<'a, String, String>,
        fn((&'a String, &'a String)) -> (&'a str, &'a str),
    >;

    fn into_iter(self) -> Self::IntoIter {
        fn as_strs<'a>((name, value): (&'a String, &'a String)) -> (&'a str, &'a str) {
            (name.as_str(), value.as_str())
        }

        self.values.iter().map(as_strs)
    }
}

fn normalize_header_name(name: String) -> String {
    name.to_ascii_lowercase()
}

/// A raw Nexus link URL and the type used to interpret it.
///
/// This transport-independent value object does not validate or parse URLs.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Link {
    url: String,
    link_type: String,
}

impl Debug for Link {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Link").finish_non_exhaustive()
    }
}

impl Link {
    /// Creates a link from its raw URL and type.
    pub fn new(url: impl Into<String>, link_type: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            link_type: link_type.into(),
        }
    }

    /// Returns the raw link URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the type used to interpret the link URL.
    pub fn link_type(&self) -> &str {
        &self.link_type
    }
}

#[derive(Default)]
struct CancellationState {
    reason: Option<Arc<str>>,
    next_waiter_id: usize,
    waiters: Vec<RegisteredWaiter>,
}

struct RegisteredWaiter {
    id: usize,
    waker: Waker,
}

#[derive(Default)]
struct CancellationInner {
    state: Mutex<CancellationState>,
}

impl CancellationInner {
    fn lock(&self) -> MutexGuard<'_, CancellationState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Read-only notification that the current handler request was canceled.
///
/// Request cancellation applies only to the current invocation. It is distinct
/// from an explicit request to cancel an asynchronously running Nexus operation.
#[derive(Clone)]
pub struct RequestCancellation {
    inner: Arc<CancellationInner>,
}

impl RequestCancellation {
    fn pending() -> Self {
        Self {
            inner: Arc::new(CancellationInner::default()),
        }
    }

    /// Returns true once the current handler request has been canceled.
    pub fn is_cancelled(&self) -> bool {
        self.inner.lock().reason.is_some()
    }

    /// Returns the first cancellation reason, if the request was canceled.
    pub fn cancellation_reason(&self) -> Option<Arc<str>> {
        self.inner.lock().reason.clone()
    }

    /// Returns the first cancellation reason, if the request was canceled.
    pub fn reason(&self) -> Option<Arc<str>> {
        self.cancellation_reason()
    }

    /// Waits until the current handler request is canceled.
    ///
    /// Multiple tasks may wait concurrently. A waiter created after
    /// cancellation completes immediately.
    pub fn cancelled(&self) -> impl Future<Output = ()> + Send + '_ {
        Cancelled {
            cancellation: self,
            waiter_id: None,
        }
    }
}

struct Cancelled<'a> {
    cancellation: &'a RequestCancellation,
    waiter_id: Option<usize>,
}

impl Future for Cancelled<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut state = this.cancellation.inner.lock();
        if state.reason.is_some() {
            this.waiter_id = None;
            return Poll::Ready(());
        }

        if let Some(waiter_id) = this.waiter_id
            && let Some(waiter) = state
                .waiters
                .iter_mut()
                .find(|waiter| waiter.id == waiter_id)
        {
            if !waiter.waker.will_wake(context.waker()) {
                waiter.waker.clone_from(context.waker());
            }
            return Poll::Pending;
        }

        let waiter_id = next_waiter_id(&mut state);
        state.waiters.push(RegisteredWaiter {
            id: waiter_id,
            waker: context.waker().clone(),
        });
        this.waiter_id = Some(waiter_id);
        Poll::Pending
    }
}

impl Drop for Cancelled<'_> {
    fn drop(&mut self) {
        if let Some(waiter_id) = self.waiter_id {
            self.cancellation
                .inner
                .lock()
                .waiters
                .retain(|waiter| waiter.id != waiter_id);
        }
    }
}

fn next_waiter_id(state: &mut CancellationState) -> usize {
    loop {
        let candidate = state.next_waiter_id;
        state.next_waiter_id = state.next_waiter_id.wrapping_add(1);
        if state.waiters.iter().all(|waiter| waiter.id != candidate) {
            return candidate;
        }
    }
}

impl Debug for RequestCancellation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RequestCancellation")
            .field("is_cancelled", &self.is_cancelled())
            .finish_non_exhaustive()
    }
}

/// Framework-side authority used to cancel a handler request.
///
/// Handlers receive only the corresponding [`RequestCancellation`].
#[derive(Clone)]
pub struct RequestCanceller {
    cancellation: RequestCancellation,
}

impl RequestCanceller {
    /// Creates a pending request cancellation pair.
    pub fn new() -> Self {
        Self {
            cancellation: RequestCancellation::pending(),
        }
    }

    /// Returns the read-only cancellation notification for a handler context.
    pub fn cancellation(&self) -> RequestCancellation {
        self.cancellation.clone()
    }

    /// Cancels the request with a reason.
    ///
    /// The first cancellation reason wins. Returns true when this call caused
    /// cancellation and false when the request was already canceled.
    pub fn cancel(&self, reason: impl Into<String>) -> bool {
        let reason = Arc::<str>::from(reason.into());
        let waiters = {
            let mut state = self.cancellation.inner.lock();
            if state.reason.is_some() {
                return false;
            }
            state.reason = Some(reason);
            std::mem::take(&mut state.waiters)
        };

        for waiter in waiters {
            waiter.waker.wake();
        }
        true
    }
}

impl Default for RequestCanceller {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for RequestCanceller {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RequestCanceller")
            .field("is_cancelled", &self.cancellation.is_cancelled())
            .finish_non_exhaustive()
    }
}

/// Opaque token identifying an asynchronously running operation.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct OperationToken(Arc<str>);

impl OperationToken {
    /// Creates a new opaque operation token.
    pub fn new(token: impl Into<String>) -> Self {
        Self(Arc::<str>::from(token.into()))
    }

    /// Returns the token as a borrowed string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for OperationToken {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("OperationToken")
            .field(&"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
struct CommonContext {
    service: String,
    operation: String,
    headers: Headers,
    request_deadline: Option<SystemTime>,
    request_cancellation: RequestCancellation,
}

impl CommonContext {
    fn from_builder(builder: CommonContextBuilder) -> Self {
        Self {
            service: builder.service,
            operation: builder.operation,
            headers: builder.headers,
            request_deadline: builder.request_deadline,
            request_cancellation: builder.request_cancellation,
        }
    }
}

struct CommonContextBuilder {
    service: String,
    operation: String,
    headers: Headers,
    request_deadline: Option<SystemTime>,
    request_cancellation: RequestCancellation,
}

impl CommonContextBuilder {
    fn new(service: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            operation: operation.into(),
            headers: Headers::new(),
            request_deadline: None,
            request_cancellation: RequestCancellation::pending(),
        }
    }
}

/// Context provided to [`OperationHandler::start`].
///
/// Construct this context with [`OperationStartContext::builder`]. Its private
/// fields allow additional request metadata to be added without exposing struct
/// construction as part of the public API.
///
/// Clones share request-cancellation and outbound-link state. This allows an
/// integration to retain a clone and collect links after a handler consumes its
/// context. All other request metadata is immutable after construction.
#[derive(Clone)]
pub struct OperationStartContext {
    common: CommonContext,
    request_id: String,
    callback_url: Option<String>,
    callback_headers: Headers,
    inbound_links: Vec<Link>,
    outbound_links: Arc<Mutex<Vec<Link>>>,
}

impl OperationStartContext {
    /// Creates a builder with the required request identity fields.
    pub fn builder(
        service: impl Into<String>,
        operation: impl Into<String>,
        request_id: impl Into<String>,
    ) -> OperationStartContextBuilder {
        OperationStartContextBuilder::new(service, operation, request_id)
    }

    /// Returns the service name for this call.
    pub fn service(&self) -> &str {
        &self.common.service
    }

    /// Returns the operation name for this call.
    pub fn operation(&self) -> &str {
        &self.common.operation
    }

    /// Returns the immutable request headers.
    pub const fn headers(&self) -> &Headers {
        &self.common.headers
    }

    /// Returns the deadline for this handler invocation, if supplied.
    ///
    /// This is the request deadline, not the asynchronously running operation's
    /// deadline.
    pub const fn request_deadline(&self) -> Option<SystemTime> {
        self.common.request_deadline
    }

    /// Returns read-only cancellation information for this handler invocation.
    pub const fn request_cancellation(&self) -> &RequestCancellation {
        &self.common.request_cancellation
    }

    /// Returns the unique request identifier used for start deduplication.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Returns the callback URL for asynchronous completion, if supplied.
    pub fn callback_url(&self) -> Option<&str> {
        self.callback_url.as_deref()
    }

    /// Returns immutable headers to send to the callback URL.
    pub const fn callback_headers(&self) -> &Headers {
        &self.callback_headers
    }

    /// Returns immutable links supplied by the caller.
    pub fn inbound_links(&self) -> &[Link] {
        &self.inbound_links
    }

    /// Adds one outbound link while preserving insertion order and duplicates.
    pub fn add_outbound_link(&self, link: Link) {
        lock_unpoisoned(&self.outbound_links).push(link);
    }

    /// Adds outbound links while preserving insertion order and duplicates.
    pub fn extend_outbound_links(&self, links: impl IntoIterator<Item = Link>) {
        let links: Vec<_> = links.into_iter().collect();
        lock_unpoisoned(&self.outbound_links).extend(links);
    }

    /// Replaces all outbound links.
    pub fn set_outbound_links(&self, links: impl IntoIterator<Item = Link>) {
        let links: Vec<_> = links.into_iter().collect();
        let mut outbound_links = lock_unpoisoned(&self.outbound_links);
        outbound_links.clear();
        outbound_links.extend(links);
    }

    /// Returns an independent snapshot of the current outbound links.
    ///
    /// Outbound-link state is shared by clones of this context, while each
    /// returned snapshot can be mutated independently.
    pub fn outbound_links_snapshot(&self) -> Vec<Link> {
        lock_unpoisoned(&self.outbound_links).clone()
    }
}

impl Debug for OperationStartContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationStartContext")
            .field("service", &self.service())
            .field("operation", &self.operation())
            .field("request_id", &self.request_id())
            .field("header_count", &self.headers().len())
            .field("request_deadline", &self.request_deadline())
            .field(
                "request_cancelled",
                &self.request_cancellation().is_cancelled(),
            )
            .field("has_callback", &self.callback_url().is_some())
            .field("callback_header_count", &self.callback_headers().len())
            .field("inbound_link_count", &self.inbound_links().len())
            .field(
                "outbound_link_count",
                &lock_unpoisoned(&self.outbound_links).len(),
            )
            .finish_non_exhaustive()
    }
}

/// Builder for [`OperationStartContext`].
pub struct OperationStartContextBuilder {
    common: CommonContextBuilder,
    request_id: String,
    callback_url: Option<String>,
    callback_headers: Headers,
    inbound_links: Vec<Link>,
    outbound_links: Vec<Link>,
}

impl OperationStartContextBuilder {
    /// Creates a builder with the required request identity fields.
    pub fn new(
        service: impl Into<String>,
        operation: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        Self {
            common: CommonContextBuilder::new(service, operation),
            request_id: request_id.into(),
            callback_url: None,
            callback_headers: Headers::new(),
            inbound_links: Vec::new(),
            outbound_links: Vec::new(),
        }
    }

    /// Sets all immutable request headers.
    pub fn headers(mut self, headers: Headers) -> Self {
        self.common.headers = headers;
        self
    }

    /// Inserts one immutable request header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.common.headers.insert(name, value);
        self
    }

    /// Sets the deadline for this handler invocation.
    pub fn request_deadline(mut self, deadline: SystemTime) -> Self {
        self.common.request_deadline = Some(deadline);
        self
    }

    /// Sets the read-only request cancellation notification.
    pub fn request_cancellation(mut self, cancellation: RequestCancellation) -> Self {
        self.common.request_cancellation = cancellation;
        self
    }

    /// Sets the callback URL for asynchronous completion.
    pub fn callback_url(mut self, callback_url: impl Into<String>) -> Self {
        self.callback_url = Some(callback_url.into());
        self
    }

    /// Sets all immutable callback headers.
    pub fn callback_headers(mut self, headers: Headers) -> Self {
        self.callback_headers = headers;
        self
    }

    /// Inserts one immutable callback header.
    pub fn callback_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.callback_headers.insert(name, value);
        self
    }

    /// Adds one immutable inbound link.
    pub fn inbound_link(mut self, link: Link) -> Self {
        self.inbound_links.push(link);
        self
    }

    /// Adds immutable inbound links.
    pub fn inbound_links(mut self, links: impl IntoIterator<Item = Link>) -> Self {
        self.inbound_links.extend(links);
        self
    }

    /// Adds one initial outbound link.
    pub fn outbound_link(mut self, link: Link) -> Self {
        self.outbound_links.push(link);
        self
    }

    /// Adds initial outbound links.
    pub fn outbound_links(mut self, links: impl IntoIterator<Item = Link>) -> Self {
        self.outbound_links.extend(links);
        self
    }

    /// Builds the context.
    pub fn build(self) -> OperationStartContext {
        OperationStartContext {
            common: CommonContext::from_builder(self.common),
            request_id: self.request_id,
            callback_url: self.callback_url,
            callback_headers: self.callback_headers,
            inbound_links: self.inbound_links,
            outbound_links: Arc::new(Mutex::new(self.outbound_links)),
        }
    }
}

/// Context provided to [`OperationHandler::cancel`].
///
/// Request cancellation on this context describes cancellation of the current
/// cancel invocation and is distinct from the operation cancellation identified
/// by [`Self::operation_token`].
#[derive(Clone)]
pub struct OperationCancelContext {
    common: CommonContext,
    operation_token: OperationToken,
}

impl OperationCancelContext {
    /// Creates a builder with the required call identity and operation token.
    pub fn builder(
        service: impl Into<String>,
        operation: impl Into<String>,
        operation_token: OperationToken,
    ) -> OperationCancelContextBuilder {
        OperationCancelContextBuilder::new(service, operation, operation_token)
    }

    /// Returns the service name for this call.
    pub fn service(&self) -> &str {
        &self.common.service
    }

    /// Returns the operation name for this call.
    pub fn operation(&self) -> &str {
        &self.common.operation
    }

    /// Returns the immutable request headers.
    pub const fn headers(&self) -> &Headers {
        &self.common.headers
    }

    /// Returns the deadline for this handler invocation, if supplied.
    ///
    /// This is the request deadline, not the asynchronously running operation's
    /// deadline.
    pub const fn request_deadline(&self) -> Option<SystemTime> {
        self.common.request_deadline
    }

    /// Returns read-only cancellation information for this handler invocation.
    pub const fn request_cancellation(&self) -> &RequestCancellation {
        &self.common.request_cancellation
    }

    /// Returns the opaque token identifying the running operation.
    pub const fn operation_token(&self) -> &OperationToken {
        &self.operation_token
    }
}

impl Debug for OperationCancelContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationCancelContext")
            .field("service", &self.service())
            .field("operation", &self.operation())
            .field("header_count", &self.headers().len())
            .field("request_deadline", &self.request_deadline())
            .field(
                "request_cancelled",
                &self.request_cancellation().is_cancelled(),
            )
            .finish_non_exhaustive()
    }
}

/// Builder for [`OperationCancelContext`].
pub struct OperationCancelContextBuilder {
    common: CommonContextBuilder,
    operation_token: OperationToken,
}

impl OperationCancelContextBuilder {
    /// Creates a builder with the required call identity and operation token.
    pub fn new(
        service: impl Into<String>,
        operation: impl Into<String>,
        operation_token: OperationToken,
    ) -> Self {
        Self {
            common: CommonContextBuilder::new(service, operation),
            operation_token,
        }
    }

    /// Sets all immutable request headers.
    pub fn headers(mut self, headers: Headers) -> Self {
        self.common.headers = headers;
        self
    }

    /// Inserts one immutable request header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.common.headers.insert(name, value);
        self
    }

    /// Sets the deadline for this handler invocation.
    pub fn request_deadline(mut self, deadline: SystemTime) -> Self {
        self.common.request_deadline = Some(deadline);
        self
    }

    /// Sets the read-only request cancellation notification.
    pub fn request_cancellation(mut self, cancellation: RequestCancellation) -> Self {
        self.common.request_cancellation = cancellation;
        self
    }

    /// Builds the context.
    pub fn build(self) -> OperationCancelContext {
        OperationCancelContext {
            common: CommonContext::from_builder(self.common),
            operation_token: self.operation_token,
        }
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Start result for an operation handler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationStartResult<O> {
    /// The operation completed immediately with a value.
    Sync(O),
    /// The operation continues asynchronously and can be canceled via the token.
    Async(OperationToken),
}

/// Per-operation Nexus handler contract.
pub trait OperationHandler<I, O>: Send + Sync {
    /// Starts the operation.
    fn start(
        &self,
        context: OperationStartContext,
        input: I,
    ) -> impl Future<Output = Result<OperationStartResult<O>, BoxError>> + Send;

    /// Requests cancellation of a previously started operation.
    fn cancel(
        &self,
        context: OperationCancelContext,
    ) -> impl Future<Output = Result<(), BoxError>> + Send;
}

impl<I, O, H> OperationHandler<I, O> for &H
where
    H: OperationHandler<I, O> + ?Sized,
{
    fn start(
        &self,
        context: OperationStartContext,
        input: I,
    ) -> impl Future<Output = Result<OperationStartResult<O>, BoxError>> + Send {
        (**self).start(context, input)
    }

    fn cancel(
        &self,
        context: OperationCancelContext,
    ) -> impl Future<Output = Result<(), BoxError>> + Send {
        (**self).cancel(context)
    }
}
