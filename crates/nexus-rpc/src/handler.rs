use std::error::Error;

use crate::{
    HandlerError, OperationDefinition, OperationError, ServiceDefinition,
    error::OperationErrorState, operation::OperationHandler,
};

/// Authoring contract implemented by generated or manually authored services.
pub trait ServiceHandler: Send + Sync + 'static {
    /// Returns the immutable service definition for the handler type.
    fn definition() -> &'static ServiceDefinition;

    /// Registers the service's operations with a downstream registrar.
    fn register<R>(self, registrar: &mut R)
    where
        Self: Sized,
        R: OperationRegistrar;
}

/// Typed downstream registration seam.
pub trait OperationRegistrar {
    /// Registers one typed operation definition and a `'static` handler wrapper.
    fn register<I, O, H>(&mut self, definition: &'static OperationDefinition<I, O>, handler: H)
    where
        I: Send + 'static,
        O: Send + 'static,
        H: OperationHandler<I, O> + 'static;
}

/// Ordered classification of an arbitrary handler error.
#[derive(Debug)]
pub enum ClassifiedHandlerError<'a> {
    OperationFailed(&'a OperationError),
    OperationCanceled(&'a OperationError),
    Handler(&'a HandlerError),
    Internal(&'a (dyn Error + Send + Sync + 'static)),
}

/// Classifies an arbitrary error according to Nexus precedence rules.
pub fn classify_handler_error<'a>(
    error: &'a (dyn Error + Send + Sync + 'static),
) -> ClassifiedHandlerError<'a> {
    if let Some(operation_error) = find_in_chain::<OperationError>(error) {
        return match operation_error.state() {
            OperationErrorState::Failed => ClassifiedHandlerError::OperationFailed(operation_error),
            OperationErrorState::Canceled => {
                ClassifiedHandlerError::OperationCanceled(operation_error)
            }
        };
    }

    if let Some(handler_error) = find_in_chain::<HandlerError>(error) {
        return ClassifiedHandlerError::Handler(handler_error);
    }

    ClassifiedHandlerError::Internal(error)
}

fn find_in_chain<'a, T>(error: &'a (dyn Error + 'static)) -> Option<&'a T>
where
    T: Error + 'static,
{
    let mut current = Some(error);
    while let Some(candidate) = current {
        if let Some(typed) = candidate.downcast_ref::<T>() {
            return Some(typed);
        }
        current = candidate.source();
    }
    None
}
