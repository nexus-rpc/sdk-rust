extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::nexus_service;

#[nexus_service]
trait GreetingService {
    #[nexus_operation]
    fn greet(input: String) -> usize;
}

fn expects_greeting<Op: NexusOperation<Service = GreetingService, Input = String, Output = usize>>(
    _operation: Op,
) {
}

fn main() {
    expects_greeting(GreetingService::greet);
    assert_eq!(GreetingService::name(), "GreetingService");
    assert_eq!(GreetingService::greet.name(), "greet");
    let definition = GreetingService::definition();
    assert_eq!(definition.name(), "GreetingService");
    assert_eq!(definition.operations()[0].wire_name(), "greet");
}
