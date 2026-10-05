#[nexus_rpc::nexus_service]
#[cfg(any())]
trait DisabledService {
    fn operation(input: String) -> String;
}

#[nexus_rpc::nexus_service]
#[cfg_attr(all(), cfg(any()))]
trait AlsoDisabledService {
    fn operation(input: String) -> String;
}

fn main() {}
