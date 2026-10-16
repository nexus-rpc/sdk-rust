use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Serialize};

/// An owned error that can be retained as the cause of a Nexus failure.
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// A portable failure payload retaining opaque wire metadata and details.
#[non_exhaustive]
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FailureInfo {
    pub message: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<Box<FailureInfo>>,
}

impl FailureInfo {
    /// Creates a failure payload without interpreting its message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Self::default()
        }
    }
}

/// Protocol error type for a handler-level failure, distinct from an operation's outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandlerErrorType {
    Unknown,
    BadRequest,
    Unauthenticated,
    Unauthorized,
    NotFound,
    RequestTimeout,
    Conflict,
    ResourceExhausted,
    Internal,
    NotImplemented,
    Unavailable,
    UpstreamTimeout,
}

impl HandlerErrorType {
    /// Returns the protocol's default retry behavior for this error type.
    pub const fn default_retry_behavior(self) -> HandlerErrorRetryBehavior {
        match self {
            Self::Unknown
            | Self::RequestTimeout
            | Self::ResourceExhausted
            | Self::Internal
            | Self::Unavailable
            | Self::UpstreamTimeout => HandlerErrorRetryBehavior::Retryable,
            Self::BadRequest
            | Self::Unauthenticated
            | Self::Unauthorized
            | Self::NotFound
            | Self::Conflict
            | Self::NotImplemented => HandlerErrorRetryBehavior::NonRetryable,
        }
    }

    /// Returns the canonical protocol wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::BadRequest => "BAD_REQUEST",
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::NotFound => "NOT_FOUND",
            Self::RequestTimeout => "REQUEST_TIMEOUT",
            Self::Conflict => "CONFLICT",
            Self::ResourceExhausted => "RESOURCE_EXHAUSTED",
            Self::Internal => "INTERNAL",
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::Unavailable => "UNAVAILABLE",
            Self::UpstreamTimeout => "UPSTREAM_TIMEOUT",
        }
    }

    /// Recognizes canonical, case-sensitive protocol wire names.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        match name {
            "UNKNOWN" => Some(Self::Unknown),
            "BAD_REQUEST" => Some(Self::BadRequest),
            "UNAUTHENTICATED" => Some(Self::Unauthenticated),
            "UNAUTHORIZED" => Some(Self::Unauthorized),
            "NOT_FOUND" => Some(Self::NotFound),
            "REQUEST_TIMEOUT" => Some(Self::RequestTimeout),
            "CONFLICT" => Some(Self::Conflict),
            "RESOURCE_EXHAUSTED" => Some(Self::ResourceExhausted),
            "INTERNAL" => Some(Self::Internal),
            "NOT_IMPLEMENTED" => Some(Self::NotImplemented),
            "UNAVAILABLE" => Some(Self::Unavailable),
            "UPSTREAM_TIMEOUT" => Some(Self::UpstreamTimeout),
            _ => None,
        }
    }
}

impl Display for HandlerErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Retry policy for a handler failure, independent of the underlying source error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandlerErrorRetryBehavior {
    /// Use the error type's protocol default.
    Unspecified,
    /// The request may be retried.
    Retryable,
    /// The request should not be retried.
    NonRetryable,
}

/// A handler-level failure with protocol classification and optional application cause.
#[derive(Debug, thiserror::Error)]
#[error("{raw_error_type}: {message}")]
pub struct HandlerError {
    raw_error_type: String,
    message: String,
    retry_behavior: HandlerErrorRetryBehavior,
    source: Option<BoxError>,
    stack_trace: Option<String>,
    original_failure: Option<Box<FailureInfo>>,
}

impl HandlerError {
    /// Creates a handler failure using the error type's default retry behavior.
    pub fn new(error_type: HandlerErrorType, message: impl Into<String>) -> Self {
        Self::from_raw_type(error_type.as_str(), message)
    }

