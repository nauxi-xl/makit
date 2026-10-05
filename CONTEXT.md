# Makit

A general-purpose, configuration-driven build system for C/C++ and Rust projects, carrying over the ideas of Linux Kbuild/Kconfig without the kernel-specific baggage. It also drives non-compile tasks (generated files, docs, packaging).

## Language

### Project

**Project manifest**:
The committed `.makit/config.toml` in the Source tree, holding facts that are the same for every build: project metadata and Toolchain declarations.
_Avoid_: makit.toml, makit.properties, project config, settings file

**Toolchain**:
A named set of tools (compilers, linker, archiver, rustc, cargo) with a role of either host or target. Target Toolchains must be declared; the host Toolchain may be detected when undeclared and can be overridden. Makit turns the declared target Toolchains into a Kconfig `choice`, so picking one is an ordinary Configuration change.
_Avoid_: compiler, SDK, CROSS_COMPILE

**Tool**:
A program declared under `[tools]` in the Project manifest, reachable from Rules only through the variable of the same name (e.g. `$(PANDOC)`).
_Avoid_: binary, command, utility

### Configuration

**Symbol**:
A named, typed option declared in Kconfig language (`config FOO`), whose value is decided by its dependencies, defaults and the user's choices. Symbols select among declared Toolchains but never define them.
_Avoid_: option, flag, feature flag

**Configuration**:
The complete set of resolved Symbol values for one build, persisted as `.config`.
_Avoid_: config file, settings, profile

**Defconfig**:
A minimal, committed file in the Source tree that lists only the Symbols differing from their defaults; it seeds a Configuration.
_Avoid_: preset, profile, board config

### Building

**Build file**:
The per-directory file, written in a pure, static subset of GNU Make, that declares that directory's Goal lists, flags and Rules.
_Avoid_: Makefile, Kbuild file, manifest

**Rule file**:
A shared `.mk` file inside the Source tree, included by Build files, that defines variables and Rules but no Goal lists.
_Avoid_: makefile fragment, include file

**Goal list**:
A per-directory list of Goals to produce, whose membership is controlled by Symbols (Kbuild's `obj-$(CONFIG_FOO) += ...`). Listing a directory pulls that directory's Build file into the graph.
_Avoid_: target list, sources

**Goal**:
Any output Makit produces: object, library, binary, generated file, document or package. Compiling is not special; it is just one kind of Rule.
_Avoid_: target, task, artifact

**Linked Goal**:
A Goal produced by linking (a program, static library or shared library). It absorbs the loose `obj-y` of every directory it pulls in.
_Avoid_: executable target, binary target

**Host Goal**:
A Goal built with the host toolchain so it can run during the build itself (e.g. a code generator), as opposed to Goals built for the target.
_Avoid_: tool, build tool, hostprog

**Rule**:
A named command template, written as a Make rule, that turns declared inputs into declared outputs. Makit ships built-in Rules for C/C++ and Rust; projects define their own for everything else.
_Avoid_: recipe, task, step

**Test Goal**:
A Goal whose output is the recorded outcome (pass/fail and log) of running a test, cached like any other Goal.
_Avoid_: test target, test run

**Command**:
A user-defined, side-effecting operation (flash, run, deploy) declared as `run-<name>` in `.makit/commands.mk` and invoked with `makit run <name>`. It may depend on Goals but is never cached, sandboxed or part of the build graph.
_Avoid_: phony target, task, script

**Module**:
A Goal selected with a Symbol set to `m`, built as a separately loadable unit (shared library or plugin) rather than linked into the main output.
_Avoid_: plugin, dylib, loadable

### Execution

**Action**:
One concrete run of a Rule on specific inputs, producing one or more Goals; the unit Makit schedules, sandboxes and caches.
_Avoid_: job, command, step

**Action key**:
The fingerprint that identifies an Action: its inputs' contents, command line, allowed environment, toolchain identity and the Symbols it uses. Equal keys mean equal outputs.
_Avoid_: cache key, hash, signature

**Toolchain identity**:
The fingerprint of the compilers and tools an Action invokes; part of every Action key.

**Fetch**:
The only phase allowed to use the network; it downloads remote inputs (crates, tarballs, git sources) and verifies them against declared hashes before any build runs.
_Avoid_: download, sync, vendor

**Cache**:
The machine-wide store of Action outputs, addressed by Action key and shared by every Output tree.
_Avoid_: build cache, artifact store

**User settings**:
Per-user Makit preferences in `~/.config/makit.toml`, limited to things that cannot change outputs; today only the Cache size limit.
_Avoid_: global config, user config

### Trees

**Source tree**:
The directory tree holding the project's inputs (sources, Kconfig files, Build files). Makit never writes into it.

**Output tree**:
A separate directory receiving every artifact of one build (Kbuild's `O=`); one Source tree can have many Output trees.
_Avoid_: build dir, obj dir

**Export**:
The part of an Output tree published for External projects: public headers, the Configuration, the Toolchain identity and linkable libraries.
_Avoid_: SDK, install, dist

**External project**:
A project without access to the Source tree that builds against another project's Export (Kbuild's `M=`).
_Avoid_: out-of-tree module, downstream, consumer
