use nexus_rpc::{NexusOperation, service};

#[service]
trait Example {
    fn length(input: String) -> usize;
}

fn expects_bytes<Op: NexusOperation<Input = Vec<u8>, Output = usize>>(_operation: Op) {}
fn expects_string<Op: NexusOperation<Input = String, Output = String>>(_operation: Op) {}

fn main() {
    expects_bytes(Example::length);
    expects_string(Example::length);
}
