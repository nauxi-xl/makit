# Toolchains are declared explicitly in `.makit/config.toml`; Symbols only select

Kbuild picks the toolchain from the environment and command line (`CROSS_COMPILE=`, `CC=`, `LLVM=1`), and most build systems auto-detect a host compiler. Both are invisible inputs. In Makit, every toolchain is declared by name in the committed Project manifest (`.makit/config.toml`): its role (host or target) and its commands (cc, cxx, ld, ar, rustc, cargo). Target Toolchains must be declared, and a missing declaration is an error. The host Toolchain may be left undeclared, in which case Makit detects it and records its Toolchain identity; a declared host Toolchain can be overridden from the command line. This relaxation keeps simple projects (and projects with no Host Goals) free of boilerplate, at the cost of host builds depending on the machine. The Configuration only chooses among the declared toolchains (e.g. a Kconfig `choice` between `TOOLCHAIN_ARM_GCC` and `TOOLCHAIN_ARM_CLANG`) and never holds paths. A command-line override is allowed, but Makit flags the build as diverging from its Defconfig.

## Considered Options

- **Toolchain paths as Symbols in the Configuration**: self-contained per Output tree, but mixes per-project facts with per-build choices and lets two Defconfigs silently disagree about what "gcc" means.
- **`makit.properties` (Java key=value)**: the original suggestion; rejected for TOML because toolchain declarations are nested and will grow lists (flags, sysroots).

## Consequences

- An undeclared host Toolchain is resolved in this order: command-line override, `.makit/config.toml`, `HOSTCC`/`HOSTCXX` environment, then `cc`/`c++`/`rustc`/`cargo` on `PATH`. The resolved paths and Toolchain identity are recorded in the Output tree, and a later build that resolves a different host stops with an error instead of silently rebuilding everything (same stance as ADR-0011).
- Makit synthesizes a Kconfig `choice TOOLCHAIN` with one `config TOOLCHAIN_<NAME>` per declared target Toolchain, so Toolchains are picked in `menuconfig`/Defconfigs like any Symbol and project Kconfig can `depends on` them. A developer who wants another compiler declares a new Toolchain entry and selects it in their Configuration; there is no separate per-developer override file.
- Synthesized names upper-case the Toolchain name and map `-` to `_` (`arm-gcc` → `TOOLCHAIN_ARM_GCC`). They sit in a "Toolchain" menu at the top of the Kconfig tree. The default is the Toolchain marked `default = true`. Host Toolchains are not part of the choice. The `TOOLCHAIN_` prefix is reserved, and a project Symbol using it is an error. Adding or removing a Toolchain changes the Kconfig, so existing Configurations become stale (ADR-0011).
