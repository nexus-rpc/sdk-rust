use nexus_rpc::{NexusOperation, service};

#[service]
pub(self) trait LocalService {
    #[nexus_rpc::operation]
    fn local(input: ());
}

mod nested {
    #[nexus_rpc::service]
    pub(super) trait ParentService {
        #[nexus_rpc::operation]
        fn parent(input: ());
    }
}

fn main() {
    assert_eq!(LocalService::local.name(), "local");
    assert_eq!(nested::ParentService::parent.name(), "parent");
}
