use nexus_rpc::{NexusOperation, NexusService, service};

#[service(name = "typed-service")]
trait TypedService {
    #[nexus_rpc::operation(name = "string-length")]
    fn length(input: String) -> usize;

    #[nexus_rpc::operation]
    fn notify(input: String);
}

fn assert_service<T: NexusService>(_service: T) {}

fn assert_operation<T>(_operation: T)
where
    T: NexusOperation<Service = TypedService, Input = String, Output = usize>,
{
}

#[test]
fn service_and_operation_markers_expose_typed_definitions() {
    assert_service(TypedService);
    assert_operation(TypedService::length);

    assert_eq!(TypedService::name(), "typed-service");
    assert_eq!(TypedService::length.name(), "string-length");
}

#[test]
fn omitted_operation_return_is_unit() {
    fn assert_unit_output<T>(_: T)
    where
        T: NexusOperation<Service = TypedService, Input = String, Output = ()>,
    {
    }

    assert_unit_output(TypedService::notify);
    assert_eq!(TypedService::notify.name(), "notify");
}
