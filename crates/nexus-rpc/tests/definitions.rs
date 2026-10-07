use nexus_rpc::{NexusOperation, NexusService, NexusServiceMetadata, nexus_service, service};

#[service]
trait Catalog {
    fn lookup(input: String) -> u64;
    fn ping(input: ()) -> ();
}

#[nexus_service(name = "billing")]
trait Payments {
    #[nexus_rpc::operation(name = "charge-card")]
    fn charge(input: u64) -> bool;
    #[nexus_rpc::nexus_operation(name = "refund-card")]
    fn refund(input: u64) -> ();
}

fn typed_operation<S, I, O>(operation: impl NexusOperation<Service = S, Input = I, Output = O>)
where
    S: NexusService,
    I: Send + 'static,
    O: Send + 'static,
{
    assert!(!operation.name().is_empty());
}

#[test]
fn default_names_and_types_match_service_metadata() {
    typed_operation::<Catalog, String, u64>(Catalog::lookup);
    typed_operation::<Catalog, (), ()>(Catalog::ping);

    let definition = Catalog::definition();
    assert_eq!(Catalog::name(), "Catalog");
    assert_eq!(definition.name(), Catalog::name());
    let operations = definition.operations();
    assert_eq!(operations.len(), 2);
    assert_eq!(
        (
            operations[0].rust_name(),
            operations[0].wire_name(),
            operations[0].input_type_name(),
            operations[0].output_type_name(),
        ),
        ("lookup", Catalog::lookup.name(), "String", "u64"),
    );
    assert_eq!(
        (
            operations[1].rust_name(),
            operations[1].wire_name(),
            operations[1].input_type_name(),
            operations[1].output_type_name(),
        ),
        ("ping", Catalog::ping.name(), "()", "()"),
    );
    assert_eq!(Catalog::lookup.name(), "lookup");
    assert_eq!(Catalog::ping.name(), "ping");
}

#[test]
fn overridden_names_and_aliases_match_service_metadata() {
    typed_operation::<Payments, u64, bool>(Payments::charge);
    typed_operation::<Payments, u64, ()>(Payments::refund);

    let definition = Payments::definition();
    assert_eq!(Payments::name(), "billing");
    assert_eq!(definition.name(), Payments::name());
    let operations = definition.operations();
    assert_eq!(operations.len(), 2);
    assert_eq!(
        operations
            .iter()
            .map(|info| (
                info.rust_name(),
                info.wire_name(),
                info.input_type_name(),
                info.output_type_name(),
            ))
            .collect::<Vec<_>>(),
        vec![
            ("charge", Payments::charge.name(), "u64", "bool"),
            ("refund", Payments::refund.name(), "u64", "()"),
        ],
    );
    assert_eq!(Payments::charge.name(), "charge-card");
    assert_eq!(Payments::refund.name(), "refund-card");
}
