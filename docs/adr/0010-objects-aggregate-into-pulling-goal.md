# Loose `obj-y` aggregates into whichever Linked Goal pulls its directory

As in Kbuild, a directory's bare `obj-y` entries have no output of their own. They join every Linked Goal that pulls the directory in (`sensord-y += drivers/`). A directory may be pulled in by several Linked Goals. Each object is still compiled only once, because flags come from the directory (`ccflags-y`) and the file (`CFLAGS_foo.o`), never from the Goal that pulls it in. Readers coming from CMake/Bazel may expect every object to be listed explicitly on its target. We deliberately keep Kbuild's implicit aggregation because it lets Symbols switch whole subtrees in and out with one line.

## Considered Options

- **Explicit-only linking**: every Linked Goal lists its objects and libraries. Clearer to newcomers, but loses the core Kbuild idiom.
