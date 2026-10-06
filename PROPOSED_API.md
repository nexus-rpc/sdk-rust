# Proposed Nexus Rust API

Working draft based on the current `nexus-rpc/sdk-rust` and `temporalio/sdk-core`
changes. The Temporal APIs below are currently behind the `experimental` feature.

## 1. Define a service once (`nexus-rpc/sdk-rust`)

```rust
use nexus_rpc::{NexusOperation, NexusService, service};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ChargeInput {
    pub customer_id: String,
    pub cents: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ChargeOutput {
    pub receipt_id: String,
}

#[service(name = "payments")]
pub trait PaymentService {
    #[nexus_rpc::operation(name = "charge-card")]
    fn charge(input: ChargeInput) -> ChargeOutput;
}

fn accepts_only_charge<O>(operation: O)
where
    O: NexusOperation<
        Service = PaymentService,
        Input = ChargeInput,
        Output = ChargeOutput,
    >,
{}

fn generated_markers_are_typed() {
    // The macro generates typed zero-sized markers, not a client trait implementation.
    assert_eq!(PaymentService::name(), "payments");
    assert_eq!(PaymentService::charge.name(), "charge-card");
    accepts_only_charge(PaymentService::charge);
}
```

## 2. Implement a portable handler (`nexus-rpc/sdk-rust`)

```rust
use nexus_rpc::{
    BoxError, OperationCancelContext, OperationHandler, OperationStartContext,
    OperationStartResult, OperationToken, nexus_operation_handler,
    nexus_service_handler,
};

struct ChargeHandler;

impl OperationHandler<ChargeInput, ChargeOutput> for ChargeHandler {
    async fn start(
        &self,
        context: OperationStartContext,
        input: ChargeInput,
    ) -> Result<OperationStartResult<ChargeOutput>, BoxError> {
        context.add_outbound_link(receipt_link(&input));
        Ok(OperationStartResult::Sync(charge(input).await?))

        // For completion through a callback instead:
        // Ok(OperationStartResult::Async(OperationToken::new(operation_id)))
    }

    async fn cancel(&self, context: OperationCancelContext) -> Result<(), BoxError> {
        cancel_charge(context.operation_token()).await?;
        Ok(())
    }
}

struct PaymentHandlers {
    charge: ChargeHandler,
}

#[nexus_service_handler(service = PaymentService)]
impl PaymentHandlers {
    #[nexus_operation_handler]
    fn charge(&self) -> impl OperationHandler<ChargeInput, ChargeOutput> + '_ {
        &self.charge
    }
}
```

## 3. Implement a Temporal workflow-backed handler (both repos)

```rust
use temporalio_client::WorkflowStartOptions;
use temporalio_sdk::nexus::{
    TemporalNexusClient, TemporalOperationError, TemporalOperationResult,
    TemporalOperationStartContext, nexus_service_handler, temporal_operation,
};

struct PaymentHandlers;

#[nexus_service_handler(service = PaymentService)]
impl PaymentHandlers {
    #[temporal_operation]
    async fn charge(
        &self,
        context: TemporalOperationStartContext,
        client: TemporalNexusClient,
        input: ChargeInput,
    ) -> Result<TemporalOperationResult<ChargeOutput>, TemporalOperationError> {
        client
            .start_workflow(
                ChargeWorkflow::run,
                input,
                WorkflowStartOptions::new(
                    "",
                    format!("charge-{}", context.request_id()),
                )
                .build(),
            )
            .await
    }
}

let worker_options = WorkerOptions::new("payments-task-queue")
    .register_workflow::<ChargeWorkflow>()?
    .register_nexus_service(PaymentHandlers)
    .build()?;
```

This `nexus_service_handler` is the Temporal SDK's facade: it expands
`#[temporal_operation]` methods and delegates the resulting portable handlers to
`nexus_rpc::nexus_service_handler`. The Nexus SDK does not reference Temporal.
`start_workflow` wires request deduplication, callbacks, links, and cancellation of the
backing workflow.

## 4. Call the same definition (`temporalio/sdk-core`)

From a workflow:

```rust
use temporalio_sdk::nexus::{
    NexusWorkflowClientOptions, NexusWorkflowOperationOptions,
};

let payments = ctx.nexus_client(
    PaymentService,
    NexusWorkflowClientOptions::new("payments-endpoint").build(),
);

let receipt: ChargeOutput = payments
    .execute(
        PaymentService::charge,
        input,
        NexusWorkflowOperationOptions::default(),
    )
    .await?;

let handle = payments
    .start(
        PaymentService::charge,
        another_input,
        NexusWorkflowOperationOptions::default(),
    )
    .await?;
handle.cancel();
let receipt = handle.result().await?;
```

From outside a workflow (a standalone Nexus operation):

```rust
use temporalio_client::NexusOperationStartOptions;

let payments = client.nexus_client(PaymentService, "payments-endpoint");
let handle = payments
    .start(
        PaymentService::charge,
        input,
        NexusOperationStartOptions::new("charge-123").build(),
    )
    .await?;

assert_eq!(handle.operation_id(), "charge-123");
let receipt: ChargeOutput = handle.result().await?;

// The handle also exposes operation_token, describe, cancel, and terminate.
```

An escape hatch remains available for dynamic service and operation names:

```rust
use temporalio_client::{NexusOperationStartOptions, UntypedNexusService};

let payments = client.nexus_client(
    UntypedNexusService::new("payments"),
    "payments-endpoint",
);
let receipt: ChargeOutput = payments
    .execute_operation(
        "charge-card",
        input,
        NexusOperationStartOptions::new("charge-123").build(),
    )
    .await?;
```

## Repository ownership

| `nexus-rpc/sdk-rust` | `temporalio/sdk-core` |
| --- | --- |
| `#[service]`, `#[operation]` | `WorkflowContext::nexus_client` |
| `PaymentService`, `PaymentService::charge` markers | `Client::nexus_client` and operation handles |
| `NexusService`, `NexusOperation` | `WorkerOptions::register_nexus_service` |
| `OperationHandler`, generic `#[nexus_service_handler]` | Temporal-aware `#[nexus_service_handler]` facade and `#[temporal_operation]` |
| start/cancel contexts, links, headers, tokens | payload conversion, codecs, and Temporal failures |
| portable handler errors and start results | polling, dispatch, callbacks, workflow start/cancel |
| typed visitor SPI used by integrations | Temporal's visitor/registry implementation |

The intended dependency direction is one-way:

```text
application definitions ──> nexus-rpc
Temporal application   ──> temporalio-sdk ──> nexus-rpc
nexus-rpc              -X-> temporalio-sdk / Temporal transport types
```
