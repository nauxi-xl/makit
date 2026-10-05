# Kconfig is implemented natively in Rust, with Linux `conf` as test oracle

Makit ships as a single Rust binary (ADR-0002), so embedding `kconfiglib` would force a Python runtime on every host. We implement the full Kconfig language (parser, evaluator for `depends on` / `select` / `imply` / `choice` / tristate, and the `defconfig` / `olddefconfig` frontends) in Rust, excluding kernel-only extensions such as `modules`. Conformance is checked by running Linux's `conf` binary on the same Kconfig inputs and comparing the resulting `.config`; we execute it, never copy its GPL code. A `menuconfig` TUI comes after v1.

## Considered Options

- **kconfiglib via PyO3 or subprocess**: fastest to ship, but needs Python on the host and depends on an effectively unmaintained upstream.
- **Existing Rust Kconfig parser crate + own evaluator**: maturity unverified; the parser is the smaller part of the work anyway.
