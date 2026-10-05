# A stale Configuration fails the build instead of being silently synced

Kbuild silently runs `syncconfig` when Kconfig files change, which can alter the Configuration without anyone noticing. Makit stops with an error when `.config` no longer matches the Kconfig sources and tells the user to run `olddefconfig` (or pass `--auto-sync`). A Configuration that changes behind the user's back is incompatible with the reproducibility contract (ADR-0006).
