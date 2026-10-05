---
status: superseded by ADR-0014
---

# Build files use a static, Kbuild-style language

Per-directory build files keep Kbuild's idiom (`obj-$(CONFIG_FOO) += foo.o bar/`, `ccflags-y += ...`) in a small language parsed by Makit. It has no `$(shell)`, no arbitrary Make functions and no evaluation at parse time, so the graph is fully static, loads fast and stays reproducible.

## Considered Options

- **Declarative TOML/KDL**: needs several tables for what Kbuild says in three lines.
- **Starlark-style scripting**: powerful, but tends to grow into a Bazel-sized system and makes the graph harder to reason about.
