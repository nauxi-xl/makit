# Tiered sandbox; built-in compile Rules may read the whole Source tree

C compilers only discover their headers after running (via depfiles), so declaring every input up front as Bazel does is impractical for C. On Linux, each Action runs in user/mount/network namespaces: the Source tree and toolchain are read-only, the only writable place is that Action's output directory, there is no network, and the environment is scrubbed. Project-defined Rules must declare all their inputs, and on Linux an undeclared read fails the Action. Built-in C/C++ compile Rules may read anything in the Source tree, because the depfile records exactly what they read. macOS and Windows run in a degraded mode (scrubbed environment, fixed cwd) and print a warning.

## Considered Options

- **Bazel-style input-only sandbox for every Action**: strictest, but forces users to declare every include directory and header.
- **No sandbox, environment scrubbing only**: leaves network access and stray reads as silent sources of non-reproducibility.
