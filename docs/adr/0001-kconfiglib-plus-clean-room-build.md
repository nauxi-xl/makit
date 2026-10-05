---
status: partly superseded — the Kconfig half by ADR-0003; the clean-room build layer still stands
---

# Kconfig via kconfiglib; build layer written clean-room, not forked from Linux

Linux Kbuild and `scripts/kconfig` are GPL-2.0-only and deeply tied to the kernel (vmlinux, modpost, `KBUILD_*`), which is exactly what earlier reuses (U-Boot, BusyBox, Zephyr) dragged along. We implement the Kconfig language with `kconfiglib` (ISC license), since it is the hardest part to get right and already exists under a permissive license, and write the goal-list / build layer from scratch, keeping Kbuild's semantics and familiar syntax but none of its code. This keeps Makit under a permissive license for public distribution.

## Considered Options

- **Fork Kbuild and trim it**: fastest and compatible out of the box, but the whole system becomes GPL-2.0-only and stays shaped by kernel assumptions.
- **Fully clean-room including Kconfig**: maximum freedom, but re-implementing Kconfig semantics (`select`, `imply`, `choice`, tristate math) correctly is a large, low-value effort.
