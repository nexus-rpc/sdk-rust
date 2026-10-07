use nexus_rpc::service;

#[service]
trait Example {
    fn echo(input: String) -> String;
}

use __nexus_service_metadata_for_Example::{
    __NexusAllBindings, __NexusBind_echo, __NexusOperationVisitor, __NexusRegistered_echo,
    visit_operations,
};

fn main() {}
