# Makit is a Rust program with a single build graph, not a Makefile framework

Kbuild runs as recursive GNU Make: one `make` sub-process per directory, a full re-parse of the Makefiles on every build, and dependency information split across processes. That makes no-op and incremental builds slow and prevents global scheduling. Makit keeps Kbuild's *semantics* (Symbol-controlled Goal lists, rebuild on command or Symbol change, a separate Output tree) but is implemented as a standalone Rust binary that loads every directory's build file into one in-memory graph and schedules it itself. GNU Make is neither required nor used.

## Considered Options

- **Trimmed Kbuild on GNU Make**: familiar and fast to start, but inherits the recursion and parse cost we are trying to escape, and runs poorly on non-Linux hosts.
