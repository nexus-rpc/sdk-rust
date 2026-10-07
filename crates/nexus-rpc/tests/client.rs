use nexus_rpc::{NexusOperation, NexusService, NexusServiceMetadata, service};

#[service(name = "typed-service")]
trait TypedService {
    #[nexus_rpc::operation(name = "string-length")]
    fn length(input: String) -> usize;

    fn definition(input: ()) -> ();
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

    let definition = <TypedService as NexusServiceMetadata>::definition();
    assert_eq!(definition.name(), TypedService::name());
    let length = &definition.operations()[0];
    assert_eq!(length.rust_name(), "length");
    assert_eq!(length.wire_name(), TypedService::length.name());
    assert_eq!(length.input_type_name(), "String");
    assert_eq!(length.output_type_name(), "usize");
    assert_eq!(
        definition.operations()[1].wire_name(),
        TypedService::definition.name()
    );
}
