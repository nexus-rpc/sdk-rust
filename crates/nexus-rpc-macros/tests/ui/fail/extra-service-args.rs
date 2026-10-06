#![allow(unused_imports)]

extern crate self as nexus_rpc;

include!("../support/runtime.rs");

use nexus_rpc_macros::{nexus_service, service};

#[nexus_service(name = "Example", extra = "bad")]
trait LegacyService {}

#[service(name = "Example", extra = "bad")]
trait ExampleService {}

fn main() {}
