#[nexus_rpc::service]
trait ConditionalService {
    #[cfg(any())]
    #[nexus_rpc::operation]
    fn disabled(input: MissingType) -> MissingType;
}

#[nexus_rpc::service]
trait ConditionalAttributeService {
    #[cfg_attr(all(), cfg(any()))]
    #[nexus_rpc::operation]
    fn disabled(input: MissingType) -> MissingType;
}

fn main() {}
