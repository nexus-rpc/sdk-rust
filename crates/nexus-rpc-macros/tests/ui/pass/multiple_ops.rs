extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::service;

#[service]
trait UtilityService {
    #[operation]
    fn ping(input: ()) -> ();

    #[operation]
    fn status(input: String) -> bool;
}

fn expects_ping<Op: NexusOperation<Service = UtilityService, Input = (), Output = ()>>(
    _operation: Op,
) {
}

fn expects_status<Op: NexusOperation<Service = UtilityService, Input = String, Output = bool>>(
    _operation: Op,
) {
}

fn main() {
    expects_ping(UtilityService::ping);
    expects_status(UtilityService::status);
    let operations = UtilityService::definition().operations();
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].wire_name(), UtilityService::ping.name());
    assert_eq!(operations[1].wire_name(), UtilityService::status.name());
}