    /// Preserves unrecognized protocol codes so they can be forwarded unchanged.
    pub fn from_raw_type(raw_error_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            raw_error_type: raw_error_type.into(),
            message: message.into(),
            retry_behavior: HandlerErrorRetryBehavior::Unspecified,
            source: None,
            stack_trace: None,
            original_failure: None,
        }
    }

    /// Overrides the error type's default retry behavior.
    pub fn with_retry_behavior(mut self, retry_behavior: HandlerErrorRetryBehavior) -> Self {
        self.retry_behavior = retry_behavior;
        self
    }

    /// Attaches the original cause without changing the protocol classification.
    pub fn with_source(mut self, source: impl Into<BoxError>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Attaches an explicit stack trace independently of the application source.
    pub fn with_stack_trace(mut self, stack_trace: impl Into<String>) -> Self {
        self.stack_trace = Some(stack_trace.into());
        self
    }

    /// Retains the wire failure without replacing the application source.
    pub fn with_original_failure(mut self, original_failure: FailureInfo) -> Self {
        self.original_failure = Some(Box::new(original_failure));
        self
    }

    /// Returns the recognized protocol type, or [`HandlerErrorType::Unknown`].
    pub fn error_type(&self) -> HandlerErrorType {
        HandlerErrorType::from_wire_name(self.raw_error_type()).unwrap_or(HandlerErrorType::Unknown)
    }

    /// Returns the original protocol code, including unrecognized codes.
    pub fn raw_error_type(&self) -> &str {
        &self.raw_error_type
    }

    /// Returns the explicit stack trace.
    pub fn stack_trace(&self) -> Option<&str> {
        self.stack_trace.as_deref()
    }

    /// Returns the retained wire failure.
    pub fn original_failure(&self) -> Option<&FailureInfo> {
        self.original_failure.as_deref()
    }

    /// Returns the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the retry override, or [`HandlerErrorRetryBehavior::Unspecified`].
    pub const fn retry_behavior(&self) -> HandlerErrorRetryBehavior {
        self.retry_behavior
    }

    /// Resolves an unspecified retry policy using the protocol error type.
    pub fn effective_retry_behavior(&self) -> HandlerErrorRetryBehavior {
        match self.retry_behavior {
            HandlerErrorRetryBehavior::Unspecified => self.error_type().default_retry_behavior(),
            explicit => explicit,
        }
    }
}

/// A terminal operation outcome, distinct from a handler-level failure.
#[derive(Debug, thiserror::Error)]
#[error("{state}: {message}")]
pub struct OperationError {
    state: OperationErrorState,
    message: String,
    source: Option<BoxError>,
    stack_trace: Option<String>,
    original_failure: Option<Box<FailureInfo>>,
}

impl OperationError {
    /// Creates a terminal failed operation error.
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            state: OperationErrorState::Failed,
            message: message.into(),
            source: None,
            stack_trace: None,
            original_failure: None,
        }
    }

    /// Creates a terminal canceled operation error.
    pub fn canceled(message: impl Into<String>) -> Self {
        Self {
            state: OperationErrorState::Canceled,
            message: message.into(),
            source: None,
            stack_trace: None,
            original_failure: None,
        }
    }

    /// Attaches the original cause without changing the terminal state.
    pub fn with_source(mut self, source: impl Into<BoxError>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Attaches an explicit stack trace independently of the application source.
    pub fn with_stack_trace(mut self, stack_trace: impl Into<String>) -> Self {
        self.stack_trace = Some(stack_trace.into());
        self
    }

    /// Retains the wire failure without replacing the application source.
    pub fn with_original_failure(mut self, original_failure: FailureInfo) -> Self {
        self.original_failure = Some(Box::new(original_failure));
        self
    }

    /// Returns the explicit stack trace.
    pub fn stack_trace(&self) -> Option<&str> {
        self.stack_trace.as_deref()
    }

    /// Returns the retained wire failure.
    pub fn original_failure(&self) -> Option<&FailureInfo> {
        self.original_failure.as_deref()
    }

    /// Returns the operation error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the terminal operation state.
    pub const fn state(&self) -> OperationErrorState {
        self.state
    }
}

/// Terminal state represented by an [`OperationError`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationErrorState {
    /// The operation completed unsuccessfully.
    Failed,
    /// The operation was canceled.
    Canceled,
}

