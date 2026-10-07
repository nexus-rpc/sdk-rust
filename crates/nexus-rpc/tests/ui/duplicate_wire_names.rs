use nexus_rpc::service;

#[service]
trait Catalog {
    fn lookup(input: ()) -> ();
    #[nexus_rpc::operation(name = "lookup")]
    fn search(input: ()) -> ();
}

fn main() {}
