#![allow(unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::nexus_service;

#[nexus_service]
trait BadService {
    const VERSION: usize;
}

fn main() {}
