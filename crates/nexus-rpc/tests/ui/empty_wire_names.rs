use nexus_rpc::service;

#[service(name = "")]
trait EmptyService {
    fn lookup(input: ()) -> ();
}

#[service]
trait EmptyOperation {
    #[nexus_rpc::operation(name = "")]
    fn lookup(input: ()) -> ();
}

fn main() {}
