# Build files are a pure, static subset of GNU Make parsed by Makit

Supersedes ADR-0005. Kbuild users expect to write ordinary Make rules, so Build files accept a subset of GNU Make that Makit parses itself; GNU Make is still never run (ADR-0002). Allowed: explicit, pattern and grouped (`&:`) rules, automatic variables, `:=` / `+=` / `?=`, conditionals, `include` and pure functions (`patsubst`, `subst`, `filter`, `addprefix`, `foreach`, `if`, sorted `wildcard`). Forbidden: `$(shell)`, `$(eval)`, recursive `$(MAKE)`, self-referencing recursive variables and `.PHONY`. This keeps one static graph. Each Make rule is a Rule. Its prerequisites are its declared inputs (enforced by the sandbox, ADR-0008) and its targets are its only permitted outputs. Recipes run through the `SHELL` declared in `.makit/config.toml`, with an empty `PATH`, so tools are reachable only through the variables named after their `.makit/config.toml` keys (`$(PANDOC)`, `$(CC)`, `$(HOSTCC)`). `include` is limited to `.mk` files inside the Source tree (or an Export), which may define variables and Rules but no Goal lists. Third-party Makefiles that need full GNU Make are built by a Rule that invokes a declared `$(MAKE)` tool inside the sandbox.

## Considered Options

- **Delegate Make rules to real GNU Make**: full compatibility, but brings back the split graph, with no Symbol tracking, sandbox or cache.
- **Full GNU Make reimplementation in Rust**: enormous, and `$(shell)` would still break reproducibility.

## Consequences

- A Rule is only *wired*, never *forced*: its targets must appear in some Goal list or as a prerequisite of another wired Rule, otherwise loading the graph fails (orphan Rule). Whether a wired Rule actually runs is still decided by the Configuration: `gen-$(CONFIG_DOCS) += manual.pdf` with `CONFIG_DOCS=n` builds nothing.
- Side-effecting work (flash, deploy, run) cannot be a Rule; it is a Command (ADR-0015).
