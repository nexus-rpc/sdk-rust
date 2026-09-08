use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Boxed error type accepted by Nexus handlers.
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// Error type for handler-level failures.
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
    /// Returns the default retry behavior for this handler error type.
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
}

/// Retry behavior for a handler error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandlerErrorRetryBehavior {
    Unspecified,
    Retryable,
    NonRetryable,
}

/// Explicit Nexus handler failure.
#[derive(Debug)]
pub struct HandlerError {
    error_type: HandlerErrorType,
    message: String,
    retry_behavior: HandlerErrorRetryBehavior,
    source: Option<BoxError>,
}

impl HandlerError {
    /// Creates a new handler error.
    pub fn new(error_type: HandlerErrorType, message: impl Into<String>) -> Self {
        Self {
            error_type,
            message: message.into(),
            retry_behavior: HandlerErrorRetryBehavior::Unspecified,
            source: None,
        }
    }

    /// Overrides the default retry behavior for this handler error.
    pub fn with_retry_behavior(mut self, retry_behavior: HandlerErrorRetryBehavior) -> Self {
        self.retry_behavior = retry_behavior;
        self
    }

    /// Returns the Nexus handler error type.
    pub const fn error_type(&self) -> HandlerErrorType {
        self.error_type
    }

    /// Returns the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the explicit retry-behavior override, if any.
    pub const fn retry_behavior(&self) -> HandlerErrorRetryBehavior {
        self.retry_behavior
    }

    /// Returns the effective retry behavior after applying Nexus defaults.
    pub const fn effective_retry_behavior(&self) -> HandlerErrorRetryBehavior {
        match self.retry_behavior {
            HandlerErrorRetryBehavior::Unspecified => self.error_type.default_retry_behavior(),
            explicit => explicit,
        }
    }

    /// Attaches the underlying error that caused this handler failure.
    pub fn with_source(mut self, source: impl Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

impl Error for HandlerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as _)
    }
}

impl Display for HandlerErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let value = match self {
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
        };
        f.write_str(value)
    }
}

/// Explicit Nexus operation failure state.
#[derive(Debug)]
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

    /// Returns the operation error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the terminal state represented by this error.
    pub const fn state(&self) -> OperationErrorState {
        self.state
    }

    /// Attaches the underlying error that caused this operation failure.
    pub fn with_source(mut self, source: impl Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl Display for OperationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.state, self.message)
    }
}

impl Error for OperationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as _)
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
    use std::error::Error;

    use super::{HandlerError, HandlerErrorType, OperationError};

    #[test]
    fn handler_error_source_is_preserved() {
        let source = std::io::Error::other("boom");
        let error = HandlerError::new(HandlerErrorType::Internal, "handler").with_source(source);

        assert_eq!(error.source().expect("missing source").to_string(), "boom");
    }

    #[test]
    fn operation_error_source_is_preserved() {
        let source = std::io::Error::other("boom");
        let error = OperationError::failed("failed").with_source(source);

        assert_eq!(error.source().expect("missing source").to_string(), "boom");
    }
}
