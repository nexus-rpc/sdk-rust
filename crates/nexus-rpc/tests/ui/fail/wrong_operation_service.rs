use nexus_rpc::{NexusOperation, service};

#[service]
trait First {
    fn echo(input: String) -> String;
}

#[service]
trait Second {
    fn echo(input: String) -> String;
}

fn expects_first<Op: NexusOperation<Service = First>>(_operation: Op) {}

fn main() {
    expects_first(Second::echo);
}
