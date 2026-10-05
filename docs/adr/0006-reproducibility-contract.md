# Reproducibility contract: bit-for-bit given the same toolchain, not hermetic

Makit guarantees bit-identical outputs for the same Source tree, Configuration and toolchain, regardless of Output tree path, time, user or locale. It enforces this itself: it sets `SOURCE_DATE_EPOCH`, remaps paths (`-ffile-prefix-map`, `--remap-path-prefix`), orders inputs stably, writes deterministic archives and scrubs the environment. A `makit repro-check` command builds twice in differently-named Output trees and compares hashes. Makit does **not** download or pin toolchains: the toolchain is whatever the Configuration points at, and its identity goes into every Action key. Users who need hermetic toolchains pin them externally (e.g. Nix).

## Considered Options

- **Hermetic toolchains managed by Makit (Bazel/Nix-style)**: strongest guarantee, but a large subsystem (fetching, per-platform toolchain packaging) that is out of proportion for v1.
