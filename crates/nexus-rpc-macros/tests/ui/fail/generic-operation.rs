#![allow(unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{nexus_operation, nexus_service};

#[nexus_service]
trait BadService {
    #[nexus_operation]
    fn ping<T>(input: T) -> ();
}

fn main() {}
