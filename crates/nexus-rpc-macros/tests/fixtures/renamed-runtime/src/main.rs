use nr::{NexusOperation, NexusService};

#[nr::nexus_service(name = "renamed-service")]
trait RenamedService {
    #[nr::nexus_operation(name = "renamed-operation")]
    fn operation(input: ()) -> ();
}

fn assert_operation<Op: NexusOperation<Service = RenamedService, Input = (), Output = ()>>(
    _operation: Op,
) {
}

fn main() {
    assert_operation(RenamedService::operation);
    assert_eq!(RenamedService::name(), "renamed-service");
    assert_eq!(RenamedService::operation.name(), "renamed-operation");
}
