#![allow(unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{nexus_operation, nexus_service, nexus_service_handler};

#[nexus_service]
trait ExampleService {
    #[nexus_operation]
    fn ping(input: ()) -> ();
}

struct ExampleHandlers;

#[nexus_service_handler(service = ExampleService)]
impl ExampleHandlers {}

fn main() {}
