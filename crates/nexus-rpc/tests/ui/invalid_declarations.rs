use nexus_rpc::service;

#[service]
trait InvalidOperations {
    async fn asynchronous(input: ()) -> ();
    fn body(input: ()) -> () {}
    fn receiver(self) -> ();
    fn generic<T>(input: T) -> ();
}

fn main() {}
