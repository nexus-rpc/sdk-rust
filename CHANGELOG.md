# Changelog

## Unreleased

## [0.1.0] - 2026-11-16

Initial release of `nexus-rpc` and `nexus-rpc-macros`, supporting Rust 1.88 and newer.

### Added

- Add `service` and `operation` macros that generate zero-sized service and operation markers with compile-time declaration validation.
- Add typed `NexusService` and `NexusOperation` contracts that preserve the declaring service, input, and output types.
- Add explicit service and operation wire names and immutable metadata through `NexusServiceMetadata`, `ServiceDefinition`, and `OperationDefinitionInfo`.
- Add portable handler and operation errors with protocol retry policies and source chains.
- Preserve unknown handler error codes and support explicit stack traces and portable, serializable original failure payloads on both error kinds.
