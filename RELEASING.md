# Releasing

The workspace publishes two crates, `nexus-rpc-macros` and `nexus-rpc`, initially
at version `0.1.0`. Prepare both at the same version and keep the macro dependency
version in `crates/nexus-rpc/Cargo.toml` in sync.

Publishing is permanent: an uploaded version cannot be overwritten or reused.
Obtain maintainer approval for the specific release before uploading.

## Prepare and validate

Use the repository's pinned Rust toolchain (currently 1.94) for packaging and
publishing. The minimum supported Rust version, 1.88, is tested separately.

1. Review the public API and both crate versions. Update `CHANGELOG.md` with a
   dated section for the release, including the initial API for `0.1.0`.
2. Run all [verification commands](README.md#verification-commands).
3. Inspect the archives:

   ```sh
   cargo package -p nexus-rpc-macros --list
   cargo package -p nexus-rpc --list
   ```

   Both must include `README.md` and `LICENSE`. The combined publishing dry run
   builds the extracted archives, staging the unpublished macro dependency in a
   temporary registry. Separate dry runs cannot validate the main crate before
   its macro dependency exists on crates.io.
4. Review and commit the release changes, and ensure CI passes for the exact
   revision to be published. Publish from a clean checkout of that revision.

## Bootstrap crates.io

Trusted Publishing currently requires each crate to have been published once.
The first publication of both crates therefore uses an API token.

1. Sign in to [crates.io](https://crates.io), verify the account's email address,
   and confirm that both crate names are available.
2. Create a short-lived API token with permission to publish new crates.
3. Authenticate locally:

   ```sh
   cargo login
   ```

   Paste the token only at Cargo's prompt. Never put it in chat, a command line,
   or a repository file.
4. After release approval, run:

   ```sh
   cargo publish -p nexus-rpc-macros -p nexus-rpc
   ```

   Cargo publishes in dependency order. Publication is not atomic: if it stops
   after uploading one crate, check crates.io before retrying. Do not republish
   an existing version. If only the macro crate was uploaded, validate and publish
   the remaining crate:

   ```sh
   cargo publish -p nexus-rpc --dry-run
   cargo publish -p nexus-rpc
   ```

5. Add the intended maintainers or GitHub team as owners of **both** crates.
6. Confirm that both versions are available on crates.io and that their docs.rs
   documentation builds. Create a `v0.1.0` tag at the published commit and a
   draft GitHub release with the reviewed release notes. For later releases,
   substitute the actual version. Tag pushes and GitHub release creation are
   separate publishing actions requiring maintainer approval.
7. Revoke the bootstrap token on crates.io, then run `cargo logout` to remove
   its locally stored credential.

## Subsequent release automation

The release workflow is not yet implemented. Follow the
[Temporal Rust SDK release model](https://github.com/temporalio/sdk-rust/blob/main/.github/workflows/create-release.yml):

- A manually dispatched `create-release.yml` workflow, restricted to `main`.
- A GitHub `release` environment with appropriate reviewer and branch rules.
- Serialized publishing and a bounded job timeout.
- OIDC authentication through `rust-lang/crates-io-auth-action`, with
  `id-token: write`, instead of a long-lived repository secret.
- A publish plan that skips versions already on crates.io, allowing recovery
  from a partially completed release.
- A draft GitHub release targeting the published commit, with support for the
  first release having no previous version tag.

After the workflow exists, configure Trusted Publishing in the settings of
**each** crate with repository owner `nexus-rpc`, repository `sdk-rust`, workflow
filename `create-release.yml`, and environment `release`.

See the [crates.io Trusted Publishing documentation](https://crates.io/docs/trusted-publishing).