impl Display for OperationErrorState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed => f.write_str("failed"),
            Self::Canceled => f.write_str("canceled"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("application failure {detail}")]
    struct ApplicationError {
        detail: String,
        #[source]
        cause: std::io::Error,
    }

    #[test]
    fn handler_preserves_boxed_source_chain() {
        let source: BoxError = Box::new(ApplicationError {
            detail: "request 42".into(),
            cause: std::io::Error::other("disk unavailable"),
        });
        let error = HandlerError::from_raw_type("Future_error/42", "handler failed");
        assert_eq!(error.raw_error_type(), "Future_error/42");
        assert_eq!(error.error_type(), HandlerErrorType::Unknown);
        assert_eq!(
            error.effective_retry_behavior(),
            HandlerErrorRetryBehavior::Retryable
        );
        let error = error
            .with_retry_behavior(HandlerErrorRetryBehavior::NonRetryable)
            .with_source(source);
        assert_eq!(
            error.effective_retry_behavior(),
            HandlerErrorRetryBehavior::NonRetryable
        );
        assert_eq!(error.to_string(), "Future_error/42: handler failed");
        let cause = error.source().expect("application cause");
        assert!(cause.downcast_ref::<ApplicationError>().is_some());
        assert_eq!(
            cause.source().expect("root cause").to_string(),
            "disk unavailable"
        );
    }

    #[test]
    fn handler_wire_names_are_case_sensitive() {
        assert_eq!(
            HandlerErrorType::from_wire_name("UNKNOWN"),
            Some(HandlerErrorType::Unknown)
        );
        assert_eq!(HandlerErrorType::from_wire_name("unavailable"), None);
        let error = HandlerError::new(HandlerErrorType::Unavailable, "offline");
        assert_eq!(error.raw_error_type(), "UNAVAILABLE");
        assert_eq!(error.error_type(), HandlerErrorType::Unavailable);
    }

    #[test]
    fn original_failure_roundtrips_without_interpreting_details() {
        let wire = serde_json::json!({
            "message": "original failure",
            "metadata": {"encoding": "opaque/custom", "extra": "retained"},
            "details": "not JSON: \u{0000}+/=",
            "stack_trace": "original stack",
            "cause": {
                "message": "nested failure",
                "details": "{\"opaque\":true}"
            }
        });
        let original: FailureInfo = serde_json::from_value(wire.clone()).expect("wire failure");
        assert_eq!(
            serde_json::to_value(&original).expect("serialized failure"),
            wire
        );
        assert_eq!(
            serde_json::to_value(FailureInfo::new("minimal")).expect("minimal failure"),
            serde_json::json!({"message": "minimal"})
        );
        assert_eq!(
            serde_json::from_str::<FailureInfo>("{}").expect("default failure"),
            FailureInfo::default()
        );

        let handler = HandlerError::new(HandlerErrorType::Internal, "handler failure")
            .with_original_failure(original.clone())
            .with_stack_trace("explicit handler stack")
            .with_source(std::io::Error::other("handler source"));
        let operation = OperationError::failed("operation failure")
            .with_stack_trace("explicit operation stack")
            .with_source(std::io::Error::other("operation source"))
            .with_original_failure(original.clone());
        assert_eq!(handler.stack_trace(), Some("explicit handler stack"));
        assert_eq!(operation.stack_trace(), Some("explicit operation stack"));
        assert_eq!(handler.original_failure(), Some(&original));
        assert_eq!(operation.original_failure(), Some(&original));
        assert_eq!(
            handler.source().expect("handler source").to_string(),
            "handler source"
        );
        assert_eq!(
            operation.source().expect("operation source").to_string(),
            "operation source"
        );
    }

    #[test]
    fn operation_preserves_source_chain() {
        let error = OperationError::canceled("operation canceled").with_source(ApplicationError {
            detail: "request 17".into(),
            cause: std::io::Error::other("cancellation requested"),
        });
        let source = error
            .source()
            .expect("application cause")
            .downcast_ref::<ApplicationError>()
            .expect("original application error");
        assert_eq!(source.detail, "request 17");
        assert_eq!(source.cause.to_string(), "cancellation requested");
    }
}
