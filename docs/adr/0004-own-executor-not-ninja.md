# Makit executes its own graph instead of generating Ninja files

Generating `build.ninja` (as CMake, Meson and GN do) would be less code, but Ninja tracks freshness by mtime and command line only. It cannot express Kbuild's per-Symbol dependencies, content hashing or sandboxed actions, and it adds a second tool users must install. Makit schedules and runs actions itself. It exports `compile_commands.json` for C/C++ tooling; exporting `build.ninja` is not planned.
