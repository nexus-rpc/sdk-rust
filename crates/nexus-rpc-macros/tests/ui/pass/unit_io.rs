extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::nexus_service;

#[nexus_service]
trait UnitService {
    #[nexus_operation]
    fn nop(input: ()) -> ();
}

fn expects_unit<Op: NexusOperation<Service = UnitService, Input = (), Output = ()>>(
    _operation: Op,
) {
}

fn main() {
    expects_unit(UnitService::nop);
}
