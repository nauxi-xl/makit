# Network access happens only in `makit fetch`, verified by hash

Builds run without network (ADR-0008). Every remote input goes through a separate `makit fetch` phase into a store outside the Source tree: Cargo crates (`cargo fetch --locked`, so `Cargo.lock` is mandatory, and builds use `--offline --locked`), tarballs and git sources. Non-Cargo sources must declare a BLAKE3 hash, and fetch fails on a mismatch.

## Considered Options

- **Let Cargo Actions use the network**: simpler, but breaks reproducibility and the sandbox.
- **Require `cargo vendor` into the Source tree**: works offline, but bloats repositories and handles only Rust.
