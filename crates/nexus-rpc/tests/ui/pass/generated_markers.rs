use nexus_rpc::{NexusOperation, NexusService, nexus_service};

#[nexus_service(name = "wire-service")]
trait TestService {
    #[nexus_operation(name = "wire-operation")]
    fn operation(input: String) -> usize;
}

fn assert_service<T: NexusService>(_service: T) {}

fn assert_operation<T>(_operation: T)
where
    T: NexusOperation<Service = TestService, Input = String, Output = usize>,
{
}

fn main() {
    assert_service(TestService);
    assert_operation(TestService::operation);
    let _service = TestService;
    let _operation = TestService::operation;
}
