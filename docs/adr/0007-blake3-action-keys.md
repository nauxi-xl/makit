# Freshness is decided by BLAKE3 Action keys with per-Symbol tracking

An Action key is the BLAKE3 hash of an Action's input contents, command line, allowed environment, toolchain identity and the values of the Symbols that Action actually uses. For C/C++, the used Symbols are found Kbuild-`fixdep`-style by scanning the source and every header in its depfile for `CONFIG_*` identifiers. For Rust, the granularity is the whole Cargo package. mtime/size/inode serve only as a shortcut to skip re-hashing unchanged files. Outputs whose content hash did not change stop propagation (early cutoff). Keys are shared through a local content-addressed cache used by every Output tree on the machine, and the key format is kept compatible with a later Bazel Remote Execution API cache.

## Considered Options

- **mtime + command line (Make/Ninja)**: cheap, but breaks across Output trees and caches and has no notion of Symbols.
- **Whole-`autoconf.h` hashing**: simpler, but any Symbol change rebuilds every C file.
- **Pure hashing without the mtime shortcut**: simpler invariants, but too slow for no-op builds on large trees.
