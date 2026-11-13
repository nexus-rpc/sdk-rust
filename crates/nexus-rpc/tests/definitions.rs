use nexus_rpc::{NexusOperation, NexusService, NexusServiceMetadata, service};

#[service]
trait Catalog {
    fn lookup(input: String) -> u64;
    fn ping(input: ()) -> ();
}

#[service(name = "billing")]
trait Payments {
    #[nexus_rpc::operation(name = "charge-card")]
    fn charge(input: u64) -> bool;
    #[nexus_rpc::operation(name = "refund-card")]
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
    assert_eq!(
        operations
            .iter()
            .map(|info| info.wire_name())
            .collect::<Vec<_>>(),
        vec![Catalog::lookup.name(), Catalog::ping.name()],
    );
    assert_eq!(Catalog::lookup.name(), "lookup");
    assert_eq!(Catalog::ping.name(), "ping");
}

#[test]
fn overridden_names_and_types_match_service_metadata() {
    typed_operation::<Payments, u64, bool>(Payments::charge);
    typed_operation::<Payments, u64, ()>(Payments::refund);

    let definition = Payments::definition();
    assert_eq!(Payments::name(), "billing");
    assert_eq!(definition.name(), Payments::name());
    let operations = definition.operations();
    assert_eq!(
        operations
            .iter()
            .map(|info| info.wire_name())
            .collect::<Vec<_>>(),
        vec![Payments::charge.name(), Payments::refund.name()],
    );
    assert_eq!(Payments::charge.name(), "charge-card");
    assert_eq!(Payments::refund.name(), "refund-card");
}
