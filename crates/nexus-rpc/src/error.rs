use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// An owned error that can be retained as the cause of a Nexus failure.
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

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
            Self::Unknown => "unknown",
            Self::BadRequest => "bad_request",
            Self::Unauthenticated => "unauthenticated",
            Self::Unauthorized => "unauthorized",
            Self::NotFound => "not_found",
            Self::RequestTimeout => "request_timeout",
            Self::Conflict => "conflict",
            Self::ResourceExhausted => "resource_exhausted",
            Self::Internal => "internal",
            Self::NotImplemented => "not_implemented",
            Self::Unavailable => "unavailable",
            Self::UpstreamTimeout => "upstream_timeout",
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
#[error("{error_type}: {message}")]
pub struct HandlerError {
    error_type: HandlerErrorType,
    message: String,
    retry_behavior: HandlerErrorRetryBehavior,
    source: Option<BoxError>,
}

impl HandlerError {
    /// Creates a handler failure using the error type's default retry behavior.
    pub fn new(error_type: HandlerErrorType, message: impl Into<String>) -> Self {
        Self {
            error_type,
            message: message.into(),
            retry_behavior: HandlerErrorRetryBehavior::Unspecified,
            source: None,
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

    /// Returns the protocol error type.
    pub const fn error_type(&self) -> HandlerErrorType {
        self.error_type
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
    pub const fn effective_retry_behavior(&self) -> HandlerErrorRetryBehavior {
        match self.retry_behavior {
            HandlerErrorRetryBehavior::Unspecified => self.error_type.default_retry_behavior(),
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
}

impl OperationError {
    /// Creates a terminal failed operation error.
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            state: OperationErrorState::Failed,
            message: message.into(),
            source: None,
        }
    }

    /// Creates a terminal canceled operation error.
    pub fn canceled(message: impl Into<String>) -> Self {
        Self {
            state: OperationErrorState::Canceled,
            message: message.into(),
            source: None,
        }
    }

    /// Attaches the original cause without changing the terminal state.
    pub fn with_source(mut self, source: impl Into<BoxError>) -> Self {
        self.source = Some(source.into());
        self
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
        let error = HandlerError::new(HandlerErrorType::Internal, "handler failed")
            .with_retry_behavior(HandlerErrorRetryBehavior::NonRetryable)
            .with_source(source);

        let cause = error.source().expect("application cause");
        assert!(cause.downcast_ref::<ApplicationError>().is_some());
        assert_eq!(
            cause.source().expect("root cause").to_string(),
            "disk unavailable"
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
