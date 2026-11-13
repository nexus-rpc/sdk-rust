use nexus_rpc::{NexusOperation, service};

#[service]
trait Catalog {
    fn lookup(input: String) -> u64;
}

#[service]
trait Other {
    fn lookup(input: String) -> u64;
}

fn requires_catalog(_: impl NexusOperation<Service = Catalog>) {}
fn requires_bytes(_: impl NexusOperation<Input = Vec<u8>>) {}
fn requires_string(_: impl NexusOperation<Output = String>) {}

fn main() {
    requires_catalog(Other::lookup);
    requires_bytes(Catalog::lookup);
    requires_string(Catalog::lookup);
}
