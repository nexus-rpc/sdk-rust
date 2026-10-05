use nexus_rpc::nexus_service;

#[nexus_service]
trait TestService {
    fn operation(input: String) -> usize;
}

fn main() {
    let _ = TestService::missing;
}
