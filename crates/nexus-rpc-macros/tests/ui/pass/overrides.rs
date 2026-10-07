extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::nexus_service;

#[nexus_service(name = "payments")]
pub trait PaymentService {
    #[nexus_operation(name = "charge-card")]
    fn charge(input: String) -> String;
}

fn main() {
    let definition = PaymentService::definition();
    assert_eq!(definition.name(), "payments");
    assert_eq!(definition.operations()[0].rust_name(), "charge");
    assert_eq!(definition.operations()[0].wire_name(), "charge-card");
    assert_eq!(PaymentService::charge.name(), "charge-card");
}
