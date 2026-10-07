# Changelog

## Unreleased

### Changed
* This SDK defines Nexus services, typed operation markers, and optional definition metadata only.
  Operation authoring and execution belong to the Temporal SDK.

### Removed
* Portable handler traits, handler/provider macros, registration visitors, and generated handler
  wrappers.
* Handler request contexts, cancellation primitives, operation results and tokens, and handler
  error authoring APIs.
