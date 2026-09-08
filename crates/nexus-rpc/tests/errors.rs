use std::{error::Error, fmt};

use nexus_rpc::{
    HandlerError, HandlerErrorRetryBehavior, HandlerErrorType, OperationError, OperationErrorState,
    integration::{ClassifiedHandlerError, classify_handler_error},
};

#[derive(Debug)]
struct WrapperError {
    message: &'static str,
    source: Box<dyn Error + Send + Sync + 'static>,
}

impl fmt::Display for WrapperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl Error for WrapperError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[derive(Debug)]
struct PlainError(&'static str);

impl fmt::Display for PlainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for PlainError {}

#[test]
fn handler_error_defaults_match_error_type() {
    let cases = [
        (
            HandlerErrorType::Unknown,
            HandlerErrorRetryBehavior::Retryable,
        ),
        (
            HandlerErrorType::BadRequest,
            HandlerErrorRetryBehavior::NonRetryable,
        ),
        (
            HandlerErrorType::Unauthenticated,
            HandlerErrorRetryBehavior::NonRetryable,
        ),
        (
            HandlerErrorType::Unauthorized,
            HandlerErrorRetryBehavior::NonRetryable,
        ),
        (
            HandlerErrorType::NotFound,
            HandlerErrorRetryBehavior::NonRetryable,
        ),
        (
            HandlerErrorType::RequestTimeout,
            HandlerErrorRetryBehavior::Retryable,
        ),
        (
            HandlerErrorType::Conflict,
            HandlerErrorRetryBehavior::NonRetryable,
        ),
        (
            HandlerErrorType::ResourceExhausted,
            HandlerErrorRetryBehavior::Retryable,
        ),
        (
            HandlerErrorType::Internal,
            HandlerErrorRetryBehavior::Retryable,
        ),
        (
            HandlerErrorType::NotImplemented,
            HandlerErrorRetryBehavior::NonRetryable,
        ),
        (
            HandlerErrorType::Unavailable,
            HandlerErrorRetryBehavior::Retryable,
        ),
        (
            HandlerErrorType::UpstreamTimeout,
            HandlerErrorRetryBehavior::Retryable,
        ),
    ];

    for (error_type, expected) in cases {
        let error = HandlerError::new(error_type, "boom");
        assert_eq!(
            error.retry_behavior(),
            HandlerErrorRetryBehavior::Unspecified
        );
        assert_eq!(error.effective_retry_behavior(), expected);
    }
}

#[test]
fn handler_error_retry_override_is_preserved() {
    let error = HandlerError::new(HandlerErrorType::BadRequest, "boom")
        .with_retry_behavior(HandlerErrorRetryBehavior::Retryable);

    assert_eq!(error.retry_behavior(), HandlerErrorRetryBehavior::Retryable);
    assert_eq!(
        error.effective_retry_behavior(),
        HandlerErrorRetryBehavior::Retryable
    );
}

#[test]
fn operation_error_states_are_preserved() {
    let failed = OperationError::failed("failed");
    let canceled = OperationError::canceled("canceled");

    assert_eq!(failed.state(), OperationErrorState::Failed);
    assert_eq!(canceled.state(), OperationErrorState::Canceled);

    match classify_handler_error(&failed) {
        ClassifiedHandlerError::OperationFailed(error) => {
            assert_eq!(error.message(), "failed");
        }
        other => panic!("unexpected classification: {other:?}"),
    }

    match classify_handler_error(&canceled) {
        ClassifiedHandlerError::OperationCanceled(error) => {
            assert_eq!(error.message(), "canceled");
        }
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn wrapped_operation_error_beats_wrapped_handler_error() {
    let error = WrapperError {
        message: "root",
        source: Box::new(WrapperError {
            message: "handler-layer",
            source: Box::new(WrapperError {
                message: "operation-layer",
                source: Box::new(WrapperError {
                    message: "deep",
                    source: Box::new(WrapperError {
                        message: "operation",
                        source: Box::new(OperationError::failed("terminal")),
                    }),
                }),
            }),
        }),
    };

    match classify_handler_error(&error) {
        ClassifiedHandlerError::OperationFailed(operation_error) => {
            assert_eq!(operation_error.message(), "terminal");
        }
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn wrapped_handler_error_is_preserved_without_operation_error() {
    let error = WrapperError {
        message: "outer",
        source: Box::new(WrapperError {
            message: "inner",
            source: Box::new(HandlerError::new(
                HandlerErrorType::ResourceExhausted,
                "limit",
            )),
        }),
    };

    match classify_handler_error(&error) {
        ClassifiedHandlerError::Handler(handler_error) => {
            assert_eq!(
                handler_error.error_type(),
                HandlerErrorType::ResourceExhausted
            );
            assert_eq!(handler_error.message(), "limit");
        }
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn arbitrary_errors_fall_back_to_internal() {
    let error = PlainError("plain");

    match classify_handler_error(&error) {
        ClassifiedHandlerError::Internal(source) => {
            assert_eq!(source.to_string(), "plain");
        }
        other => panic!("unexpected classification: {other:?}"),
    }
}
