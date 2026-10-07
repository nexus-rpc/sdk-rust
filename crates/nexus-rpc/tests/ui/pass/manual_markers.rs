use nexus_rpc::{NexusOperation, NexusService};

struct Service;

impl NexusService for Service {
    fn name() -> &'static str {
        "manual-service"
    }
}

struct Operation;

impl NexusOperation for Operation {
    type Service = Service;
    type Input = String;
    type Output = usize;

    fn name(&self) -> &str {
        "manual-operation"
    }
}

fn main() {
    assert_eq!(Service::name(), "manual-service");
    assert_eq!(Operation.name(), "manual-operation");
}
